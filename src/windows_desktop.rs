use std::{path::PathBuf, str::FromStr, sync::Arc};

use tracing::warn;

#[derive(Clone, Debug)]
pub struct WindowsAppInfo {
    pub app_name: Arc<str>,
}

#[derive(Debug)]
pub struct WindowsDesktopInfo;

impl WindowsDesktopInfo {
    pub fn new() -> Self {
        Self
    }

    pub fn get_extra_info(&self, process_path: &str) -> Option<WindowsAppInfo> {
        let path = PathBuf::from_str(process_path).ok()?;
        let file_map = match pelite::FileMap::open(&path) {
            Ok(map) => map,
            Err(e) => {
                warn!(
                    "Failed to map file for version info: {}: {}",
                    process_path, e
                );
                return None;
            }
        };
        let image = match pelite::PeFile::from_bytes(file_map.as_ref()) {
            Ok(img) => img,
            Err(e) => {
                warn!("File is not a PE image for {}: {}", process_path, e);
                return None;
            }
        };
        let resources = match image.resources() {
            Ok(r) => r,
            Err(e) => {
                warn!("Resources not found for {}: {}", process_path, e);
                return None;
            }
        };
        let info = match resources.version_info() {
            Ok(v) => v,
            Err(e) => {
                warn!("Version info not found for {}: {}", process_path, e);
                return None;
            }
        };

        let mut product_name: Option<Arc<str>> = None;
        for lang in info.translation() {
            info.strings(*lang, |key, value| {
                println!("key: {}, value: {}", key, value);
                if key == "ProductName" && product_name.is_none() {
                    product_name = Some(Arc::from(value));
                }
            });
        }

        product_name.map(|app_name| WindowsAppInfo { app_name })
    }
}
