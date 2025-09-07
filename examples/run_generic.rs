use std::{panic::catch_unwind, thread, time::Duration};

use tracing::Level;
use whatawhat_lib::{
    GenericWindowManager, WindowManager as _,
    config::{WatcherConfig, WatcherConfigBuilder},
};

// #[tokio::main]
fn main() {
    let thread_handle = thread::spawn(|| {
        let result = catch_unwind(|| {
            let mut window_manager = GenericWindowManager::new(
                WatcherConfigBuilder::default()
                    .am_on_main_thread(false)
                    .idle_timeout(Duration::from_secs(10))
                    .build()
                    .unwrap(),
            )
            .unwrap();

            tracing_subscriber::fmt()
                // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
                // will be written to stdout.
                .with_max_level(Level::DEBUG)
                // sets this to be the default, global subscriber for this application.
                .init();

            loop {
                let active_window = window_manager.get_active_window_data();
                println!("Active window: {:?}", active_window);
                let idle_time = window_manager.is_idle();
                println!("Idle time: {:?}", idle_time);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        dbg!(&result);
    });
    thread_handle.join().unwrap();
}
