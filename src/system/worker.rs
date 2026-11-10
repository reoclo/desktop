use super::{SystemAction, SystemRunner};
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
                handle.update(move |s, _| {
                    s.busy = false;
                    match result {
                        Ok(message) => {
                            s.modal = None;
                            if let SystemAction::StartAtLogin(enabled) = action {
                                s.start_at_login = enabled;
                            }
                            if action == SystemAction::InstallDaemon {
                                s.service_installed_ok();
                            }
                            if matches!(action, SystemAction::SetupHosts(_)) {
                                // The helper's own answer decides what the app reports.
                                s.request("POST", "/v1/hosts/sync", None);
                                s.awaiting_helper = true;
                            } else {
                                s.request("GET", "/v1/status", None);
                                s.notice = Some(message);
                            }
                        }
                        Err(message) => {
                            s.error = Some(message);
                            s.system_failed(&action);
                        }
                    }
                });
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
            for handle in &handles {
                let found = found.clone();
                handle.update(move |s, _| s.cli = Some(found));
            }
        }
    });
    sender
}
