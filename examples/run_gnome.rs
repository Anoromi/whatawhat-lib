#[cfg(feature = "gnome")]
use {
    std::time::Duration,
    tracing::Level,
    whatawhat_lib::{WindowManager as _, gnome::GnomeWindowWatcher, config::WatcherConfig},
};

#[cfg(feature = "gnome")]
fn main() {
    let config = WatcherConfig {
        idle_timeout: Duration::from_secs(10),
        ..Default::default()
    };
    let mut window_manager = GnomeWindowWatcher::new(config).unwrap();

    tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // sets this to be the default, global subscriber for this application.
        .init();

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
    println!("Not supported");
}
