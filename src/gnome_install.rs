use std::{path::Path, process::Command};

use anyhow::{Context as _, Result};

pub fn install_gnome_extension(path: &Path) -> Result<()> {
    Command::new("gnome-extensions")
        .arg("install")
        .arg(path)
        .status()
        .with_context(|| "Failed to install gnome extension")?;

    Ok(())
}

const EXTENSION_UUID: &str = "focused-window-dbus@whatawhat.anoromi.com";

pub fn activate_gnome_extension() -> Result<()> {
    Command::new("gnome-extensions")
        .arg("enable")
        .arg(EXTENSION_UUID)
        .status()
        .with_context(|| "Failed to activate gnome extension")?;

    Ok(())
}