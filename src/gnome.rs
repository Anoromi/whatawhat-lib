use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, trace};
use zbus::blocking::Connection;

use crate::{
    ActiveWindowData, Error, Result, WindowManager,
    config::WatcherConfig,
    linux_desktop::{DesktopInfo, LinuxDesktopInfo},
    simple_cache::SimpleCache,
    utils::{is_gnome, is_x11},
};

pub struct GnomeWindowWatcher {
    pub dbus_connection: Connection,
    pub last_title: String,
    pub last_app_id: String,
    pub idle_timeout: Duration,
    pub desktop_info_cache: SimpleCache<String, DesktopInfo>,
    pub linux_desktop_info: LinuxDesktopInfo,
    pub gnome_dbus_config: crate::config::GnomeDbusConfig,
}

#[derive(Deserialize, Default)]
struct WindowData {
    title: String,
    wm_class: String,
}

impl GnomeWindowWatcher {
    fn get_window_data(&self) -> Result<WindowData> {
        let call_response = self.dbus_connection.call_method(
            Some(self.gnome_dbus_config.window_service.as_str()),
            self.gnome_dbus_config.window_path.as_str(),
            Some(self.gnome_dbus_config.window_interface.as_str()),
            self.gnome_dbus_config.window_method.as_str(),
            &(),
        );

        match call_response {
            Ok(json) => {
                let json: String = json
                    .body()
                    .deserialize()
                    .map_err(|_| Error::dbus_response_parse_failed("DBus interface cannot be parsed as string"))?;
                serde_json::from_str(&json).map_err(|_| {
                    Error::dbus_response_parse_failed(format!(
                        "DBus interface org.gnome.shell.extensions.FocusedWindow returned wrong JSON: {json}"
                    ))
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

    fn get_idle_time_data(&self) -> Result<u64> {
        let call_response = self.dbus_connection.call_method(
            Some(self.gnome_dbus_config.idle_service.as_str()),
            self.gnome_dbus_config.idle_path.as_str(),
            Some(self.gnome_dbus_config.idle_interface.as_str()),
            self.gnome_dbus_config.idle_method.as_str(),
            &(),
        );
        let result = call_response
            .map_err(|e| Error::dbus_idle_time_failed(e.to_string()))?
            .body()
            .deserialize::<u64>()
            .map_err(|_| Error::idle_time_deserialization_failed())?;
        Ok(result)
    }
}

impl GnomeWindowWatcher {
    pub fn new(config: WatcherConfig) -> Result<Self> {
        let loader = || -> Result<Self> {
            let watcher = Self {
                dbus_connection: Connection::session()?,
                last_app_id: String::new(),
                last_title: String::new(),
                idle_timeout: config.idle_timeout,
                desktop_info_cache: SimpleCache::new(config.cache_config.clone()),
                linux_desktop_info: LinuxDesktopInfo::new(),
                gnome_dbus_config: config.gnome_dbus_config.clone(),
            };
            watcher.get_window_data()?;
            Ok(watcher)
        };

        if is_x11() {
            return Err(Error::should_use_x11());
        }

        if !is_gnome() {
            return Err(Error::not_gnome_runtime());
        }

        debug!("Gnome Wayland detected");

        let mut watcher: Result<Self> = Err(Error::dbus_call_failed("failed to initialize GNOME watcher after retries"));
        for _ in 0..3 {
            watcher = loader();
            if let Err(e) = &watcher {
                debug!("Failed to load Gnome Wayland watcher: {e}");
                std::thread::sleep(std::time::Duration::from_secs(3));
            } else {
                break;
            }
        }
        watcher
    }
}

impl WindowManager for GnomeWindowWatcher {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        let data = self.get_window_data();
        if let Err(e) = data {
            if e.to_string().contains("Object does not exist at path") {
                trace!("The extension seems to have stopped");
                return Err(Error::gnome_extension_stopped());
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
            Some(extra_info) => (extra_info.process_path, extra_info.app_name),
            None => {
                if let Some(extra_info) = self.linux_desktop_info.get_extra_info(&self.last_app_id)
                {
                    self.desktop_info_cache
                        .set(self.last_app_id.clone(), extra_info.clone());
                    (extra_info.process_path, extra_info.app_name)
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

    fn is_idle(&mut self) -> Result<bool> {
        let data = self.get_idle_time_data()?;
        Ok(data > self.idle_timeout.as_millis() as u64)
    }
}
