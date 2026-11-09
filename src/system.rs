use std::path::PathBuf;
use std::process::Command;
pub mod worker;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemAction {
    StartDaemon,
    SetupHosts,
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

pub fn admin_script(binary: &str) -> String {
    let shell = format!("'{}' hosts setup", binary.replace('\'', "'\\''"));
    format!(
        "do shell script \"{}\" with administrator privileges",
        shell.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

impl SystemRunner for MacRunner {
    fn run(&self, action: &SystemAction) -> Result<String, String> {
        let output = match action {
            SystemAction::StartDaemon => Command::new(&self.cli)
                .args(["tunnel", "daemon", "start"])
                .output(),
            SystemAction::SetupHosts => Command::new("/usr/bin/osascript")
                .args(["-e", &admin_script(&self.cli.to_string_lossy())])
                .output(),
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
