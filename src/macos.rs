use std::time::Duration;

use anyhow::{Result, anyhow};
use objc2::{AllocAnyThread, rc::Retained};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
use objc2_foundation::{NSString, ns_string};
use objc2_osa_kit::{OSALanguage, OSAScript};
use serde::{Deserialize, Serialize};
use sysinfo::{self};
use tracing::debug;

use super::ActiveWindowData;
use crate::{WindowManager, simple_cache::CacheConfig};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    unix_id: u32,
    app: String,
    title: String,
}

/// On-demand macOS manager: compiles the JXA once at construction and executes it
/// synchronously for each get_active_window_data() call. No background threads.
pub struct MacosManger {
    sysinfo: sysinfo::System,
    script: Retained<OSAScript>,
    idle_timeout: Duration,
}

impl MacosManger {
    pub fn new(idle_timeout: Duration, _cache_config: Option<CacheConfig>) -> Result<Self> {
        // Prepare OSAScript with JavaScript (JXA)
        let script = OSAScript::alloc();
        let language = unsafe { OSALanguage::languageForName(&NSString::from_str("JavaScript")) }
            .ok_or_else(|| anyhow!("Failed to get JavaScript OSALanguage"))?;
        let script = unsafe {
            OSAScript::initWithSource_language(
                script,
                ns_string!(include_str!("./print_app_status.jxa")),
                Some(&language),
            )
        };

        // Compile once up front
        let mut err: Option<_> = None;
        let _ = unsafe { script.compileAndReturnError(err.as_mut()) };
        if let Some(err) = err {
            return Err(anyhow!("compile error: {:?}", &err));
        }

        Ok(Self {
            sysinfo: sysinfo::System::new_all(),
            script,
            idle_timeout,
        })
    }
}

impl WindowManager for MacosManger {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        // Execute compiled script
        let mut err: Option<_> = None;
        let data = unsafe { self.script.executeAndReturnError(err.as_mut()) };
        if let Some(err) = err {
            return Err(anyhow!("execution error: {:?}", &err));
        }
        let json = unsafe {
            data.ok_or_else(|| anyhow!("No result from OSAScript execution"))?
                .stringValue()
        }
        .ok_or_else(|| anyhow!("Script did not return a string value"))?
        .to_string();

        debug!("Script output: {}", json);
        // Parse JXA output
        let app_info: AppInfo = serde_json::from_str(&json)
            .map_err(|e| anyhow!("Failed to parse JXA JSON: {e}; payload: {json}"))?;

        // Resolve process path via sysinfo
        // Refresh processes to ensure info is up to date
        self.sysinfo
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let pid = sysinfo::Pid::from_u32(app_info.unix_id);
        let path = self.sysinfo.process(pid);
        let process_path = match path.and_then(|p| p.exe()) {
            Some(path) => path.to_str().map(|s| s.to_string()),
            None => None,
        };

        Ok(ActiveWindowData {
            window_title: app_info.title.into(),
            process_path: process_path.map(|s| s.into()),
            app_identifier: None, // Could be a bundle ID in future; app name is below
            app_name: Some(app_info.app.into()),
        })
    }

    fn is_idle(&mut self) -> Result<bool> {
        let any_event = CGEventType(!0);
        let last_input = unsafe {
            CGEventSource::seconds_since_last_event_type(
                CGEventSourceStateID::HIDSystemState,
                any_event,
            )
        };
        Ok(last_input > self.idle_timeout.as_secs_f64())
    }
}
