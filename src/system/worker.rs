use super::{
    SystemAction, SystemRunner,
    command_path::{self, CommandPaths},
};
use crate::{cli::Locator, ui::State};
use rune::AppHandle;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

pub fn start(
    runner: Box<dyn SystemRunner>,
    handles: Vec<AppHandle<State>>,
) -> Sender<(SystemAction, Option<PathBuf>)> {
    let (sender, receive) = mpsc::channel::<(SystemAction, Option<PathBuf>)>();
    std::thread::spawn(move || {
        for (action, cli) in receive {
            let result = runner.run(&action, cli.as_deref());
            for handle in &handles {
                let result = result.clone();
                let action = action.clone();
                handle.update(move |s, _| s.system_finished(&action, result));
            }
        }
    });
    sender
}

/// Looks for the CLI off the main thread, since each candidate runs `reoclo --version`.
/// Each message is the executable the daemon reports.
pub fn find_cli(locator: Locator, handles: Vec<AppHandle<State>>) -> Sender<Option<String>> {
    let (sender, receive) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        for daemon in receive {
            let found = locator.locate(daemon.as_deref(), crate::cli::version_of);
            let ca = match &found {
                crate::cli::Cli::Ready { path, .. } => std::process::Command::new(path)
                    .args(["tunnel", "ca-path"])
                    .output()
                    .map_err(|error| error.to_string())
                    .and_then(|output| {
                        if output.status.success() {
                            Ok(PathBuf::from(
                                String::from_utf8_lossy(&output.stdout).trim(),
                            ))
                        } else {
                            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                        }
                    }),
                _ => Err("A current reoclo CLI is needed to locate the CA file.".into()),
            };
            for handle in &handles {
                let found = found.clone();
                let ca = ca.clone();
                handle.update(move |s, _| {
                    s.cli = Some(found);
                    match ca {
                        Ok(path) => {
                            s.ca_path = Some(path);
                            s.ca_path_error = None;
                        }
                        Err(error) => {
                            s.ca_path = None;
                            s.ca_path_error = Some(error);
                        }
                    }
                });
            }
        }
    });
    sender
}

/// What the Tools step asks of the agent command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRequest {
    Install,
    Remove,
}

/// Links or unlinks the agent command off the main thread, since the shell lookup starts a login shell.
pub fn start_command(
    paths: CommandPaths,
    handles: Vec<AppHandle<State>>,
) -> Sender<CommandRequest> {
    let (sender, receive) = mpsc::channel::<CommandRequest>();
    std::thread::spawn(move || {
        for request in receive {
            let state = match request {
                CommandRequest::Install => command_path::install(
                    &paths,
                    || command_path::lookup(&paths.home),
                    crate::cli::version_of,
                ),
                CommandRequest::Remove => command_path::remove(&paths),
            };
            for handle in &handles {
                let state = state.clone();
                handle.update(move |s, _| s.command_finished(state));
            }
        }
    });
    sender
}
