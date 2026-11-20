//! The first-run flow for someone who never opens a terminal. `State::first_run_step` picks the
//! screen from what the daemon, the system and the workers have reported, and each button calls
//! the same plumbing the rest of the app uses.
mod agents;
mod fixture;
mod pairing;
mod screens;
mod tools;
use super::{State, ViewState, primitives::*};
use crate::theme::*;
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::Kit;

/// One screen of the flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstRun {
    Welcome,
    /// A teammate's `reoclo://open` link arrived before this Mac was set up.
    Link,
    /// The service is starting, or the device code is on screen and the browser is open.
    Waiting,
    /// The certificate is next; macOS asks for the login password.
    Pairing,
    /// The web address helper is next; macOS asks for an administrator.
    PairingHelper,
    /// No `reoclo` on the Mac, so the app adds its own copy.
    Tools,
    /// Another `reoclo`, such as Homebrew's, is already installed and stays in charge.
    ToolsBrew,
    Ready,
    /// Ready, plus Connect your agents for the coding agents found on this Mac.
    ReadyAgents,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    LogIn,
    Pair,
    Tools,
    Ready,
}

const STAGES: [(Stage, &str); 4] = [
    (Stage::LogIn, "Log in"),
    (Stage::Pair, "Set up this Mac"),
    (Stage::Tools, "Tools for your agents"),
    (Stage::Ready, "Ready for your agent"),
];

impl FirstRun {
    fn stage(self) -> Stage {
        match self {
            Self::Welcome | Self::Link | Self::Waiting => Stage::LogIn,
            Self::Pairing | Self::PairingHelper => Stage::Pair,
            Self::Tools | Self::ToolsBrew => Stage::Tools,
            Self::Ready | Self::ReadyAgents => Stage::Ready,
        }
    }
}

/// What one step puts on the page: its heading, its body and its footer actions.
struct Page {
    title: &'static str,
    detail: &'static str,
    body: Vec<El>,
    /// Footer actions, left to right. The primary action, when there is one, comes last.
    actions: Vec<El>,
}

fn rail(step: FirstRun, k: &Kit) -> El {
    let current = step.stage();
    let at = STAGES.iter().position(|(stage, _)| *stage == current);
    let mut list = col().w_full().gap(XS);
    for (index, (stage, name)) in STAGES.iter().enumerate() {
        let (tone, label) = match at {
            // The last step is reached once everything before it is done.
            Some(at) if index < at || (index == at && *stage == Stage::Ready) => {
                (Tone::Success, "Done")
            }
            Some(at) if index == at => (Tone::Info, "In progress"),
            _ => (Tone::Inactive, "Not started"),
        };
        let here = *stage == current;
        let mut item = col()
            .w_full()
            .gap(XS)
            .padding(SM)
            .radius(k.radius.md)
            .child(
                text(*name)
                    .font(k.ty.body_medium)
                    .color(if here { k.c.text_base } else { k.c.text_muted })
                    .truncate(),
            )
            .child(indicator(tone, label, k));
        if here {
            item = item.fill(k.c.layer_03).role("status");
        }
        list = list.child(item);
    }
    col()
        .w(FIRST_RUN_RAIL)
        .h_full()
        .fill(k.c.bg_deep)
        .px(MD)
        .pt(XXL + LG)
        .pb(LG)
        .gap(XL)
        .child(wordmark(k).px(SM))
        .child(list)
        .child(spacer())
        .child(
            text("You can run setup again from Settings.")
                .font(HINT)
                .color(k.c.text_muted)
                .px(SM)
                .wrap(),
        )
}

pub fn view(s: &State, k: &Kit) -> El {
    if s.phase == ViewState::Loading {
        return super::skeleton::first_run(k);
    }
    let step = s.first_run_step();
    let page = screens::page(s, step, k);
    let mut content = col()
        .grow(1.0)
        .min_w(0.0)
        .max_w(FIRST_RUN_MEASURE)
        .gap(XL)
        .child(title(page.title, Some(page.detail), k));
    for item in page.body {
        content = content.child(item);
    }
    let mut footer = row().w_full().gap(SM).px(XXL).py(LG).center_y();
    if let Some(notice) = &s.notice {
        footer = footer.child(
            text(notice)
                .role("status")
                .font(HINT)
                .color(k.c.text_muted)
                .truncate(),
        );
    }
    footer = footer.child(spacer());
    for action in page.actions {
        footer = footer.child(action);
    }
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(
            k.scroll_bar(
                col()
                    .w_full()
                    .grow(1.0)
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .scroll_edge_fade(LG)
                    .key("first-run-scroll")
                    .px(XXL)
                    .pt(XXL + LG)
                    .pb(XL)
                    .child(content),
            ),
        )
        .child(rule(k))
        .child(footer);
    row()
        .key("first-run")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .child(rail(step, k))
        .child(el().w(RULE).h_full().fill(k.c.border_base))
        .child(main)
}
