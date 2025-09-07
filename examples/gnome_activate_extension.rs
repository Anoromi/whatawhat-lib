use std::{path::PathBuf, str::FromStr};

use whatawhat_lib::gnome_install;

fn main() {
    let _path = PathBuf::from_str("./focused-window-dbus@whatawhat.anoromi.com.shell-extension.zip")
        .unwrap();
    gnome_install::activate_gnome_extension().unwrap();
}
