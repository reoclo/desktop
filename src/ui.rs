mod certificates;
mod dialogs;
mod fault;
mod fixtures;
mod form;
mod hosts;
mod panel;
mod replies;
mod settings;
mod skeleton;
mod state;
pub use fault::Fault;
mod primitives;
mod tunnels;
use crate::theme::{self, *};
use rune::{Cx, El, col, text};
use rune_kit::shell::{Sidebar, SidebarRow, SidebarSection, SplitView, Toolbar};
pub use state::{Page, Request, State, ViewState, read_preferences};

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
    let content = match s.phase {
        ViewState::Loaded | ViewState::Empty if s.has_tunnels() || s.page != Page::Tunnels => {
            match s.page {
                Page::Hosts => hosts::view(s, &k),
                Page::Certificates => certificates::view(s, &k),
                Page::Settings => settings::view(s, &k, &mut layer),
                Page::Tunnels => tunnels::view(s, &k),
            }
        }
        _ => primitives::data_state(s, &k),
    };
    let sidebar = Sidebar::new()
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
        )
        .section(
            SidebarSection::new("This Mac").rows(Page::ALL.into_iter().map(|page| {
                SidebarRow::new(page.name())
                    .selected(s.page == page)
                    .on_select(move |s: &mut State, _| s.show_page(page))
                    .build(&k)
                    .key(format!("nav-{}", page.name().to_lowercase()))
            })),
        )
        .build(&k);
    let status = if s.phase == ViewState::Offline {
        Some(primitives::indicator(primitives::Tone::Warning, "Daemon offline", &k).role("status"))
    } else if s.busy {
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
    let toolbar = match status {
        Some(status) => Toolbar::new().trailing(status),
        None => Toolbar::new(),
    };
    let body = SplitView::new(content)
        .sidebar(&s.sidebar, |s: &mut State| &mut s.sidebar, sidebar)
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(0.0)
        .reduced_motion(cx.is_reduced_motion())
        .build(&k);
    let mut body = col().w_full().h_full().fill(k.c.page).child(body);
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
