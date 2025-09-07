use std::{
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use objc2::{AllocAnyThread, rc::Retained};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
use objc2_foundation::{NSString, ns_string};
use objc2_osa_kit::{OSALanguage, OSAScript};
use serde::{Deserialize, Serialize};
use sysinfo::{self};

use super::ActiveWindowData;
use crate::{WindowManager, config::WatcherConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    runner: MacosRunner,
    // script: Retained<OSAScript>,
    idle_timeout: Duration,
}

impl MacosManger {
    pub fn new(config: WatcherConfig) -> Result<Self> {
        let runner = if config.am_on_main_thread {
            create_on_main_thread_osascript_process()?
        } else {
            dbg!("Creating separate osascript process");
            create_separate_osascript_process(config.idle_check_interval)?
        };

        Ok(Self {
            sysinfo: sysinfo::System::new_all(),
            runner,
            idle_timeout: config.idle_timeout,
        })
    }
}

impl WindowManager for MacosManger {
    fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        let app_info = match &mut self.runner {
            MacosRunner::OnMainThread { script } => {
                // Execute compiled script
                let mut err: Option<_> = None;
                let data = unsafe { script.executeAndReturnError(err.as_mut()) };
                if let Some(err) = err {
                    return Err(anyhow!("execution error: {:?}", &err));
                }
                // dbg!("Script output: {:?}", &data);
                let json = unsafe {
                    data.ok_or_else(|| anyhow!("No result from OSAScript execution"))?
                        .stringValue()
                }
                .ok_or_else(|| anyhow!("Script did not return a string value"))?
                .to_string();

                // dbg!("Script output: {}", &json);
                // Parse JXA output
                let app_info: AppInfo = serde_json::from_str(&json)
                    .map_err(|e| anyhow!("Failed to parse JXA JSON: {e}; payload: {json}"))?;
                app_info
            }
            MacosRunner::SeparateProcess {
                current_app_info, ..
            } => {
                let app_info = current_app_info.lock().unwrap();
                let Some(app_info) = app_info.as_ref() else {
                    return Err(anyhow!("No app info was loaded"));
                };
                dbg!("App info: {:?}", app_info);
                app_info.clone()
            }
        };

        let pid = sysinfo::Pid::from_u32(app_info.unix_id);
        // Resolve process path via sysinfo
        // Refresh processes to ensure info is up to date
        self.sysinfo
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
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

enum MacosRunner {
    SeparateProcess {
        process: Child,
        _handle: thread::JoinHandle<Result<()>>,
        stop_signal: std::sync::mpsc::Sender<()>,
        current_app_info: Arc<Mutex<Option<AppInfo>>>,
    },
    OnMainThread {
        script: Retained<OSAScript>,
    },
}

fn create_on_main_thread_osascript_process() -> Result<MacosRunner> {
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

    Ok(MacosRunner::OnMainThread { script })
}

fn create_separate_osascript_process(collection_interval: Duration) -> Result<MacosRunner> {
    let current_app_info = Arc::new(Mutex::new(None));
    let inner_current_app_info = current_app_info.clone();

    #[allow(
        clippy::zombie_processes,
        reason = "Process is killed by the Drop impl"
    )]
    let mut process = Command::new("osascript")
        .stdout(Stdio::piped())
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(create_osascript_command(collection_interval))
        .spawn()
        .unwrap();

    let stdout = process.stdout.take().expect("Stdout was not piped");
    let (stop_signal, stop_signal_receiver) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let lines = BufReader::new(stdout).lines();
        for line in lines {
            if stop_signal_receiver.try_recv().is_ok() {
                return Ok(());
            }
            let line = line.unwrap();
            let app_info: AppInfo = serde_json::from_str(&line).unwrap();
            let mut current_app_info = inner_current_app_info.lock().unwrap();
            *current_app_info = Some(app_info);
        }
        Ok(())
    });
    Ok(MacosRunner::SeparateProcess {
        process,
        _handle: handle,
        stop_signal,
        current_app_info,
    })
}

impl Drop for MacosRunner {
    fn drop(&mut self) {
        match self {
            MacosRunner::SeparateProcess {
                process,
                stop_signal,
                ..
            } => {
                let _ = stop_signal.send(());
                let _ = process.kill();
            }
            MacosRunner::OnMainThread { .. } => {}
        }
    }
}

fn create_osascript_command(collection_interval: Duration) -> String {
    format!(
        r#"#!/usr/bin/osascript -l JavaScript

// adapted from:
// https://gist.github.com/EvanLovely/cb01eafb0d61515c835ecd56f6ac199a

// new to jxa?
// - https://apple-dev.groups.io/g/jxa/wiki/3202
// - interactive repl: `osascript -il JavaScript`
// - API reference: Script Editor -> File -> Open Dictionary

function getApp() {{
  var seApp = Application("System Events")
  var oProcess = seApp.processes.whose({{ frontmost: true }})[0]
  var appName = oProcess.displayedName()

  // as of 05/01/21 incognio & url are not actively used in AW
  // variables must be set to `undefined` since this script is re-run via osascript
  // and the previously set values will be cached otherwise
  var url = undefined,
    incognito = undefined,
    title = undefined

  // it's not possible to get the URL from firefox
  // https://stackoverflow.com/questions/17846948/does-firefox-offer-applescript-support-to-get-url-of-windows

  switch (appName) {{
    case "Safari":
      // incognito is not available via safari applescript
      url = Application(appName).documents[0].url()
      title = Application(appName).documents[0].name()
      break
    case "Google Chrome":
    case "Google Chrome Canary":
    case "Chromium":
    case "Brave Browser":
      const activeWindow = Application(appName).windows[0]
      const activeTab = activeWindow.activeTab()

      url = activeTab.url()
      title = activeTab.name()
      incognito = activeWindow.mode() === "incognito"
      break
    case "Firefox":
    case "Firefox Developer Edition":
      title = Application(appName).windows[0].name()
      break
    default:
      mainWindow = oProcess
        .windows()
        .find((w) => w.attributes.byName("AXMain").value() === true)

      // in some cases, the primary window of an application may not be found
      // this occurs rarely and seems to be triggered by switching to a different application
      if (mainWindow) {{
        title = mainWindow.attributes.byName("AXTitle").value()
      }}
  }}

  // key names must match expected names in lib.py
  return JSON.stringify({{
    app: appName,
    url,
    title,
    incognito,
  }})
}}

var seApp = Application("System Events")
function runCollector() {{
  while (true) {{
    console.log(getApp())
    delay({})
  }}
}}

runCollector()

    "#,
        collection_interval.as_secs()
    )
}
