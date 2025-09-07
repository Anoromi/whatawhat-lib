#[cfg(feature = "gnome")]
pub mod gnome;
#[cfg(feature = "kde")]
pub mod kde;
#[cfg(feature = "wayland")]
pub mod wayland_idle;
#[cfg(feature = "wayland")]
pub mod wayland_wlr;
#[cfg(feature = "win")]
pub mod win;
#[cfg(feature = "win")]
pub mod windows_desktop;
#[cfg(feature = "wayland")]
pub mod wl_connection;
#[cfg(feature = "x11")]
pub mod x11;

#[cfg(feature = "macos")]
pub mod macos;

pub mod idle;
#[cfg(any(
    feature = "x11",
    feature = "wayland",
    feature = "gnome",
    feature = "kde"
))]
pub mod linux_desktop;
pub mod simple_cache;
pub mod utils;
pub mod gnome_install;
pub mod config;

use std::sync::Arc;

use anyhow::Result;
#[cfg(any(
    feature = "x11",
    feature = "wayland",
    feature = "gnome",
    feature = "kde"
))]
use tracing::info;

use crate::config::WatcherConfig;

#[derive(Debug, Clone)]
pub struct ActiveWindowData {
    /// Name of the window. For example 'bash in hello' or 'Document 1' or 'Vibing in YouTube -
    /// Chrome'
    pub window_title: Arc<str>,
    /// Represents an identifier of the application.
    /// On windows it is a process name. For example `C:\Windows\System32\cmd.exe`
    /// On x11 it is a process name. For example `/home/etc/nvim``
    /// On wayland, gnome, and kde it's a resource class. For example `org.kde.kate`
    pub process_path: Option<Arc<str>>,
    pub app_identifier: Option<Arc<str>>,
    pub app_name: Option<Arc<str>>,
}

/// Intended to serve as a contract windows and linux systems must implement.
#[cfg_attr(feature = "mock", mockall::automock)]
pub trait WindowManager {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData>;

    /// Retrieve amount of time user has been inactive in milliseconds
    fn is_idle(&mut self) -> Result<bool>;
}

/// Serves as a cross-compatible WindowManager implementation.
pub struct GenericWindowManager {
    inner: Box<dyn WindowManager>,
}

impl GenericWindowManager {
    pub fn new(_config: WatcherConfig) -> Result<Self> {
        #[cfg(feature = "win")]
        {
            use win::WindowsWindowManager;
            return Ok(Self {
                inner: Box::new(WindowsWindowManager::new(_config)),
            });
        }
        #[cfg(feature = "gnome")]
        {
            use gnome::GnomeWindowWatcher;
            let watcher = GnomeWindowWatcher::new(_config.clone());
            match watcher {
                Ok(watcher) => {
                    let result = Ok(Self {
                        inner: Box::new(watcher),
                    });
                    info!("Loaded Gnome Wayland watcher");
                    return result;
                }
                Err(e) => {
                    use tracing::warn;
                    warn!("Failed to load Gnome Wayland watcher: {e}");
                }
            }
        }
        #[cfg(feature = "kde")]
        {
            use kde::KdeWindowManager;
            let watcher = KdeWindowManager::new(_config.clone());
            match watcher {
                Ok(watcher) => {
                    let result = Ok(Self {
                        inner: Box::new(watcher),
                    });
                    info!("Loaded Kde wayland watcher");
                    return result;
                }
                Err(e) => {
                    use tracing::warn;
                    warn!("Failed to load Kde Wayland watcher: {e}");
                }
            }
        }
        #[cfg(feature = "wayland")]
        {
            use wayland_wlr::WaylandWindowWatcher;
            let watcher = WaylandWindowWatcher::new(_config.clone());
            match watcher {
                Ok(watcher) => {
                    let result = Ok(Self {
                        inner: Box::new(watcher),
                    });
                    info!("Loaded Wayland window watcher");
                    return result;
                }
                Err(e) => {
                    use tracing::warn;
                    warn!("Failed to load Wayland window watcher: {e}");
                }
            }
        }
        #[cfg(feature = "x11")]
        {
            use x11::LinuxWindowManager;
            let watcher = LinuxWindowManager::new(_config.clone());
            match watcher {
                Ok(watcher) => {
                    let result = Ok(Self {
                        inner: Box::new(watcher),
                    });
                    info!("Loaded X11 window manager");
                    return result;
                }
                Err(e) => {
                    use tracing::warn;
                    warn!("Failed to load X11 window manager: {e}");
                }
            }
        }
        #[cfg(feature = "macos")]
        {
            use macos::MacosManger;
            return Ok(Self {
                inner: Box::new(MacosManger::new(_config)?),
            });
        }
        #[allow(unreachable_code)]
        {
            Err(anyhow::anyhow!("No window manager was selected"))
        }
    }
}

impl WindowManager for GenericWindowManager {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        self.inner.get_active_window_data()
    }

    fn is_idle(&mut self) -> Result<bool> {
        self.inner.is_idle()
    }
}
