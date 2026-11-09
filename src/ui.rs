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
pub use state::{Page, Request, State, ViewState};

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
        ViewState::Loaded | ViewState::Empty
            if s.phase == ViewState::Loaded || s.page != Page::Tunnels =>
        {
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
        .header(
            text("Reoclo")
                .font(HEADING)
                .color(k.c.text_base)
                .px(LG)
                .py(XL),
        )
        .section(
            SidebarSection::new("This Mac").rows(Page::ALL.into_iter().map(|page| {
                SidebarRow::new(page.name())
                    .selected(s.page == page)
                    .on_select(move |s: &mut State, _| {
                        s.page = page;
                        s.error = None;
                    })
                    .build(&k)
                    .key(format!("nav-{}", page.name().to_lowercase()))
            })),
        )
        .build(&k);
    let toolbar = if s.phase == ViewState::Offline {
        Toolbar::new().trailing(text("Daemon offline").font(LABEL).color(k.c.warning.fg))
    } else {
        Toolbar::new()
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
