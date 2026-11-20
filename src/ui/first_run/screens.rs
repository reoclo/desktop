use super::*;
use crate::ui::{LoginStep, form::actions::default_hostname};
use rune_kit::Button;

/// The text "Copy prompt" puts on the clipboard, for the person to paste into their agent.
pub const AGENT_PROMPT: &str = "I use Reoclo to work with our team's servers. The reoclo command is installed and logged in on this Mac. Run reoclo whoami to check the account, then reoclo --help to find the commands you need. Reoclo's MCP server, reoclo mcp, serves its guides. Ask me before you change anything on a server.";

pub(super) fn page(s: &State, step: FirstRun, k: &Kit) -> Page {
    match step {
        FirstRun::Welcome => welcome(s, k),
        FirstRun::Link => link(s, k),
        FirstRun::Waiting => waiting(s, k),
        FirstRun::Pairing | FirstRun::PairingHelper => pairing::pairing(s, step, k),
        FirstRun::Tools => tools::tools(s, k),
        FirstRun::ToolsBrew => tools::tools_present(s, k),
        FirstRun::Ready | FirstRun::ReadyAgents => ready(s, step, k),
    }
}

pub(super) fn hint(copy: &str, k: &Kit) -> El {
    text(copy).font(HINT).color(k.c.text_muted).wrap()
}

pub(super) fn plain_list(items: &[&str], k: &Kit) -> El {
    let mut list = col().w_full().gap(SM);
    for item in items {
        list = list.child(
            row()
                .w_full()
                .gap(MD)
                .child(el().size(XS, XS).mt(SM).fill(k.c.text_muted).a11y_hidden())
                .child(
                    text(*item)
                        .font(BODY)
                        .color(k.c.text_base)
                        .wrap()
                        .grow(1.0)
                        .min_w(0.0),
                ),
        );
    }
    list
}

pub(super) fn facts(pairs: &[(&str, &str)], k: &Kit) -> El {
    let mut grid = row().w_full().gap(LG);
    for (label, value) in pairs {
        grid = grid.child(col().w(FACT_WIDTH).child(field(label, value, k)));
    }
    grid
}

fn login_failure(s: &State) -> Option<&str> {
    match s.login_step() {
        LoginStep::Failed(message) => Some(message),
        _ => None,
    }
}

fn log_in(label: &str, k: &Kit) -> El {
    Button::new(label)
        .on_press(|s: &mut State| s.begin_login())
        .primary(k)
        .key("first-run-next")
}

fn welcome(s: &State, k: &Kit) -> Page {
    let failure = login_failure(s);
    let mut body: Vec<El> = failure
        .map(|message| alert(message, k))
        .into_iter()
        .collect();
    body.push(plain_list(
        &[
            "Your browser opens the Reoclo sign-in page, where you approve this Mac.",
            "Reoclo starts a small background service. macOS lists it under Login Items and may show a notice saying so.",
        ],
        k,
    ));
    body.push(hint(
        "No account yet? You can create one on the sign-in page.",
        k,
    ));
    Page {
        title: "Welcome to Reoclo",
        detail: "Log in once and this Mac is ready for your coding agent. Everything happens here and in your browser, so you never need a terminal.",
        body,
        actions: vec![log_in(
            if failure.is_some() {
                "Try again"
            } else {
                "Log in"
            },
            k,
        )],
    }
}

fn link(s: &State, k: &Kit) -> Page {
    let mut body: Vec<El> = login_failure(s)
        .map(|message| alert(message, k))
        .into_iter()
        .collect();
    body.extend(link_facts(s, k));
    Page {
        title: "Open a link from your team",
        detail: "This link opens a page that runs on one of your team's servers. Log in first, and Reoclo opens it as soon as this Mac is ready.",
        body,
        actions: vec![
            Button::new("Cancel")
                .on_press(|s: &mut State| s.link = None)
                .secondary(k)
                .key("first-run-cancel"),
            log_in("Log in and open", k),
        ],
    }
}

fn link_facts(s: &State, k: &Kit) -> Vec<El> {
    let Some(pending) = &s.link else {
        return Vec::new();
    };
    let link = &pending.link;
    let address = format!("https://{}", default_hostname(&link.name, &link.server));
    let port = link.port.to_string();
    vec![
        facts(&[("Organization", &link.org), ("Server", &link.server)], k),
        facts(&[("Opens as", "HTTPS"), ("Port on the server", &port)], k),
        field("Address on this Mac", &address, k),
    ]
}

fn waiting(s: &State, k: &Kit) -> Page {
    let cancel = Button::new("Cancel")
        .on_press(|s: &mut State| s.cancel_login())
        .secondary(k)
        .key("first-run-cancel");
    let (title, detail, status) = match s.login_step() {
        LoginStep::InstallingService | LoginStep::WaitingForDaemon => (
            "Starting Reoclo",
            "Reoclo is starting its background service. macOS may list it under Login Items and show a notice saying so.",
            "Starting the background service",
        ),
        LoginStep::RequestingCode => (
            "Starting Reoclo",
            "Reoclo is asking for a sign-in code. Your browser opens next.",
            "Asking for a sign-in code",
        ),
        LoginStep::SignedIn => (
            "Starting Reoclo",
            "You are logged in. Reoclo is getting this Mac ready.",
            "Finishing sign-in",
        ),
        LoginStep::Idle | LoginStep::WaitingForApproval | LoginStep::Failed(_) => (
            "Approve this Mac in your browser",
            "Your browser opened the Reoclo sign-in page. Check that it shows the code below, then choose Approve. This page moves on by itself.",
            "Waiting for you to approve",
        ),
    };
    let mut body = vec![indicator(Tone::Info, status, k)];
    body.extend(device_code(s, k, "first-run-"));
    Page {
        title,
        detail,
        body,
        actions: vec![cancel],
    }
}

fn ready(s: &State, step: FirstRun, k: &Kit) -> Page {
    let prompt = col()
        .w_full()
        .gap(MD)
        .child(
            text(AGENT_PROMPT)
                .key("first-run-prompt")
                .font(BODY)
                .color(k.c.text_base)
                .wrap()
                .selectable(true)
                .selection_color(k.selection())
                .w_full()
                .padding(LG)
                .radius(k.radius.xl)
                .fill(k.c.layer_02)
                .border(RULE, k.c.border_strong),
        )
        .child(
            row().child(
                Button::new("Copy prompt")
                    .on_press_tx(|s: &mut State, tx| {
                        tx.copy(AGENT_PROMPT);
                        s.notice = Some("Prompt copied. Paste it into your agent.".into());
                    })
                    .primary(k)
                    .key("first-run-copy-prompt"),
            ),
        );
    let mut body = vec![prompt];
    if step == FirstRun::ReadyAgents {
        body.push(rule(k));
        body.push(agents::section(s, k));
    }
    Page {
        title: "Ready for your agent",
        detail: "Copy this prompt and paste it into your coding agent. It takes over from there.",
        body,
        actions: vec![
            Button::new("Done")
                .on_press(|s: &mut State| s.finish_setup())
                .primary(k)
                .key("first-run-next"),
        ],
    }
}
