use super::screens::{facts, hint, plain_list};
use super::*;
use crate::{
    cli::{self, Cli},
    system::command_path::CommandState,
    ui::StepState,
};
use rune_kit::Button;

pub(super) fn tools(s: &State, k: &Kit) -> Page {
    let mut body = Vec::new();
    match &s.flow.tools {
        StepState::Failed(message) => body.push(alert(message, k)),
        StepState::Running => body.push(indicator(Tone::Info, "Adding the command", k)),
        StepState::Needed | StepState::Explaining | StepState::Done => {}
    }
    let version = match &s.cli {
        Some(Cli::Ready { version, .. }) => version.as_str(),
        _ => "",
    };
    body.push(facts(
        &[("Location", "~/.local/bin/reoclo"), ("Version", version)],
        k,
    ));
    body.push(plain_list(
        &[
            "The command updates with this app, so it never falls behind.",
            "Reoclo adds one line to your shell settings so agents can find it. No password is needed.",
        ],
        k,
    ));
    body.push(hint(
        "Restart any agent that is already open so it finds the command.",
        k,
    ));
    Page {
        title: "Command-line tools for your agents",
        detail: "Coding agents such as Claude Code or Cursor work with Reoclo through its reoclo command. Reoclo puts its own copy where agents look for it.",
        body,
        actions: vec![
            Button::new("Skip")
                .on_press(|s: &mut State| s.finish_tools())
                .secondary(k)
                .key("first-run-skip"),
            Button::new("Add command-line tools")
                .disabled(s.flow.tools == StepState::Running)
                .on_press(|s: &mut State| s.install_command())
                .primary(k)
                .key("first-run-next"),
        ],
    }
}

/// The `reoclo` already on this Mac that the app leaves alone: where, which version, and whether it is too old.
fn existing_command(s: &State) -> Option<(String, String, bool)> {
    if let Some(CommandState::AlreadyPresent { path, version }) = &s.command {
        let version = version.clone().unwrap_or_default();
        let old = cli::older(&version, cli::MIN_VERSION);
        return Some((path.display().to_string(), version, old));
    }
    match &s.cli {
        Some(Cli::Ready {
            path,
            version,
            bundled: false,
        }) => Some((path.display().to_string(), version.clone(), false)),
        Some(Cli::TooOld { path, version }) => {
            Some((path.display().to_string(), version.clone(), true))
        }
        Some(Cli::Ready { bundled: true, .. } | Cli::Missing) | None => None,
    }
}

pub(super) fn tools_present(s: &State, k: &Kit) -> Page {
    let (path, version, old) = existing_command(s).unwrap_or_default();
    let (detail, status, tone) = if old {
        (
            "Reoclo found an older reoclo command on this Mac. It leaves that copy alone, so update it yourself and your agents keep working.",
            "Needs an update",
            Tone::Warning,
        )
    } else {
        (
            "Reoclo found the reoclo command on this Mac. It leaves that copy alone and uses it, so your agents keep working as they do today.",
            "Already on this Mac",
            Tone::Success,
        )
    };
    let mut body = vec![
        indicator(tone, status, k),
        facts(&[("Found", &path), ("Version", &version)], k),
    ];
    if old && let Some(Cli::TooOld { path, .. }) = &s.cli {
        body.push(field("Update it with", cli::upgrade_command(path), k));
    } else {
        body.push(hint(
            "Whatever installed this copy keeps it up to date, Homebrew for one.",
            k,
        ));
    }
    Page {
        title: "Command-line tools for your agents",
        detail,
        body,
        actions: vec![
            Button::new("Continue")
                .on_press(|s: &mut State| s.finish_tools())
                .primary(k)
                .key("first-run-next"),
        ],
    }
}
