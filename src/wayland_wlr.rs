use crate::ActiveWindowData;
use crate::WindowManager;
use crate::idle::Status;
use crate::wayland_idle::IdleWatcherRunner;

use super::wl_connection::WlEventConnection;
use super::wl_connection::subscribe_state;
use anyhow::anyhow;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, trace, warn};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, event_created_child, globals::GlobalListContents,
    protocol::wl_registry,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    Event as HandleEvent, State as HandleState, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    EVT_TOPLEVEL_OPCODE, Event as ManagerEvent, ZwlrForeignToplevelManagerV1,
};

struct WindowData {
    app_id: String,
    title: String,
}

struct ToplevelState {
    windows: HashMap<String, WindowData>,
    current_window_id: Option<String>,
}

impl ToplevelState {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            current_window_id: None,
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: <ZwlrForeignToplevelManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ManagerEvent::Toplevel { toplevel } => {
                debug!("Toplevel handle is received {}", toplevel.id());
                state.windows.insert(
                    toplevel.id().to_string(),
                    WindowData {
                        app_id: "unknown".into(),
                        title: "unknown".into(),
                    },
                );
            }
            ManagerEvent::Finished => {
                error!("Toplevel manager is finished, the application may crash");
            }
            _ => (),
        };
    }

    event_created_child!(ToplevelState, ZwlrForeignToplevelManagerV1, [
        EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

subscribe_state!(wl_registry::WlRegistry, GlobalListContents, ToplevelState);
subscribe_state!(wl_registry::WlRegistry, (), ToplevelState);

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for ToplevelState {
    fn event(
        toplevel_state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: <ZwlrForeignToplevelHandleV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id().to_string();
        let window = toplevel_state.windows.get_mut(&id);
        if let Some(window) = window {
            match event {
                HandleEvent::Title { title } => {
                    trace!("Title is changed for {id}: {title}");
                    window.title = title;
                }
                HandleEvent::AppId { app_id } => {
                    trace!("App ID is changed for {id}: {app_id}");
                    window.app_id = app_id;
                }
                HandleEvent::State { state } => {
                    trace!("State is changed for {id}: {state:?}");
                    if state.contains(&(HandleState::Activated as u8)) {
                        trace!("Window is activated: {id}");
                        toplevel_state.current_window_id = Some(id);
                    }
                }
                HandleEvent::Done => trace!("Done: {id}"),
                HandleEvent::Closed => {
                    trace!("Window is closed: {id}");
                    if toplevel_state.windows.remove(&id).is_none() {
                        warn!("Window is already removed: {id}");
                    }
                }
                _ => (),
            };
        } else {
            error!("Window is not found: {id}");
        }
    }
}

pub struct WaylandWindowWatcherInner {
    connection: WlEventConnection<ToplevelState>,
    toplevel_state: ToplevelState,
}

impl WaylandWindowWatcherInner {
    pub async fn new() -> anyhow::Result<Self> {
        let mut connection: WlEventConnection<ToplevelState> = WlEventConnection::connect()?;
        connection.get_foreign_toplevel_manager()?;

        let mut toplevel_state = ToplevelState::new();

        connection
            .event_queue
            .roundtrip(&mut toplevel_state)
            .unwrap();

        Ok(Self {
            connection,
            toplevel_state,
        })
    }

    pub fn run_iteration(&mut self) -> anyhow::Result<ActiveWindowData> {
        self.connection
            .event_queue
            .roundtrip(&mut self.toplevel_state)
            .map_err(|e| anyhow!("Event queue is not processed: {e}"))?;

        let active_window_id = self
            .toplevel_state
            .current_window_id
            .as_ref()
            .ok_or(anyhow!("Current window is unknown"))?;
        let active_window = self
            .toplevel_state
            .windows
            .get(active_window_id)
            .ok_or(anyhow!(
                "Current window is not found by ID {active_window_id}"
            ))?;

        Ok(ActiveWindowData {
            app_identifier: Some(active_window.app_id.clone().into()),
            process_path: todo!(),
            window_title: active_window.title.clone().into(),
            app_name: todo!(),
        })
    }
}

pub struct WaylandWindowWatcher {
    pub stop_signal: mpsc::Sender<()>,
    pub handle: JoinHandle<()>,
    pub current_window_data: Arc<Mutex<Option<ActiveWindowData>>>,
    pub idle_watcher: IdleWatcherRunner,
}

const WINDOW_WATCHER_INTERVAL: Duration = Duration::from_secs(1);

impl WaylandWindowWatcher {
    pub async fn new(timeout: Duration) -> anyhow::Result<Self> {
        let mut window_watcher = WaylandWindowWatcherInner::new().await?;
        let (stop_signal, stop_signal_receiver) = mpsc::channel();
        let current_window_data = Arc::new(Mutex::new(None));

        let handle = {
            let current_window_data = current_window_data.clone();
            thread::spawn(move || {
                loop {
                    match window_watcher.run_iteration() {
                        Ok(active_window_data) => {
                            let mut current_window_data = current_window_data.blocking_lock();
                            *current_window_data = Some(active_window_data);
                        }
                        Err(e) => {
                            error!("Error running idle watcher: {}", e);
                        }
                    }

                    thread::sleep(WINDOW_WATCHER_INTERVAL);
                    if let Ok(_) = stop_signal_receiver.try_recv() {
                        break;
                    }
                }
            })
        };
        Ok(Self {
            stop_signal,
            handle,
            current_window_data,
            idle_watcher: IdleWatcherRunner::new(timeout.as_millis() as u32)?,
        })
    }
}

impl Drop for WaylandWindowWatcher {
    fn drop(&mut self) {
        let _ = self.stop_signal.send(());
    }
}

#[async_trait]
impl WindowManager for WaylandWindowWatcher {
    async fn get_active_window_data(&mut self) -> anyhow::Result<ActiveWindowData> {
        let current_window_data = self.current_window_data.lock().await.clone();
        match current_window_data {
            Some(active_window_data) => Ok(active_window_data),
            None => Err(anyhow!("No active window data")),
        }
    }

    async fn is_idle(&mut self) -> anyhow::Result<bool> {
        let status_guard = self.idle_watcher.current_idle_status.lock().await;
        match *status_guard {
            Some(Status::Active { .. }) => Ok(false),
            Some(Status::Idle { .. }) => Ok(true),
            None => Ok(false),
        }
    }
}
