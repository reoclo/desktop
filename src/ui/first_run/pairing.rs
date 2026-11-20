use super::*;
use crate::ui::{SetupStep, StepState};
use rune_kit::Button;

fn pair_row(name: &str, copy: &str, (tone, label): (Tone, &str), k: &Kit) -> El {
    row()
        .w_full()
        .gap(LG)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(XS)
                .child(text(name).font(k.ty.body_medium).color(k.c.text_base))
                .child(text(copy).font(BODY).color(k.c.text_muted).wrap()),
        )
        .child(indicator(tone, label, k))
}

/// How one pairing step reads: done, working, needing another try, next, or waiting its turn.
fn pair_state(step: StepState, current: bool) -> (Tone, &'static str) {
    match step {
        StepState::Done => (Tone::Success, "Done"),
        StepState::Running => (Tone::Info, "Working"),
        StepState::Failed(_) => (Tone::Warning, "Needs another try"),
        StepState::Needed | StepState::Explaining if current => (Tone::Info, "Next"),
        StepState::Needed | StepState::Explaining => (Tone::Inactive, "Waiting"),
    }
}

/// The four things pairing sets up, with the macOS prompt that comes next explained first.
pub(super) fn pairing(s: &State, step: FirstRun, k: &Kit) -> Page {
    let helper = step == FirstRun::PairingHelper;
    let certificate = s.setup_step(SetupStep::Certificate);
    let installed = s.setup_step(SetupStep::Helper);
    let failure = [&certificate, &installed]
        .into_iter()
        .find_map(|state| match state {
            StepState::Failed(message) => Some(message.clone()),
            _ => None,
        });
    let working = certificate == StepState::Running || installed == StepState::Running;
    let rows = [
        pair_row(
            "Connected to your account",
            "You are logged in to Reoclo.",
            (Tone::Success, "Done"),
            k,
        ),
        pair_row(
            "Runs in the background",
            "Reoclo keeps your connections open, and starts again each time you log in to your Mac.",
            (Tone::Success, "Done"),
            k,
        ),
        pair_row(
            "Trusts Reoclo's certificate",
            "Pages from your servers open securely in your browser, with no warnings.",
            pair_state(certificate, !helper),
            k,
        ),
        pair_row(
            "Opens web addresses on this Mac",
            "A small helper sends Reoclo's addresses to this Mac, so links open like any website.",
            pair_state(installed, helper),
            k,
        ),
    ];
    let mut list = col().w_full().gap(MD);
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            list = list.child(rule(k));
        }
        list = list.child(item);
    }
    let (heading, copy) = if helper {
        (
            "macOS asks for an administrator password next",
            "Installing the helper changes a system setting, so macOS asks for the password of an administrator on this Mac. On most Macs that is your own password.",
        )
    } else {
        (
            "macOS asks for your password next",
            "macOS says you are making changes to your Certificate Trust Settings. Enter the password you use to log in to this Mac, or use Touch ID. Reoclo never sees it.",
        )
    };
    let notice = col()
        .key("first-run-prompt-notice")
        .role("note")
        .w_full()
        .gap(XS)
        .padding(LG)
        .radius(k.radius.xl)
        .fill(k.c.layer_02)
        .border(RULE, k.c.border_base)
        .child(
            text(heading)
                .role("heading")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(text(copy).font(BODY).color(k.c.text_muted).wrap());
    let mut body: Vec<El> = failure
        .map(|message| alert(&message, k))
        .into_iter()
        .collect();
    body.push(list);
    body.push(notice);
    Page {
        title: "Setting up this Mac",
        detail: "Reoclo needs a few things on this Mac. You only do this once.",
        body,
        actions: vec![
            Button::new("Set up later")
                .on_press(|s: &mut State| s.skip_pairing())
                .secondary(k)
                .key("first-run-set-up-later"),
            Button::new("Continue")
                .disabled(working)
                .on_press(|s: &mut State| s.continue_setup())
                .primary(k)
                .key("first-run-next"),
        ],
    }
}
