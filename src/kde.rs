/*
 * This uses a hack with KWin scripts in order to receive the active window.
 * For the moment of writing, KWin doesn't implement the appropriate protocols to get a top level window.
 * Inspired by https://github.com/k0kubun/xremap/
 */
use crate::idle::Status;
use crate::wayland_idle::IdleWatcherRunner;
use crate::{ActiveWindowData, WindowManager};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use std::env::{self, temp_dir};
use std::path::Path;
use std::sync::{Arc, mpsc::channel};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error};
use zbus::interface;
use zbus::{Connection, conn::Builder as ConnectionBuilder};

const KWIN_SCRIPT_NAME: &str = "whatawhat-lib";
const KWIN_SCRIPT: &str = include_str!("kde.js");

struct KWinScript {
    dbus_connection: Connection,
    is_loaded: bool,
}

impl KWinScript {
    fn new(dbus_connection: Connection) -> Self {
        KWinScript {
            dbus_connection,
            is_loaded: false,
        }
    }

    async fn load(&mut self) -> anyhow::Result<()> {
        let path = temp_dir().join("whatawhat-lib.js");
        std::fs::write(&path, KWIN_SCRIPT).unwrap();

        let number = self.get_registered_number(&path).await?;
        let result = self.start(number).await;
        std::fs::remove_file(&path)?;
        self.is_loaded = true;

        result
    }

    async fn is_loaded(&self) -> anyhow::Result<bool> {
        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "isScriptLoaded",
                &KWIN_SCRIPT_NAME,
            )
            .await?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    async fn get_registered_number(&self, path: &Path) -> anyhow::Result<i32> {
        let temp_path = path
            .to_str()
            .ok_or(anyhow!("Temporary file path is not valid"))?;

        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "loadScript",
                // since OsStr does not implement zvariant::Type, the temp-path must be valid utf-8
                &(temp_path, KWIN_SCRIPT_NAME),
            )
            .await?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    async fn unload(&self) -> anyhow::Result<bool> {
        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "unloadScript",
                &KWIN_SCRIPT_NAME,
            )
            .await?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    async fn start(&self, script_number: i32) -> anyhow::Result<()> {
        debug!("Starting KWin script {script_number}");

        let path = if self.get_major_version().await < 6 {
            format!("/{script_number}")
        } else {
            format!("/Scripting/Script{script_number}")
        };
        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                path,
                Some("org.kde.kwin.Script"),
                "run",
                &(),
            )
            .await
            .with_context(|| "Error on starting the script")?;
        Ok(())
    }

    async fn get_major_version(&self) -> i8 {
        if let Ok(version) = Self::get_major_version_from_env() {
            debug!("KWin version from KDE_SESSION_VERSION: {version}");

            version
        } else {
            self.get_major_version_from_dbus()
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to get KWin version: {e}");
                    5
                })
        }
    }

    fn get_major_version_from_env() -> anyhow::Result<i8> {
        env::var("KDE_SESSION_VERSION")?
            .parse::<i8>()
            .map_err(std::convert::Into::into)
    }

    async fn get_major_version_from_dbus(&self) -> anyhow::Result<i8> {
        let support_information: String = self
            .dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/KWin",
                Some("org.kde.KWin"),
                "supportInformation",
                &(),
            )
            .await?
            .body()
            .deserialize()?;

        // find a string like "KWin version: 5.27.8" and extract the version number from it:
        let version = support_information
            .lines()
            .find(|line| line.starts_with("KWin version: "))
            .ok_or(anyhow!("KWin version not found"))?
            .split_whitespace()
            .last()
            .ok_or(anyhow!("KWin version is invalid"))?;

        // Extract the major version number from the version number like "5.27.8":
        let major_version = version
            .split('.')
            .next()
            .ok_or(anyhow!("KWin version is invalid: {version}"))?
            .parse::<i8>()?;

        debug!("KWin version from DBus: {version}, major version: {major_version}");

        Ok(major_version)
    }
}

