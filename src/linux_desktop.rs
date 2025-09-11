use std::sync::Arc;

use freedesktop_desktop_entry::{DesktopEntry, unicase::Ascii};
use tracing::warn;

pub struct LinuxDesktopInfo {
    entries: Vec<DesktopEntry>,
}

#[derive(Clone)]
pub struct DesktopInfo {
    pub app_name: Arc<str>,
    pub process_path: Arc<str>,
}

impl LinuxDesktopInfo {
    pub fn new() -> Self {
        let entries = freedesktop_desktop_entry::desktop_entries(&["en_US".to_string()]);
        Self { entries }
    }

    pub fn get_extra_info(&self, app_id: &str) -> Option<DesktopInfo> {
        if !app_id.is_ascii() {
            warn!("App ID is not ASCII: {}", app_id);
            return None;
        }
        let hm = Ascii::new(app_id);
        let entry = freedesktop_desktop_entry::find_app_by_id(&self.entries, hm)?;
        dbg!(&entry);
        let exec_params = match entry.parse_exec() {
            Ok(params) => params,
            Err(e) => {
                warn!("Failed to parse exec params for {}: {}", app_id, e);
                return None;
            }
        };
        Some(DesktopInfo {
            app_name: entry.name(&["en_US".to_string()]).unwrap().into(),
            process_path: exec_params.into_iter().next()?.into(),
        })
    }
}
