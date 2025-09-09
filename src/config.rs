use std::time::Duration;

use derive_builder::Builder;

use crate::simple_cache::CacheConfig;

const DEFAULT_CACHE_CONFIG: CacheConfig = CacheConfig {
    ttl: Duration::from_secs(60 * 10),
    max_size: 100,
};

#[derive(Clone)]
pub struct GnomeDbusConfig {
    /// The DBus service name for window data calls
    pub window_service: String,
    /// The DBus path for window data calls
    pub window_path: String,
    /// The DBus interface for window data calls
    pub window_interface: String,
    /// The DBus method name for window data calls
    pub window_method: String,
    /// The DBus service name for idle time calls
    pub idle_service: String,
    /// The DBus path for idle time calls
    pub idle_path: String,
    /// The DBus interface for idle time calls
    pub idle_interface: String,
    /// The DBus method name for idle time calls
    pub idle_method: String,
}

impl Default for GnomeDbusConfig {
    fn default() -> Self {
        Self {
            window_service: "org.gnome.Shell".to_string(),
            window_path: "/org/gnome/shell/extensions/WhatawhatFocusedWindow".to_string(),
            window_interface: "org.gnome.shell.extensions.WhatawhatFocusedWindow".to_string(),
            window_method: "Get".to_string(),
            idle_service: "org.gnome.Shell".to_string(),
            idle_path: "/org/gnome/Mutter/IdleMonitor/Core".to_string(),
            idle_interface: "org.gnome.Mutter.IdleMonitor".to_string(),
            idle_method: "GetIdletime".to_string(),
        }
    }
}

#[derive(Clone, Default, Builder)]
pub struct WatcherConfig {
    /// The timeout for the idle watcher.
    #[builder(default = Duration::from_secs(1))]
    pub idle_timeout: Duration,
    /// The cache used for extra information like application names.
    #[builder(default = DEFAULT_CACHE_CONFIG)]
    pub cache_config: CacheConfig,
    /// If true, the watcher assumes that the watcher is being run on the main thread.
    /// Only relevant for macOS, because if not, the watcher will spawn an osascript process.
    #[builder(default = true)]
    pub am_on_main_thread: bool,
    /// The interval for the idle watcher.
    #[builder(default = Duration::from_secs(1))]
    pub idle_check_interval: Duration,
    /// Configuration for GNOME DBus calls
    #[builder(default)]
    pub gnome_dbus_config: GnomeDbusConfig,
}
