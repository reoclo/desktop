use std::path::{Path, PathBuf};
use std::process::Command;
pub mod command_path;
pub mod worker;

/// How the hosts helper is installed: the port 443 relay and the suffixes beyond .localhost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelperSetup {
    pub relay_port: Option<u16>,
    pub allow_suffixes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemAction {
    /// `tunnel daemon install`: starts the daemon now and at every login.
    InstallDaemon,
    /// Stop the daemon and start it again on the CLI the app found, under launchd when the service is installed.
    RestartDaemon,
    /// `__hostsd-install` copies the CLI to the root-owned `/Library/PrivilegedHelperTools/com.reoclo.hostsd`,
    /// so the helper never points into the app bundle. Moving or updating the app cannot break it; it only
    /// keeps running the copied version until `helper_behind` offers this reinstall.
    SetupHosts(HelperSetup),
    StartAtLogin(bool),
    ShowLog(PathBuf),
    /// `<program> mcp agents -o json`: which coding agents are on this Mac. Runs the app's own copy.
    ListAgents {
        program: PathBuf,
    },
    /// `<program> mcp install --agent all --command <command>`: registers Reoclo's MCP server with them.
    ConnectAgents {
        program: PathBuf,
        command: PathBuf,
    },
}

pub trait SystemRunner: Send + 'static {
    /// `cli` is the reoclo CLI to run. Start at login needs none.
    fn run(&self, action: &SystemAction, cli: Option<&Path>) -> Result<String, String>;
}

pub struct MacRunner {
    pub desktop: PathBuf,
    pub launch_agent: PathBuf,
    /// The daemon's own launch agent, present when the service is installed.
    pub service_plist: PathBuf,
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

/// `mcp install` exits with an error after printing its results when one agent failed, so the
/// answer is whatever it printed, and the error output only when it printed nothing.
fn json_answer(output: std::io::Result<std::process::Output>) -> Result<String, String> {
    let output = output.map_err(|e| e.to_string())?;
    let printed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !printed.is_empty() {
        return Ok(printed);
    }
    let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if error.is_empty() {
        "The reoclo command did not answer.".into()
    } else {
        error
    })
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
    fn run(&self, action: &SystemAction, cli: Option<&Path>) -> Result<String, String> {
        let cli = || cli.ok_or_else(|| "No reoclo CLI was found to run.".to_owned());
        let output = match action {
            SystemAction::ShowLog(path) => {
                Command::new("/usr/bin/open").arg("-R").arg(path).output()
            }
            SystemAction::InstallDaemon => Command::new(cli()?)
                .args(["tunnel", "daemon", "install"])
                .output(),
            SystemAction::ListAgents { program } => {
                return json_answer(
                    Command::new(program)
                        .args(["mcp", "agents", "-o", "json"])
                        .output(),
                );
            }
            SystemAction::ConnectAgents { program, command } => {
                return json_answer(
                    Command::new(program)
                        .args(["mcp", "install", "--agent", "all", "--command"])
                        .arg(command)
                        .args(["-o", "json"])
                        .output(),
                );
            }
            SystemAction::RestartDaemon => {
                let stop = Command::new(cli()?)
                    .args(["tunnel", "daemon", "stop"])
                    .output()
                    .map_err(|e| e.to_string())?;
                if !stop.status.success() {
                    return Err(String::from_utf8_lossy(&stop.stderr).trim().to_owned());
                }
                let start = if self.service_plist.exists() {
                    "install"
                } else {
                    "start"
                };
                let output = Command::new(cli()?)
                    .args(["tunnel", "daemon", start])
                    .output()
                    .map_err(|e| e.to_string())?;
                return if output.status.success() {
                    Ok("Daemon restarted.".into())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                };
            }
            SystemAction::SetupHosts(setup) => {
                let script = admin_script(&cli()?.to_string_lossy(), current_uid()?, setup);
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

const DEMO_AGENTS: &str = r#"{"agents":[{"id":"claude-code","name":"Claude Code","installed":true,"config_path":"~/.claude.json","registered":false,"command":null,"error":null},{"id":"codex","name":"Codex","installed":true,"config_path":"~/.codex/config.toml","registered":false,"command":null,"error":null}]}"#;

fn demo_connected(command: &Path) -> String {
    let change = |id: &str, name: &str, path: &str| serde_json::json!({"agent": id, "name": name, "action": "installed", "config_path": path, "backup_path": null, "command": command, "error": null});
    serde_json::json!({"results": [
        change("claude-code", "Claude Code", "~/.claude.json"),
        change("codex", "Codex", "~/.codex/config.toml"),
    ]})
    .to_string()
}

#[derive(Default)]
pub struct FakeRunner;
impl SystemRunner for FakeRunner {
    fn run(&self, action: &SystemAction, _cli: Option<&Path>) -> Result<String, String> {
        match action {
            SystemAction::ListAgents { .. } => Ok(DEMO_AGENTS.into()),
            SystemAction::ConnectAgents { command, .. } => Ok(demo_connected(command)),
            _ => Ok(format!(
                "Development runner completed {action:?}; no system changes."
            )),
        }
    }
}
