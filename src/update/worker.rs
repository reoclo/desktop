use super::{
    Client, INTERVAL, LATEST_URL, Phase, Status,
    install::{Bundle, Prepared},
    public_key,
};
use crate::ui::State;
use rune::AppHandle;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Sender},
    time::Instant,
};

#[derive(Clone, Debug)]
pub enum Command {
    Start,
    Check,
    Automatic(bool),
    Restart,
}

pub fn start(
    executable: PathBuf,
    handles: Vec<AppHandle<State>>,
    automatic: bool,
) -> Sender<Command> {
    let (send, receive) = mpsc::channel();
    let demo = std::env::args_os().any(|a| a == "--demo");
    std::thread::spawn(move || {
        let mut status = Status {
            automatic,
            ..Default::default()
        };
        let target = Bundle::discover(&executable);
        let client = public_key().and_then(|key| {
            Client::new(
                std::env::var("REOCLO_UPDATE_URL").unwrap_or_else(|_| LATEST_URL.into()),
                key,
            )
        });
        let (target, client) = match (target, client) {
            (Ok(target), Ok(client)) => (target, client),
            (Err(error), _) | (_, Err(error)) => {
                status.phase = Phase::Disabled(error.to_string());
                publish(&handles, &status);
                return;
            }
        };
        status.phase = Phase::Idle;
        publish(&handles, &status);
        let mut started = false;
        let mut due = Instant::now();
        let mut prepared: Option<Prepared> = None;
        loop {
            let timed = started && status.automatic && prepared.is_none();
            let command = if timed {
                match receive.recv_timeout(due.saturating_duration_since(Instant::now())) {
                    Ok(command) => command,
                    Err(mpsc::RecvTimeoutError::Timeout) => Command::Check,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match receive.recv() {
                    Ok(command) => command,
                    Err(_) => break,
                }
            };
            match command {
                Command::Start => {
                    started = true;
                }
                Command::Automatic(enabled) => {
                    status.automatic = enabled;
                    publish(&handles, &status);
                }
                Command::Check => {
                    if prepared.is_some() {
                        continue;
                    }
                    status.phase = Phase::Checking;
                    publish(&handles, &status);
                    let result = client
                        .check(env!("CARGO_PKG_VERSION"))
                        .and_then(|manifest| {
                            let Some(manifest) = manifest else {
                                return Ok(None);
                            };
                            status.phase = Phase::Available {
                                version: manifest.version.to_string(),
                            };
                            publish(&handles, &status);
                            target.prepare(client.download(manifest)?).map(Some)
                        });
                    let checked = time::OffsetDateTime::now_utc();
                    status.last_checked = Some(format!(
                        "{} {:02}:{:02} UTC",
                        checked.date(),
                        checked.hour(),
                        checked.minute()
                    ));
                    match result {
                        Ok(Some(update)) => {
                            status.phase = Phase::Ready {
                                version: update.manifest.version.to_string(),
                                notes: update.manifest.notes.clone(),
                            };
                            prepared = Some(update);
                        }
                        Ok(None) => status.phase = Phase::Idle,
                        Err(error) => status.phase = Phase::Failed(error.to_string()),
                    }
                    due = Instant::now() + INTERVAL;
                    publish(&handles, &status);
                }
                Command::Restart => {
                    if let Some(update) = prepared.take() {
                        status.phase = Phase::Installing;
                        publish(&handles, &status);
                        match super::restart::handoff(update, &target, demo) {
                            Ok(()) => {
                                for handle in &handles {
                                    handle.update(|_, tx| tx.quit());
                                }
                                break;
                            }
                            Err(error) => {
                                status.phase = Phase::Failed(error.to_string());
                                publish(&handles, &status);
                            }
                        }
                    }
                }
            }
        }
    });
    send
}

fn publish(handles: &[AppHandle<State>], status: &Status) {
    for handle in handles {
        let status = status.clone();
        handle.update(move |s, _| {
            let changed = s.updates.automatic != status.automatic;
            s.updates = status;
            if changed {
                s.save_preferences();
            }
        });
    }
}
