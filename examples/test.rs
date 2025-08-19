use freedesktop_desktop_entry::unicase::Ascii;
use gio::{prelude::FileExt, FileQueryInfoFlags};


fn main() {
    let locales = &["en_US".to_string()];
    // dbg!(freedesktop_desktop_entry::default_paths().collect::<Vec<_>>());
    let hm = freedesktop_desktop_entry::desktop_entries(locales);
    // let entry = freedesktop_desktop_entry::find_app_by_id(&hm,  Ascii::new("nvim")).unwrap();
    // dbg!(entry);
    // serde_json::to_string_pretty(&hm).unwrap();
    systemicons::init();

    for entry in hm {
        println!("{}", entry.appid);
        println!("{:?}", entry.icon());
        println!("{:?}", entry.name(locales));
        println!("{:?}", entry.path);
        if let Some(icon) = entry.icon() {

            let file = gio::File::for_path(&entry.path);
            let info = file.query_info("standard::icon", FileQueryInfoFlags::NONE, None::<&gio::Cancellable>).unwrap();
            let icon = info.icon().unwrap();

            println!("{:?}", info);
            // let icon = systemicons::get_icon(&entry.appid, 32);
            println!("{:?}", icon);
        }
        println!("{:?}", entry.parse_exec());
        println!("--------------------------------");
    }
}