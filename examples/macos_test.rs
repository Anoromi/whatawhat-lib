use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

const COMMAND: &str = r#"
function getApp() {
  var oProcess = seApp.processes.whose({ frontmost: true })
  var appName = oProcess.displayedName()
  var unixId = oProcess.unixId()
  // as of 05/01/21 incognio & url are not actively used in AW
  // variables must be set to `undefined` since this script is re-run via osascript
  // and the previously set values will be cached otherwise
  var title = undefined

  // it's not possible to get the URL from firefox
  // https://stackoverflow.com/questions/17846948/does-firefox-offer-applescript-support-to-get-url-of-windows

  switch (appName) {
    case "Safari":
      title = Application(appName).documents[0].name()
      break
    case "Google Chrome":
    case "Google Chrome Canary":
    case "Chromium":
    case "Brave Browser":
      const activeWindow = Application(appName).windows[0]
      const activeTab = activeWindow.activeTab()

      title = activeTab.name()
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
      if (mainWindow) {
        title = mainWindow.attributes.byName("AXTitle").value()
      }
  }

  // key names must match expected names in lib.py
  return JSON.stringify({
    unixId,
    app: appName,
    title,
  })
}

var seApp = Application("System Events")
function runCollector() {
  while (true) {
    console.log(getApp())
    delay(1)
  }
}

runCollector()

"#;
fn main() {
    let mut command = Command::new("osascript")
        .stdout(Stdio::piped())
        .arg("-e")
        .arg(COMMAND)
        .arg("-l")
        .arg("JavaScript")
        .spawn()
        .unwrap();

    let out = command.stdout.take().unwrap();
    let buf_reader = BufReader::new(out);
    for line in buf_reader.lines() {
        println!("{}", line.unwrap());
    }
    command.wait().unwrap();
}
