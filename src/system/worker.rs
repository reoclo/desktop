use super::{SystemAction, SystemRunner};
use crate::ui::State;
use rune::AppHandle;
use std::sync::mpsc::{self, Sender};

pub fn start(
    runner: Box<dyn SystemRunner>,
    handles: Vec<AppHandle<State>>,
) -> Sender<SystemAction> {
    let (sender, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for action in receive {
            let result = runner.run(&action);
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
