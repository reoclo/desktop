use super::{State, setting};
use crate::{
    theme::*,
    ui::primitives::{Rank, Tone, indicator},
    update::{Phase, worker::Command},
};
use rune::{El, col, row, text};
use rune_kit::{Button, Kit, Switch};

pub fn view(s: &State, k: &Kit) -> El {
    let (tone, message, can_check) = match &s.updates.phase {
        Phase::Disabled(reason) => (Tone::Inactive, reason.clone(), false),
        Phase::Idle => (
            Tone::Success,
            if s.updates.last_checked.is_some() {
                "Reoclo is up to date.".into()
            } else {
                "Ready to check for updates.".into()
            },
            true,
        ),
        Phase::Checking => (Tone::Info, "Checking for updates.".into(), false),
        Phase::Available { version } => {
            (Tone::Info, format!("Downloading Reoclo {version}."), false)
        }
        Phase::Ready { version, .. } => {
            (Tone::Success, format!("Reoclo {version} is ready."), false)
        }
        Phase::Installing => (Tone::Info, "Preparing to restart.".into(), false),
        Phase::Failed(reason) => (Tone::Warning, reason.clone(), true),
    };
    let mut status = col()
        .w_full()
        .gap(SM)
        .child(
            text("Updates")
                .role("heading")
                .font(HEADING)
                .color(k.c.text_base),
        )
        .child(
            row()
                .w_full()
                .center_y()
                .gap(LG)
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(XS)
                        .child(
                            text(format!("Reoclo {}", env!("CARGO_PKG_VERSION")))
                                .font(MONO)
                                .color(k.c.text_base),
                        )
                        .child(
                            text(match &s.updates.last_checked {
                                Some(checked) => format!("Last checked {checked}"),
                                None => "Not checked yet.".into(),
                            })
                            .font(HINT)
                            .color(k.c.text_muted)
                            .wrap(),
                        ),
                )
                .child(match &s.updates.phase {
                    Phase::Ready { .. } => Button::new("Restart to update")
                        .on_press(|s: &mut State| s.update_command(Command::Restart))
                        .primary(k)
                        .key("restart-update"),
                    Phase::Disabled(_)
                    | Phase::Checking
                    | Phase::Available { .. }
                    | Phase::Installing => rune::el(),
                    Phase::Idle | Phase::Failed(_) => Button::new("Check now")
                        .disabled(!can_check)
                        .on_press(|s: &mut State| s.update_command(Command::Check))
                        .secondary(k)
                        .key("check-update"),
                }),
        );
    // Errors stay in Settings. The panel only offers verified, ready updates.
    status = status.child(
        row()
            .w_full()
            .gap(SM)
            .child(indicator(
                tone,
                match tone {
                    Tone::Warning => "Update failed",
                    Tone::Inactive => "Updates off",
                    Tone::Success => match &s.updates.phase {
                        Phase::Ready { .. } => "Verified",
                        Phase::Disabled(_)
                        | Phase::Idle
                        | Phase::Checking
                        | Phase::Available { .. }
                        | Phase::Installing
                        | Phase::Failed(_) => "Current",
                    },
                    Tone::Info => "Update status",
                    Tone::Danger | Tone::Ready => "Update status",
                },
                k,
            ))
            .child(
                text(message)
                    .font(HINT)
                    .color(k.c.text_muted)
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0),
            )
            .role("status"),
    );
    if let Phase::Ready { notes, .. } = &s.updates.phase {
        status = status.child(text(notes).font(HINT).color(k.c.text_muted).wrap());
    }
    status
        .child(setting(
            "Automatic checks",
            Some("Check at launch and every 6 hours. Restart when you choose."),
            Switch::new(s.updates.automatic)
                .label("Automatically check for updates")
                .hide_label()
                .disabled(matches!(s.updates.phase, Phase::Disabled(_)))
                .on_toggle(State::toggle_updates)
                .build(k)
                .key("automatic-updates"),
            k,
        ))
        .key("updates-section")
}

pub fn panel(s: &State, k: &Kit) -> Option<El> {
    match &s.updates.phase {
        Phase::Ready { version, .. } => Some(
            row()
                .key("panel-update")
                .w_full()
                .gap(SM)
                .center_y()
                .child(indicator(Tone::Success, &format!("{version} ready"), k).grow(1.0))
                .child(
                    Button::new("Restart")
                        .help("Restart to update Reoclo. Tunnels keep running.")
                        .on_press(|s: &mut State| s.update_command(Command::Restart))
                        .primary(k)
                        .key("restart-update"),
                ),
        ),
        Phase::Disabled(_)
        | Phase::Idle
        | Phase::Checking
        | Phase::Available { .. }
        | Phase::Installing
        | Phase::Failed(_) => None,
    }
}
