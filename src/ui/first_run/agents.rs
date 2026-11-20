use super::screens::hint;
use super::*;
use crate::agents::{Action, Agent, Change};
use crate::ui::AgentsStep;
use rune_kit::Button;

fn heading(copy: &str, k: &Kit) -> El {
    text(copy)
        .role("heading")
        .font(HEADING)
        .color(k.c.text_base)
}

/// How one agent reads: what Connect did when it has run, else what the config says.
fn state_of(agent: &Agent, change: Option<&Change>) -> (Tone, &'static str, Option<String>) {
    if let Some(change) = change {
        return match change.action {
            Action::Installed => (Tone::Success, "Connected", None),
            Action::Updated => (Tone::Success, "Updated", None),
            Action::Unchanged => (Tone::Success, "Already connected", None),
            Action::Removed => (Tone::Inactive, "Removed", None),
            Action::NotFound => (Tone::Inactive, "Not found", None),
            Action::Failed => (
                Tone::Warning,
                "Needs a look",
                change.error.as_ref().map(|problem| problem.message.clone()),
            ),
        };
    }
    match (agent.installed, agent.registered) {
        (false, _) => (Tone::Inactive, "Not found", None),
        (true, Some(true)) => (Tone::Success, "Connected", None),
        (true, Some(false)) => (Tone::Ready, "Found", None),
        (true, None) => (Tone::Warning, "Needs a look", agent.error.clone()),
    }
}

fn agent_row(agent: &Agent, change: Option<&Change>, k: &Kit) -> El {
    let (tone, label, problem) = state_of(agent, change);
    let mut item = col().w_full().gap(XS).child(
        row()
            .w_full()
            .gap(LG)
            .center_y()
            .child(
                text(agent.name.clone())
                    .font(k.ty.body_medium)
                    .color(k.c.text_base)
                    .w(FACT_WIDTH),
            )
            .child(
                text(agent.config_path.clone())
                    .font(MONO)
                    .color(k.c.text_muted)
                    .truncate()
                    .grow(1.0)
                    .min_w(0.0),
            )
            .child(indicator(tone, label, k)),
    );
    if let Some(problem) = problem {
        item = item.child(text(problem).font(HINT).color(k.c.text_muted).wrap());
    }
    item
}

fn connect_button(label: &str, disabled: bool, k: &Kit) -> El {
    row().child(
        Button::new(label)
            .disabled(disabled)
            .on_press(|s: &mut State| s.connect_agents())
            .secondary(k)
            .key("first-run-connect-agents"),
    )
}

/// Connect your agents: the agents found on this Mac, one button to register Reoclo with them, and each result.
pub(super) fn section(s: &State, k: &Kit) -> El {
    let results: &[Change] = match &s.agents {
        AgentsStep::Done { results, .. } => results,
        _ => &[],
    };
    let mut list = col().w_full().gap(SM);
    for agent in s.agents.found() {
        let change = results.iter().find(|change| change.agent == agent.id);
        list = list.child(agent_row(agent, change, k));
    }
    let mut section = col()
        .key("first-run-agents")
        .w_full()
        .gap(MD)
        .child(heading("Connect your agents", k))
        .child(
            text("Reoclo can add itself to the agents on this Mac as an MCP server, so they find its tools and guides without the prompt.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(list);
    section = match &s.agents {
        AgentsStep::Listed(found) if found.iter().all(|agent| agent.registered == Some(true)) => {
            section.child(hint("Reoclo is already connected to your agents.", k))
        }
        AgentsStep::Listed(_) => section.child(connect_button("Connect agents", false, k)),
        AgentsStep::Connecting(_) => section.child(connect_button("Connecting", true, k)),
        AgentsStep::Done { results, .. }
            if results.iter().any(|change| change.action == Action::Failed) =>
        {
            section.child(connect_button("Try again", false, k))
        }
        AgentsStep::Done { .. } => section.child(hint(
            "Restart any agent that is already open so it finds Reoclo.",
            k,
        )),
        AgentsStep::Failed { message, .. } => section
            .child(alert(message, k))
            .child(connect_button("Try again", false, k)),
        AgentsStep::NotChecked | AgentsStep::Checking | AgentsStep::Unavailable(_) => section,
    };
    section
}
