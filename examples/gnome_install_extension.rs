use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use whatawhat_lib::gnome_install;

fn main() {
    let path = PathBuf::from_str("./focused-window-dbus@whatawhat.anoromi.com.shell-extension.zip")
        .unwrap();
    gnome_install::install_gnome_extension(
        PathBuf::from_str("./focused-window-dbus@whatawhat.anoromi.com.shell-extension.zip")
            .unwrap()
            .as_path(),
    )
    .unwrap();
}
