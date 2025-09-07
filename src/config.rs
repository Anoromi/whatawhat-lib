use std::time::Duration;

use derive_builder::Builder;

use crate::simple_cache::CacheConfig;

const DEFAULT_CACHE_CONFIG: CacheConfig = CacheConfig {
    ttl: Duration::from_secs(60 * 10),
    max_size: 100,
};

#[derive(Builder)]
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
}
