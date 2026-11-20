use super::*;

impl State {
    pub(super) fn unique_tunnel_name(&self, base: &str) -> String {
        let taken = |name: &str| self.model.tunnels.iter().any(|tunnel| tunnel.name == name);
        if !taken(base) {
            return base.to_owned();
        }
        let mut number = 2u32;
        loop {
            let suffix = format!("-{number}");
            let mut stem = base.to_owned();
            stem.truncate(63 - suffix.len());
            let name = format!("{}{suffix}", stem.trim_end_matches('-'));
            if !taken(&name) {
                return name;
            }
            number += 1;
        }
    }

    /// Reads the tunnel the link is waiting on. Ready already means it is listening, and a
    /// session opens when the browser or client connects, so Ready and Active both count.
    pub(in crate::ui) fn link_check_tunnel(&mut self) {
        let Some(pending) = self.link.as_ref() else {
            return;
        };
        if pending.step != LinkStep::Reaching {
            return;
        }
        let kind = pending.link.kind;
        let Some(tunnel) = self
            .model
            .tunnels
            .iter()
            .find(|tunnel| tunnel.name == pending.tunnel)
            .cloned()
        else {
            return;
        };
        if tunnel.state.failure() {
            let reason = if tunnel.message.is_empty() {
                tunnel.state.label()
            } else {
                tunnel.message.as_str()
            };
            self.link_error(format!("{} could not be reached: {reason}.", tunnel.name));
            return;
        }
        if !matches!(tunnel.state, TunnelState::Ready | TunnelState::Active) {
            return;
        }
        self.link = None;
        self.model.selected = Some(tunnel.name.clone());
        self.table.select(&tunnel.name);
        match kind {
            LinkKind::Https => match tunnel.url {
                Some(url) => {
                    self.notice = Some(format!("Opening {}.", tunnel.name));
                    self.effects.push(Effect::OpenUrl(url));
                }
                None => {
                    self.error = Some(format!("{} is up but has no address yet.", tunnel.name));
                }
            },
            LinkKind::Tcp => {
                let endpoint = tunnel
                    .endpoint
                    .clone()
                    .or_else(|| tunnel.local_port.map(|port| format!("127.0.0.1:{port}")));
                match endpoint {
                    Some(endpoint) => {
                        self.notice =
                            Some(format!("Copied {endpoint}. Connect to it from this Mac."));
                        self.effects.push(Effect::Copy(endpoint));
                    }
                    None => {
                        self.error =
                            Some(format!("{} is up but has no local port yet.", tunnel.name));
                    }
                }
            }
        }
    }

    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    fn flush_effects(&mut self, tx: &mut rune::Tx) -> bool {
        let effects = self.take_effects();
        let any = !effects.is_empty();
        for effect in effects {
            match effect {
                Effect::OpenUrl(url) => {
                    if let Err(error) = tx.open_url(&url) {
                        self.error = Some(error.to_string());
                    }
                }
                Effect::Copy(text) => tx.copy(&text),
            }
        }
        any
    }

    /// A step that never hears back drops the link, so a later event cannot open it unasked.
    fn expire_link(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        let Some(pending) = self.link.as_mut() else {
            return false;
        };
        let limit = match pending.step {
            LinkStep::Confirm => return false,
            LinkStep::Reaching => REACH_SECONDS,
            _ => SIGN_IN_SECONDS,
        };
        match pending.deadline {
            None => {
                pending.deadline = Some(clock + limit);
                tx.wake_at(clock + limit);
                false
            }
            Some(due) if clock >= due => {
                self.link_error("The link did not finish. Open it again to retry.".into());
                true
            }
            Some(due) => {
                tx.wake_at(due);
                false
            }
        }
    }

    /// Runs before every frame. Returns true when something changed and the view must redraw.
    pub fn poll(&mut self, clock: f64, tx: &mut rune::Tx) -> bool {
        self.poll_updates(clock, tx);
        let watching = self.page == super::super::Page::Tunnels
            && self.current_data()
            && self.model.history.has_points();
        let interval = if watching { 1.0 } else { 60.0 };
        let clock_changed = self
            .clock_seconds
            .is_some_and(|last| clock - last >= interval);
        if self.clock_seconds.is_none() || clock_changed {
            self.clock_seconds = Some(clock);
        }
        if watching {
            let now = self.clock_epoch + time::Duration::seconds_f64(clock);
            self.model.history.prune(now.unix_timestamp());
        }
        if !self.model.approvals.is_empty() || watching {
            tx.wake_at(self.clock_seconds.unwrap_or(clock) + interval);
        }
        let notice = self.expire_notice(clock, tx);
        let link = self.expire_link(clock, tx);
        let daemon = self.expire_daemon_wait(clock, tx);
        let agents = self.first_run_tick();
        let effects = self.flush_effects(tx);
        notice
            || link
            || daemon
            || agents
            || effects
            || (clock_changed && (watching || !self.model.approvals.is_empty()))
    }
}