impl Drop for KWinScript {
    fn drop(&mut self) {
        // if self.is_loaded {
        //     tokio::runtime::Builder::new_current_thread()
        //         .enable_all()
        //         .build()
        //         .unwrap()
        //         .block_on(async {
        //             debug!("Unloading KWin script");
        //             if let Err(e) = self.unload().await {
        //                 error!("Problem during stopping KWin script: {e}");
        //             };
        //         });
        // }
    }
}

async fn send_active_window(
    active_window: &Arc<Mutex<ActiveWindow>>,
) -> anyhow::Result<ActiveWindowData> {
    let active_window = active_window.lock().await;

    Ok(ActiveWindowData {
        window_title: active_window.caption.clone().into(),
        app_identifier: active_window.resource_name.clone().into(),
    })
}

struct ActiveWindow {
    resource_class: String,
    resource_name: String,
    caption: String,
}

struct ActiveWindowInterface {
    active_window: Arc<Mutex<ActiveWindow>>,
}

#[interface(name = "com.github.anoromi.whatawhat_lib")]
impl ActiveWindowInterface {
    async fn notify_active_window(
        &mut self,
        caption: String,
        resource_class: String,
        resource_name: String,
        _pid: i32,
    ) {
        debug!(
            "Active window class: \"{resource_class}\", name: \"{resource_name}\", caption: \"{caption}\""
        );
        let mut active_window = self.active_window.lock().await;
        active_window.caption = caption;
        active_window.resource_class = resource_class;
        active_window.resource_name = resource_name;
    }
}

pub struct KdeWindowManager {
    active_window: Arc<Mutex<ActiveWindow>>,
    _kwin_script: KWinScript,
    _task: JoinHandle<()>,
    pub idle_watcher: IdleWatcherRunner,
}

impl KdeWindowManager {
    pub async fn new(idle_timeout: Duration) -> anyhow::Result<Self> {
        let mut kwin_script = KWinScript::new(Connection::session().await?);
        if kwin_script.is_loaded().await? {
            debug!("KWin script is already loaded, unloading");
            kwin_script.unload().await?;
        }
        if env::var("WAYLAND_DISPLAY").is_err()
            && env::var_os("XDG_SESSION_TYPE").unwrap_or("".into()) == "x11"
        {
            return Err(anyhow!("X11 should be tried instead"));
        }

        kwin_script.load().await.unwrap();

        let active_window = Arc::new(Mutex::new(ActiveWindow {
            caption: String::new(),
            resource_name: String::new(),
            resource_class: String::new(),
        }));
        let active_window_interface = ActiveWindowInterface {
            active_window: Arc::clone(&active_window),
        };

        let (tx, rx) = channel();
        async fn get_connection(
            active_window_interface: ActiveWindowInterface,
        ) -> zbus::Result<Connection> {
            ConnectionBuilder::session()?
                .name("com.github.anoromi.whatawhat_lib")?
                .serve_at("/com/github/anoromi/whatawhat_lib", active_window_interface)?
                .build()
                .await
        }

        let task = tokio::spawn(async move {
            match get_connection(active_window_interface).await {
                Ok(connection) => {
                    tx.send(None).unwrap();
                    loop {
                        connection.monitor_activity().await;
                    }
                }
                Err(e) => tx.send(Some(e)).unwrap(),
            }
        });
        if let Some(error) = rx.recv().unwrap() {
            return Err(anyhow!("Failed to run a DBus interface: {error}"));
        }

        Ok(Self {
            active_window,
            _kwin_script: kwin_script,
            _task: task,
            idle_watcher: IdleWatcherRunner::new(idle_timeout.as_millis() as u32)?,
        })
    }
}

#[async_trait]
impl WindowManager for KdeWindowManager {
    async fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        send_active_window(&self.active_window).await
    }

    async fn is_idle(&mut self) -> Result<bool> {
        let status_guard = self.idle_watcher.current_idle_status.lock().await;
        match *status_guard {
            Some(Status::Active { .. }) => Ok(false),
            Some(Status::Idle { .. }) => Ok(true),
            None => Ok(false),
        }
    }
}
