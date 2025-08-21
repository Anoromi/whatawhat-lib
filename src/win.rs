//! Contains logic for extracting records through x11. The implementation uses xcb for communication
//! with the server.

use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use crate::{simple_cache::CacheConfig, utils::default_cache_config, windows_desktop::{WindowsAppInfo, WindowsDesktopInfo}};
use tracing::error;
use windows::{
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, HWND},
        System::{
            Diagnostics::Debug::{
                FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS, FormatMessageW,
            },
            SystemInformation::GetTickCount64,
            SystemServices::{LANG_ENGLISH, SUBLANG_ENGLISH_US},
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
                QueryFullProcessImageNameW,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
            WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId},
        },
    },
    core::PWSTR,
};

use super::{ActiveWindowData, WindowManager};

unsafe fn get_window_process_path(window_handle: HANDLE, text: &mut [u16]) -> Result<String> {
    let mut length = text.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            window_handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(text.as_mut_ptr()),
            &mut length,
        )?;
    }
    Ok(String::from_utf16_lossy(&text[..length as usize]))
}

unsafe fn get_window_title(window_handle: HWND, text: &mut [u16]) -> String {
    let len = unsafe { GetWindowTextW(window_handle, text) };
    String::from_utf16_lossy(&text[..len as usize])
}

pub struct WindowsWindowManager {
    idle_timeout: Duration,
    desktop_info_cache: crate::simple_cache::SimpleCache<String, WindowsAppInfo>,
    windows_desktop_info: WindowsDesktopInfo,
}

impl WindowsWindowManager {
    pub fn new(idle_timeout: Duration, cache_config: Option<CacheConfig>) -> Self {
        Self {
            idle_timeout,
            desktop_info_cache: crate::simple_cache::SimpleCache::new(
                cache_config.unwrap_or(default_cache_config()),
            ),
            windows_desktop_info: WindowsDesktopInfo::new(),
        }
    }
}

#[tracing::instrument]
async fn get_active_windows_data(
    desktop_info_cache: &mut crate::simple_cache::SimpleCache<String, WindowsAppInfo>,
    windows_desktop_info: &WindowsDesktopInfo,
) -> Result<ActiveWindowData> {
    let (process_path, title) = {
        let window = unsafe { GetForegroundWindow() };

        if window.is_invalid() {
            return Err(anyhow!("Failed to get foreground window"));
        }

        let mut id = 0u32;
        unsafe { GetWindowThreadProcessId(window, Some(&mut id)) };
        if id == 0 {
            let err = unsafe { GetLastError() };
            let mut message_buffer = [0u16; 2048];
            let size = unsafe {
                // Gets a message from windows about previous error in english
                FormatMessageW(
                    FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                    None,
                    err.0,
                    LANG_ENGLISH | (SUBLANG_ENGLISH_US << 10),
                    PWSTR::from_raw(message_buffer.as_mut_ptr()),
                    2048,
                    None,
                )
            };
            if size == 0 {
                return Err(anyhow!("Failed to get active window"));
            } else {
                let data = String::from_utf16(&message_buffer[0..size as usize])
                    .expect("Failed to unwrap");
                return Err(anyhow!("Failed to get active window {data}"));
            }
        }
        let process_handle =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, id) }
                .inspect_err(|e| error!("Failed to open process {e:?}"))?;

        let mut text: [u16; 4096] = [0; 4096];
        let process_path = unsafe { get_window_process_path(process_handle, &mut text) }
            .inspect_err(|e| error!("Failed to get window process path {e:?}"))?;
        let title = unsafe { get_window_title(window, &mut text) };

        unsafe { CloseHandle(process_handle) }
            .inspect_err(|e| error!("Failed to close handle {e:?}"))?;
        (process_path, title)
    };
    // Resolve app_name via cache and PE version info
    let app_name = match desktop_info_cache.get(&process_path) {
        Some(info) => Some(info.app_name),
        None => {
            if let Some(info) = windows_desktop_info.get_extra_info(&process_path) {
                desktop_info_cache.set(process_path.clone(), info.clone());
                Some(info.app_name)
            } else {
                None
            }
        }
    };

    Ok(ActiveWindowData {
        window_title: title.into(),
        app_identifier: Some(process_path.clone().into()),
        process_path: Some(process_path.into()),
        app_name,
    })
}

pub fn get_idle_time() -> Result<u64> {
    let mut last: LASTINPUTINFO = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    let is_success = unsafe { GetLastInputInfo(&mut last) };
    if !is_success.as_bool() {
        error!("Failed to retrieve user idle time");
        return Err(anyhow!("Failed to retrieve user idle time"));
    }

    let tick_count = unsafe { GetTickCount64() };
    let duration = tick_count - last.dwTime as u64;
    if duration > u32::MAX as u64 {
        Ok(u32::MAX as u64)
    } else {
        Ok(duration)
    }
}

#[async_trait]
impl WindowManager for WindowsWindowManager {
    async fn get_active_window_data(&mut self) -> Result<ActiveWindowData> {
        get_active_windows_data(&mut self.desktop_info_cache, &self.windows_desktop_info)
            .await
            .inspect_err(|e| error!("Failed to get active window {e:?}"))
    }

    async fn is_idle(&mut self) -> Result<bool> {
        let idle_time = get_idle_time().inspect_err(|e| error!("Failed to get idle time {e:?}"))?;
        Ok(idle_time > self.idle_timeout.as_millis() as u64)
    }
}
