use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

const COMMAND: &str = r#"
var seApp = Application("System Events")
function hmhm() {
  while (true) {
    var oProcess = seApp.processes.whose({ frontmost: true })
    console.log(
      JSON.stringify({
        length: oProcess.length,
        name: oProcess.name(),
        id: oProcess.id(),
      })
    )
    delay(1)
  }
}

hmhm()
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
