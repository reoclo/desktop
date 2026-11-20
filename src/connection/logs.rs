use crate::{logs::LogTail, ui::State};
use rune::AppHandle;
use std::{
    path::Path,
    sync::mpsc::{self, Sender},
    time::Duration,
};

pub(super) fn start(socket: &Path, handles: Vec<AppHandle<State>>) -> Sender<()> {
    let path = socket
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("tunneld.log"));
    let (stop, stopped) = mpsc::channel();
    std::thread::spawn(move || {
        let Some(path) = path else {
            return;
        };
        let mut tail = LogTail::new(path);
        let mut last_error = None;
        loop {
            match tail.read() {
                Ok(entries) => {
                    if !entries.is_empty() || last_error.is_some() {
                        for handle in &handles {
                            let entries = entries.clone();
                            handle.update(move |s, _| {
                                s.model.log_error = None;
                                for entry in entries {
                                    s.model.append_log(entry);
                                }
                            });
                        }
                    }
                    last_error = None;
                }
                Err(error) => {
                    let message = error.to_string();
                    if last_error.as_ref() != Some(&message) {
                        for handle in &handles {
                            let message = message.clone();
                            handle.update(move |s, _| s.model.log_error = Some(message));
                        }
                    }
                    last_error = Some(message);
                }
            }
            match stopped.recv_timeout(Duration::from_secs(1)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    });
    stop
}
