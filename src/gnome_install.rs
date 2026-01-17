use std::{path::Path, process::Command};

use crate::{Error, Result};

pub fn install_gnome_extension(path: &Path) -> Result<()> {
    Command::new("gnome-extensions")
        .arg("install")
        .arg(path)
        .status()
        .map_err(|_| Error::gnome_extension_install_failed())?;

    Ok(())
}

const EXTENSION_UUID: &str = "focused-window-dbus@whatawhat.anoromi.com";

pub fn activate_gnome_extension() -> Result<()> {
    Command::new("gnome-extensions")
        .arg("enable")
        .arg(EXTENSION_UUID)
        .status()
        .map_err(|_| Error::gnome_extension_activate_failed())?;

    Ok(())
}
