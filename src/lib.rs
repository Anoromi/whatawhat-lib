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
#[cfg(feature = "wayland")]
pub mod wl_connection;
#[cfg(feature = "x11")]
pub mod x11;

pub mod idle;
pub mod utils;

#[cfg(feature = "win")]
extern crate windows;

// #[cfg(feature = "x11")]
// extern crate xcb;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ActiveWindowData {
    /// Name of the window. For example 'bash in hello' or 'Document 1' or 'Vibing in YouTube -
    /// Chrome'
    pub window_title: Arc<str>,
    /// Represents an identifier of the application.
    /// On windows it is a process name. For example `C:\Windows\System32\cmd.exe`
    /// On x11 it is a process name. For example `/home/etc/nvim``
    /// On wayland, gnome, and kde it's a resource class. For example `org.kde.kate`
    pub app_identifier: Arc<str>,
}

/// Intended to serve as a contract windows and linux systems must implement.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait WindowManager {
    async fn get_active_window_data(&mut self) -> Result<ActiveWindowData>;

    /// Retrieve amount of time user has been inactive in milliseconds
    async fn is_idle(&mut self) -> Result<bool>;
}

/// Serves as a cross-compatible WindowManager implementation.
pub struct GenericWindowManager {
    inner: Box<dyn WindowManager + Send + Sync>,
}

impl GenericWindowManager {
    pub async fn new(idle_timeout: Duration) -> Result<Self> {
        #[cfg(feature = "win")]
        {
            use win::WindowsWindowManager;
            return Ok(Self {
                inner: Box::new(WindowsWindowManager::new(idle_timeout)),
            });
        }
        // TODO: Should try to select not select outright
        #[cfg(feature = "gnome")]
        {
            use gnome::GnomeWindowWatcher;
            let watcher = GnomeWindowWatcher::new(idle_timeout.into()).await;
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
            let watcher = KdeWindowManager::new(idle_timeout).await;
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
                    warn!("Failed to load Gnome Wayland watcher: {e}");
                }
            }
        }
        #[cfg(feature = "wayland")]
        {
            use wayland_wlr::WaylandWindowWatcher;
            let watcher = WaylandWindowWatcher::new(idle_timeout).await;
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
            let watcher = LinuxWindowManager::new(idle_timeout);
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
        #[allow(unreachable_code)]
        {
            Err(anyhow::anyhow!("No window manager was selected"))
        }
    }
}

#[async_trait]
impl WindowManager for GenericWindowManager {
    async fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        self.inner.get_active_window_data().await
    }

    async fn is_idle(&mut self) -> Result<bool> {
        self.inner.is_idle().await
    }
}
