#[cfg(feature = "gnome")]
use {
    std::time::Duration,
    tracing::Level,
    whatawhat_lib::{WindowManager as _, gnome::GnomeWindowWatcher, config::{WatcherConfig, GnomeDbusConfig}},
};

#[cfg(feature = "gnome")]
fn main() {
    // Create custom GNOME DBus configuration
    let custom_gnome_config = GnomeDbusConfig {
        // Custom window data DBus settings (these are the defaults)
        window_service: "org.gnome.Shell".to_string(),
        window_path: "/org/gnome/shell/extensions/WhatawhatFocusedWindow".to_string(),
        window_interface: "org.gnome.shell.extensions.WhatawhatFocusedWindow".to_string(),
        window_method: "Get".to_string(),
        // Custom idle time DBus settings (these are the defaults)
        idle_service: "org.gnome.Shell".to_string(),
        idle_path: "/org/gnome/Mutter/IdleMonitor/Core".to_string(),
        idle_interface: "org.gnome.Mutter.IdleMonitor".to_string(),
        idle_method: "GetIdletime".to_string(),
    };

    let config = WatcherConfig {
        idle_timeout: Duration::from_secs(10),
        gnome_dbus_config: custom_gnome_config,
        ..Default::default()
    };

    let mut window_manager = GnomeWindowWatcher::new(config).unwrap();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    println!("Using custom GNOME DBus configuration:");
    println!("Window service: {}", window_manager.gnome_dbus_config.window_service);
    println!("Idle service: {}", window_manager.gnome_dbus_config.idle_service);

    loop {
        let active_window = window_manager.get_active_window_data().unwrap();
        println!("Active window: {:?}", active_window);
        let idle_time = window_manager.is_idle().unwrap();
        println!("Idle time: {:?}", idle_time);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[cfg(not(feature = "gnome"))]
fn main() {
    println!("GNOME feature not enabled");
}
