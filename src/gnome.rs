use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, trace};
use zbus::Connection;

use crate::{
    ActiveWindowData, WindowManager,
    linux_desktop::{DesktopInfo, LinuxDesktopInfo},
    simple_cache::{CacheConfig, SimpleCache},
    utils::{is_gnome, is_x11},
};

pub struct GnomeWindowWatcher {
    dbus_connection: Connection,
    last_title: String,
    last_app_id: String,
    app_name: String,
    process_path: String,
    idle_timeout: Duration,
    desktop_info_cache: SimpleCache<String, DesktopInfo>,
    linux_desktop_info: LinuxDesktopInfo,
}

#[derive(Deserialize, Default)]
struct WindowData {
    title: String,
    wm_class: String,
}

impl GnomeWindowWatcher {
    async fn get_window_data(&self) -> anyhow::Result<WindowData> {
        let call_response = self
            .dbus_connection
            .call_method(
                Some("org.gnome.Shell"),
                "/org/gnome/shell/extensions/FocusedWindow",
                Some("org.gnome.shell.extensions.FocusedWindow"),
                "Get",
                &(),
            )
            .await;

        match call_response {
            Ok(json) => {
                let json: String = json
                    .body()
                    .deserialize()
                    .with_context(|| "DBus interface cannot be parsed as string")?;
                serde_json::from_str(&json).with_context(|| {
                    format!("DBus interface org.gnome.shell.extensions.FocusedWindow returned wrong JSON: {json}")
                })
            }
            Err(e) => {
                if e.to_string().contains("No window in focus") {
                    trace!("No window is active");
                    Ok(WindowData::default())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn get_idle_time_data(&self) -> Result<u64> {
        let call_response = self
            .dbus_connection
            .call_method(
                Some("org.gnome.Shell"),
                "/org/gnome/Mutter/IdleMonitor/Core",
                Some("org.gnome.Mutter.IdleMonitor"),
                "GetIdletime",
                &(),
            )
            .await;
        let result = call_response
            .with_context(|| "Failed to get idle time")?
            .body()
            .deserialize::<u64>()
            .with_context(|| "Failed to deserialize idle time")?;
        Ok(result)
    }
}

impl GnomeWindowWatcher {
    pub async fn new(idle_timeout: Duration) -> Result<Self> {
        let loader = async || -> Result<Self> {
            let watcher = Self {
                dbus_connection: Connection::session().await?,
                last_app_id: String::new(),
                last_title: String::new(),
                idle_timeout,
                app_name: String::new(),
                process_path: String::new(),
                desktop_info_cache: SimpleCache::new(CacheConfig {
                    ttl: Duration::from_secs(60),
                    max_size: 1000,
                }),
                linux_desktop_info: LinuxDesktopInfo::new(),
            };
            watcher.get_window_data().await?;
            Ok(watcher)
        };

        if is_x11() {
            return Err(anyhow!("X11 should be tried instead"));
        }

        if !is_gnome() {
            return Err(anyhow!("The runtime doesn't seem to be Gnome"));
        }

        debug!("Gnome Wayland detected");

        let mut watcher = Err(anyhow::anyhow!(""));
        for _ in 0..3 {
            watcher = loader().await;
            if let Err(e) = &watcher {
                debug!("Failed to load Gnome Wayland watcher: {e}");
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }
        watcher
    }
}

#[async_trait]
impl WindowManager for GnomeWindowWatcher {
    async fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        let data = self.get_window_data().await;
        if let Err(e) = data {
            if e.to_string().contains("Object does not exist at path") {
                trace!("The extension seems to have stopped");
                return Err(anyhow::anyhow!("The extension seems to have stopped"));
            }
            return Err(e);
        }
        let data = data?;

        if data.wm_class != self.last_app_id || data.title != self.last_title {
            debug!(
                r#"Changed window app_id="{}", title="{}""#,
                data.wm_class, data.title
            );
            self.last_app_id = data.wm_class;
            self.last_title = data.title;
        }

        let (process_path, app_name) = match self.desktop_info_cache.get(&self.last_app_id) {
            Some(extra_info) => (Some(extra_info.process_path), Some(extra_info.app_name)),
            None => {
                if let Some(extra_info) = self.linux_desktop_info.get_extra_info(&self.last_app_id)
                {
                    self.desktop_info_cache
                        .set(self.last_app_id.clone(), extra_info.clone());
                    (Some(extra_info.process_path), Some(extra_info.app_name))
                } else {
                    (None, None)
                }
            }
        };

        Ok(ActiveWindowData {
            window_title: self.last_title.clone().into(),
            app_identifier: Some(self.last_app_id.clone().into()),
            process_path,
            app_name,
        })
    }

    async fn is_idle(&mut self) -> Result<bool> {
        let data = self.get_idle_time_data().await?;
        Ok(data > self.idle_timeout.as_millis() as u64)
    }
}
