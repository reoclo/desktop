use super::State;
use crate::update::{Phase, worker::Command};

impl State {
    pub fn update_command(&mut self, command: Command) {
        if let Some(sender) = &self.updater
            && sender.send(command).is_err()
        {
            self.updates.phase = Phase::Failed(crate::update::Error::WorkerStopped.to_string());
        }
    }

    pub fn toggle_updates(&mut self) {
        self.updates.automatic = !self.updates.automatic;
        self.save_preferences();
        self.update_command(Command::Automatic(self.updates.automatic));
    }

    /// The first poll belongs to the first frame. Start the worker on a later frame.
    pub fn poll_updates(&mut self, clock: f64, tx: &mut rune::Tx) {
        if self.panel || self.updater_started || self.updater.is_none() {
            return;
        }
        match self.updater_first_frame {
            None => {
                self.updater_first_frame = Some(clock);
                tx.wake_at(clock + 0.001);
            }
            Some(first) if clock > first => {
                self.updater_started = true;
                self.update_command(Command::Start);
            }
            Some(first) => tx.wake_at(first + 0.001),
        }
    }
}
