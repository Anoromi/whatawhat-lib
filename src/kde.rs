/*
 * This uses a hack with KWin scripts in order to receive the active window.
 * For the moment of writing, KWin doesn't implement the appropriate protocols to get a top level window.
 * Inspired by https://github.com/k0kubun/xremap/
 */
use crate::idle::Status;
use crate::linux_desktop::{DesktopInfo, LinuxDesktopInfo};
use crate::simple_cache::SimpleCache;
use crate::wayland_idle::IdleWatcherRunner;
use crate::{ActiveWindowData, WindowManager, config::WatcherConfig};
use anyhow::{Context, Result, anyhow};
use std::env::{self, temp_dir};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{debug, error};
use zbus::blocking::{Connection, connection::Builder as ConnectionBuilder};
use zbus::interface;

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

    fn load(&mut self) -> anyhow::Result<()> {
        let path = temp_dir().join("whatawhat-lib.js");
        std::fs::write(&path, KWIN_SCRIPT).with_context(|| "Failed to create kwin script")?;

        let number = self.get_registered_number(&path)?;
        let result = self.start(number);
        std::fs::remove_file(&path)?;
        self.is_loaded = true;

        result
    }

    fn is_loaded(&self) -> anyhow::Result<bool> {
        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "isScriptLoaded",
                &KWIN_SCRIPT_NAME,
            )?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    fn get_registered_number(&self, path: &Path) -> anyhow::Result<i32> {
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
            )?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    fn unload(&self) -> anyhow::Result<bool> {
        self.dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "unloadScript",
                &KWIN_SCRIPT_NAME,
            )?
            .body()
            .deserialize()
            .map_err(std::convert::Into::into)
    }

    fn start(&self, script_number: i32) -> anyhow::Result<()> {
        debug!("Starting KWin script {script_number}");

        let path = if self.get_major_version() < 6 {
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
            .with_context(|| "Error on starting the script")?;
        Ok(())
    }

    fn get_major_version(&self) -> i8 {
        if let Ok(version) = Self::get_major_version_from_env() {
            debug!("KWin version from KDE_SESSION_VERSION: {version}");

            version
        } else {
            self.get_major_version_from_dbus().unwrap_or_else(|e| {
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

    fn get_major_version_from_dbus(&self) -> anyhow::Result<i8> {
        let support_information: String = self
            .dbus_connection
            .call_method(
                Some("org.kde.KWin"),
                "/KWin",
                Some("org.kde.KWin"),
                "supportInformation",
                &(),
            )?
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
        if let Err(e) = self.unload() {
            error!("Problem during stopping KWin script: {e}");
        };
    }
}

fn send_active_window(
    active_window: &Arc<Mutex<ActiveWindow>>,
) -> anyhow::Result<ActiveWindowData> {
    let active_window = active_window.lock().expect("Mutex poisoned");

    Ok(ActiveWindowData {
        window_title: active_window.caption.clone().into(),
        app_identifier: Some(active_window.resource_name.clone().into()),
        process_path: active_window.process_path.clone(),
        app_name: active_window.app_name.clone(),
    })
}

struct ActiveWindow {
    resource_class: Arc<str>,
    resource_name: Arc<str>,
    caption: Arc<str>,
    process_path: Option<Arc<str>>,
    app_name: Option<Arc<str>>,
}

struct ActiveWindowInterface {
    active_window: Arc<Mutex<ActiveWindow>>,
    desktop_info_cache: SimpleCache<String, DesktopInfo>,
    linux_desktop_info: LinuxDesktopInfo,
}

#[interface(name = "com.github.anoromi.whatawhat_lib")]
impl ActiveWindowInterface {
    fn notify_active_window(
        &mut self,
        caption: String,
        resource_class: String,
        resource_name: String,
        _pid: i32,
    ) {
        debug!(
            "Active window class: \"{resource_class}\", name: \"{resource_name}\", caption: \"{caption}\""
        );

        let (process_path, app_name) = match self.desktop_info_cache.get(&resource_name) {
            Some(extra_info) => (Some(extra_info.process_path), Some(extra_info.app_name)),
            None => {
                if let Some(extra_info) = self.linux_desktop_info.get_extra_info(&resource_name) {
                    self.desktop_info_cache
                        .set(resource_name.clone(), extra_info.clone());
                    (Some(extra_info.process_path), Some(extra_info.app_name))
                } else {
                    (None, None)
                }
            }
        };

        let mut active_window = self.active_window.lock().expect("Mutex poisoned");
        active_window.caption = caption.into();
        active_window.resource_class = resource_class.into();
        active_window.resource_name = resource_name.into();

        active_window.process_path = process_path;
        active_window.app_name = app_name;
    }
}

pub struct KdeWindowManager {
    active_window: Arc<Mutex<ActiveWindow>>,
    _kwin_script: KWinScript,
    dbus_connection: Connection,
    pub idle_watcher: IdleWatcherRunner,
}

impl KdeWindowManager {
    pub fn new(config: WatcherConfig) -> anyhow::Result<Self> {
        let mut kwin_script = KWinScript::new(Connection::session()?);
        if kwin_script.is_loaded()? {
            debug!("KWin script is already loaded, unloading");
            kwin_script.unload()?;
        }
        if env::var("WAYLAND_DISPLAY").is_err()
            && env::var_os("XDG_SESSION_TYPE").unwrap_or("".into()) == "x11"
        {
            return Err(anyhow!("X11 should be tried instead"));
        }

        kwin_script.load().unwrap();

        let active_window = Arc::new(Mutex::new(ActiveWindow {
            caption: "".into(),
            resource_name: "".into(),
            resource_class: "".into(),
            process_path: None,
            app_name: None,
        }));
        let active_window_interface = ActiveWindowInterface {
            active_window: Arc::clone(&active_window),
            desktop_info_cache: SimpleCache::new(config.cache_config),
            linux_desktop_info: LinuxDesktopInfo::new(),
        };

        // Build the DBus connection and register the interface synchronously (no extra thread).
        let dbus_connection = ConnectionBuilder::session()?
            .name("com.github.anoromi.whatawhat_lib")?
            .serve_at("/com/github/anoromi/whatawhat_lib", active_window_interface)?
            .build()
            .map_err(|e| anyhow!("Failed to run a DBus interface: {e}"))?;

        // Intentionally avoid initial monitor_activity() here to ensure we only process
        // events when the caller invokes methods (run-when-called semantics).

        Ok(Self {
            active_window,
            _kwin_script: kwin_script,
            dbus_connection,
            idle_watcher: IdleWatcherRunner::new(config.idle_timeout.as_millis() as u32)?,
        })
    }

    fn pump_dbus(&self) {
        // Best-effort: process any pending DBus activity inline.
        // monitor_activity blocks waiting for IO when nothing is pending on real KDE,
        // but KWin sends promptly on activation events; calls here are short in practice.
        // If this turns out to block undesirably in some environments, consider adding
        // a timed variant or switching to async with a local runtime.
        self.dbus_connection.monitor_activity();
    }
}

impl WindowManager for KdeWindowManager {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        // Process any pending DBus events so our state is up-to-date when queried.
        self.pump_dbus();
        send_active_window(&self.active_window)
    }

    fn is_idle(&mut self) -> Result<bool> {
        // Keep consistency by pumping DBus here too, in case user calls this independently.
        self.pump_dbus();

        let status_guard = self
            .idle_watcher
            .current_idle_status
            .lock()
            .expect("Mutex poisoned");
        match *status_guard {
            Some(Status::Active { .. }) => Ok(false),
            Some(Status::Idle { .. }) => Ok(true),
            None => Ok(false),
        }
    }
}
