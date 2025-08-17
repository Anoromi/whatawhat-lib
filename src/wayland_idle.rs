use crate::idle::{self, Status};

use super::wl_connection::{WlEventConnection, subscribe_state};
use anyhow::Context as _;
use chrono::{TimeDelta, Utc};
use std::{
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::Mutex;
use tracing::{error, info};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    globals::GlobalListContents,
    protocol::{wl_registry, wl_seat::WlSeat},
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::Event as IdleNotificationV1Event;
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::ExtIdleNotificationV1;
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

pub struct WatcherState {
    idle_notification: ExtIdleNotificationV1,
    pub idle_state: idle::Tracker,
}

impl Drop for WatcherState {
    fn drop(&mut self) {
        info!("Releasing idle notification");
        self.idle_notification.destroy();
    }
}

impl WatcherState {
    fn new(idle_notification: ExtIdleNotificationV1, idle_timeout: TimeDelta) -> Self {
        Self {
            idle_notification,
            idle_state: idle::Tracker::new(Utc::now(), idle_timeout),
        }
    }

    fn idle(&mut self) {
        let time = Utc::now();
        self.idle_state.mark_idle(time);
    }

    fn resume(&mut self) {
        let time = Utc::now();
        self.idle_state.mark_not_idle(time);
    }
}

subscribe_state!(wl_registry::WlRegistry, GlobalListContents, WatcherState);
subscribe_state!(wl_registry::WlRegistry, (), WatcherState);
subscribe_state!(WlSeat, (), WatcherState);
subscribe_state!(ExtIdleNotifierV1, (), WatcherState);

impl Dispatch<ExtIdleNotificationV1, ()> for WatcherState {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: <ExtIdleNotificationV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let IdleNotificationV1Event::Idled = event {
            state.idle();
        } else if let IdleNotificationV1Event::Resumed = event {
            state.resume();
        }
    }
}

pub struct IdleWatcher {
    connection: WlEventConnection<WatcherState>,
    pub watcher_state: WatcherState,
}

impl IdleWatcher {
    pub fn new(timeout: u32) -> anyhow::Result<Self> {
        let mut connection: WlEventConnection<WatcherState> = WlEventConnection::connect()?;
        connection.get_ext_idle()?;

        let mut watcher_state = WatcherState::new(
            connection.get_ext_idle_notification(timeout).unwrap(),
            TimeDelta::milliseconds(timeout as i64),
        );
        connection
            .event_queue
            .roundtrip(&mut watcher_state)
            .unwrap();

        Ok(Self {
            connection,
            watcher_state,
        })
    }

    pub fn run_iteration(&mut self) -> anyhow::Result<Status> {
        self.connection
            .event_queue
            .roundtrip(&mut self.watcher_state)
            .with_context(|| "Event queue is not processed")?;
        Ok(self.watcher_state.idle_state.get_reactive(Utc::now())?)
    }
}

pub struct IdleWatcherRunner {
    pub stop_signal: mpsc::Sender<()>,
    pub handle: JoinHandle<()>,
    pub current_idle_status: Arc<Mutex<Option<idle::Status>>>,
}

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(10);

impl IdleWatcherRunner {
    pub fn new(timeout: u32) -> anyhow::Result<Self> {
        let mut idle_watcher = IdleWatcher::new(timeout)?;
        let (stop_signal, stop_signal_receiver) = mpsc::channel();
        let current_idle_status = Arc::new(Mutex::new(None));

        let handle = {
            let current_idle_status = current_idle_status.clone();
            thread::spawn(move || {
                // while let Ok(_) = stop_signal_receiver.recv() {
                loop {
                    match idle_watcher.run_iteration() {
                        Ok(status) => {
                            let mut current_idle_status = current_idle_status.blocking_lock();
                            *current_idle_status = Some(status);
                        }
                        Err(e) => {
                            error!("Error running idle watcher: {}", e);
                        }
                    }

                    thread::sleep(IDLE_CHECK_INTERVAL);
                    if let Ok(_) = stop_signal_receiver.try_recv() {
                        break;
                    }
                }
            })
        };
        Ok(Self {
            stop_signal,
            handle,
            current_idle_status,
        })
    }
}

impl Drop for IdleWatcherRunner {
    fn drop(&mut self) {
        let _ = self.stop_signal.send(());
    }
}
