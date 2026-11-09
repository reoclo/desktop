use std::path::PathBuf;
use std::process::Command;
pub mod worker;

/// How the hosts helper is installed: the port 443 relay and the suffixes beyond .localhost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelperSetup {
    pub relay_port: Option<u16>,
    pub allow_suffixes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemAction {
    StartDaemon,
    SetupHosts(HelperSetup),
    StartAtLogin(bool),
}

pub trait SystemRunner: Send + 'static {
    fn run(&self, action: &SystemAction) -> Result<String, String>;
}

pub struct MacRunner {
    pub cli: PathBuf,
    pub desktop: PathBuf,
    pub launch_agent: PathBuf,
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn launch_agent(binary: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>com.reoclo.desktop</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/></dict></plist>\n",
        xml(binary)
    )
}

/// `reoclo hosts setup` refuses to run as root, because it asks for sudo itself.
/// The administrator prompt is already root, so it runs the step that command would.
pub fn admin_script(binary: &str, uid: u32, setup: &HelperSetup) -> String {
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    let mut shell = format!("{} __hostsd-install --uid {uid}", quote(binary));
    for suffix in &setup.allow_suffixes {
        shell.push_str(&format!(" --allow-suffix {}", quote(suffix)));
    }
    if let Some(port) = setup.relay_port {
        shell.push_str(&format!(" --port-443 --relay-port {port}"));
    }
    format!(
        "do shell script \"{}\" with administrator privileges",
        shell.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// osascript reports failures as "0:121: execution error: <message> (<code>)".
pub fn admin_error(stderr: &str) -> String {
    let text = stderr.trim();
    if text.ends_with("(-128)") {
        return "Administrator approval was canceled.".into();
    }
    let body = text
        .split_once("execution error: ")
        .map_or(text, |(_, rest)| rest);
    match body.rsplit_once(" (") {
        Some((message, code)) if code.trim_end_matches(')').parse::<i32>().is_ok() => {
            message.trim().to_owned()
        }
        _ => body.to_owned(),
    }
}

fn current_uid() -> Result<u32, String> {
    let output = Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .map_err(|e| e.to_string())?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| "Could not read the current user id.".to_owned())
}

impl SystemRunner for MacRunner {
    fn run(&self, action: &SystemAction) -> Result<String, String> {
        let output = match action {
            SystemAction::StartDaemon => Command::new(&self.cli)
                .args(["tunnel", "daemon", "start"])
                .output(),
            SystemAction::SetupHosts(setup) => {
                let script = admin_script(&self.cli.to_string_lossy(), current_uid()?, setup);
                let output = Command::new("/usr/bin/osascript")
                    .args(["-e", &script])
                    .output()
                    .map_err(|e| e.to_string())?;
                return if output.status.success() {
                    Ok("Hosts helper installed.".into())
                } else {
                    Err(admin_error(&String::from_utf8_lossy(&output.stderr)))
                };
            }
            SystemAction::StartAtLogin(enabled) => {
                if *enabled {
                    if let Some(parent) = self.launch_agent.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(
                        &self.launch_agent,
                        launch_agent(&self.desktop.to_string_lossy()),
                    )
                    .map_err(|e| e.to_string())?;
                } else if self.launch_agent.exists() {
                    std::fs::remove_file(&self.launch_agent).map_err(|e| e.to_string())?;
                }
                return Ok(if *enabled {
                    "Reoclo will start at your next login."
                } else {
                    "Start at login is off."
                }
                .into());
            }
        }
        .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }
}

#[derive(Default)]
pub struct FakeRunner;
impl SystemRunner for FakeRunner {
    fn run(&self, action: &SystemAction) -> Result<String, String> {
        Ok(format!(
            "Development runner completed {action:?}; no system changes."
        ))
    }
}
