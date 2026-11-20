mod approvals;
mod certificates;
mod compact_tunnels;
mod connect_agents;
mod data_state;
mod dialogs;
mod fault;
mod first_run;
mod fixtures;
mod form;
mod hosts;
mod links;
mod logs;
mod modal;
mod onboarding;
mod panel;
mod panel_tunnel;
mod replies;
mod settings;
mod setup_flow;
mod skeleton;
mod state;
pub use fault::Fault;
mod primitives;
mod traffic_graph;
mod tunnel_detail;
mod tunnel_health;
mod tunnel_rows;
mod tunnels;
mod updates;
use crate::theme::{self, *};
pub use connect_agents::AgentsStep;
pub use first_run::FirstRun;
pub use links::{Effect, LinkStep, PendingLink};
use rune::{Cx, El, col, text};
use rune_kit::shell::{Sidebar, SidebarRow, SplitView, Toolbar};
pub use setup_flow::{DAEMON_WAIT_SECONDS, LoginStep, SetupStep, StepState};
pub use state::{Page, PortsScan, Request, State, ViewState, read_preferences};

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    cx.reduced_motion(s.reduced_motion || cx.system_reduced_motion());
    let dark = match s.appearance.as_str() {
        "light" => false,
        "dark" => true,
        _ => cx.system_dark(),
    };
    let k = match theme::kit(dark) {
        Ok(k) => k,
        Err(error) => return text(format!("Theme error: {error}")),
    };
    let mut layer = Vec::new();
    if s.panel {
        let body = panel::view(s, &k);
        dialogs::confirmations(s, &k, &mut layer);
        return rune_kit::root(body, layer);
    }
    if s.first_run {
        return rune_kit::root(first_run::view(s, &k), layer);
    }
    let content = match s.phase {
        _ if s.page == Page::Logs && s.phase != ViewState::Loading => logs::view(s, &k, &mut layer),
        ViewState::Offline | ViewState::SignedOut | ViewState::Error
            if s.page == Page::Settings =>
        {
            settings::view(s, &k, &mut layer)
        }
        ViewState::Offline | ViewState::SignedOut | ViewState::Error
            if !s.model.tunnels.is_empty() && s.page == Page::Tunnels =>
        {
            tunnels::view(s, &k)
        }
        ViewState::Loaded | ViewState::Empty => match s.page {
            Page::Hosts => hosts::view(s, &k),
            Page::Certificates => certificates::view(s, &k),
            Page::Settings => settings::view(s, &k, &mut layer),
            Page::Tunnels => tunnels::view(s, &k),
            Page::Logs => logs::view(s, &k, &mut layer),
        },
        _ => primitives::data_state(s, &k),
    };
    let mut sidebar = Sidebar::new()
        .header(primitives::wordmark(&k).px(SM).py(MD))
        .footer(
            rune::row().child(
                rune_kit::Button::new("Documentation")
                    .variant(rune_kit::ButtonVariant::GhostMuted)
                    .on_press_tx(|s: &mut State, tx| {
                        if let Err(error) =
                            tx.open_url("https://docs.reoclo.com/guides/local-https/")
                        {
                            s.error = Some(error.to_string());
                        }
                    })
                    .build(&k)
                    .px(SM)
                    .key("open-docs"),
            ),
        );
    for page in Page::ALL {
        sidebar = sidebar.child(
            SidebarRow::new(page.name())
                .selected(s.page == page)
                .on_select(move |s: &mut State, _| s.show_page(page))
                .build(&k)
                .key(format!("nav-{}", page.name().to_lowercase())),
        );
    }
    let sidebar = sidebar.build(&k);
    let status = if s.phase == ViewState::Offline {
        Some(primitives::indicator(primitives::Tone::Warning, "Daemon offline", &k).role("status"))
    } else if s.phase == ViewState::SignedOut {
        Some(primitives::indicator(primitives::Tone::Warning, "Signed out", &k).role("status"))
    } else if s.busy || !s.pending.is_empty() {
        Some(primitives::indicator(primitives::Tone::Info, "Working", &k).role("status"))
    } else {
        s.notice.as_ref().map(|notice| {
            text(notice)
                .role("status")
                .font(HINT)
                .color(k.c.text_muted)
                .truncate()
        })
    };
    let mut toolbar = Toolbar::new();
    // The sidebar carries the only wordmark; while it is collapsed, the top bar does.
    if !s.sidebar.is_open() {
        toolbar = toolbar.leading(primitives::wordmark(&k).key("toolbar-wordmark"));
    }
    if let Some(status) = status {
        toolbar = toolbar.trailing(status);
    }
    let body = SplitView::new(content)
        .sidebar(&s.sidebar, |s: &mut State| &mut s.sidebar, sidebar)
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(0.0)
        .reduced_motion(cx.is_reduced_motion())
        .build(&k);
    let mut body = col()
        .w_full()
        .h_full()
        .fill(k.c.page)
        .child(body)
        .on_layout(|s: &mut State, [_, _, width, _]| {
            s.viewport_width = width;
            let compact = width < COMPACT_WIDTH;
            if compact != s.compact {
                s.compact = compact;
                s.sidebar.set_open(!compact);
            }
        });
    if let Some(error) = &s.tray_error {
        body = body.child(
            text(error)
                .role("alert")
                .font(BODY)
                .color(k.c.warning.fg)
                .padding(LG)
                .wrap(),
        );
    }
    form::sheet(s, &k, &mut layer);
    dialogs::confirmations(s, &k, &mut layer);
    rune_kit::root(body, std::mem::take(&mut layer))
}
