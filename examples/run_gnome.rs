#[cfg(feature = "gnome")]
use {
    std::time::Duration,
    tracing::Level,
    whatawhat_lib::{WindowManager as _, gnome::GnomeWindowWatcher},
};

#[cfg(feature = "gnome")]
#[tokio::main]
async fn main() {
    let mut window_manager = GnomeWindowWatcher::new(Duration::from_secs(10))
        .await
        .unwrap();

    tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // sets this to be the default, global subscriber for this application.
        .init();

    loop {
        let active_window = window_manager.get_active_window_data().await.unwrap();
        println!("Active window: {:?}", active_window);
        let idle_time = window_manager.is_idle().await.unwrap();
        println!("Idle time: {:?}", idle_time);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#[cfg(not(feature = "gnome"))]
fn main() {
    println!("Not supported");
}
