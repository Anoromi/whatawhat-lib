use std::{env, time::Duration};

use tracing::info;

use crate::simple_cache::CacheConfig;

pub fn is_gnome() -> bool {
    if let Ok(de) = std::env::var("XDG_CURRENT_DESKTOP") {
        info!("De", de);
        de.to_lowercase().contains("gnome")
    } else {
        false
    }
}

pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("XDG_SESSION_TYPE")
            .unwrap_or("".into())
            .to_lowercase()
            .contains("wayland")
}

pub fn is_x11() -> bool {
    env::var("WAYLAND_DISPLAY").is_err()
        && env::var_os("XDG_SESSION_TYPE").unwrap_or("".into()) == "x11"
}

pub fn default_cache_config() -> CacheConfig {
    CacheConfig {
        ttl: Duration::from_secs(60),
        max_size: 1000,
    }
}
