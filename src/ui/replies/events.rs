use super::*;

impl State {
    /// A daemon event arrived on the event stream.
    pub fn apply_event(&mut self, name: &str, data: &str) {
        if let Err(error) = self.model.apply(name, data) {
            self.error = Some(format!("Invalid daemon event: {error}"));
        }
        match name {
            "auth" => match self.model.auth.status.as_str() {
                "success" => {
                    self.phase = ViewState::Loaded;
                    self.error = None;
                    self.model.status.signed_in = Some(true);
                    self.link_signed_in();
                }
                "pending" => self.phase = ViewState::SignedOut,
                "failed" => {
                    self.phase = ViewState::SignedOut;
                    self.error = self.model.auth.message.clone();
                    self.link = None;
                }
                _ => {}
            },
            "tunnel" => self.link_check_tunnel(),
            _ => {}
        }
        self.sync_tray();
    }
}
