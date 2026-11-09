use super::{State, ViewState, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Switch};

pub fn view(s: &State, k: &Kit) -> El {
    let header = row()
        .w_full()
        .padding(LG)
        .center_y()
        .child(text("Reoclo").font(HEADING).color(k.c.text_base).grow(1.0));
    let mut list = col().w_full().padding(LG).gap(XL);
    if s.phase == ViewState::Loaded {
        if !s.model.approvals.is_empty() {
            list = list.child(
                text(format!(
                    "{} pending approval{}",
                    s.model.approvals.len(),
                    if s.model.approvals.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                ))
                .font(LABEL)
                .color(k.c.warning.fg),
            );
            for approval in &s.model.approvals {
                let approve = format!("/v1/approvals/{}/approve", segment(&approval.id));
                let deny = format!("/v1/approvals/{}/deny", segment(&approval.id));
                list = list.child(
                    col()
                        .gap(SM)
                        .child(text(&approval.agent).font(BODY).color(k.c.text_base).wrap())
                        .child(
                            text(&approval.description)
                                .font(BODY)
                                .color(k.c.text_muted)
                                .wrap(),
                        )
                        .child(
                            row()
                                .gap(SM)
                                .child(
                                    Button::new("Approve")
                                        .variant(ButtonVariant::Contrast)
                                        .disabled(s.busy)
                                        .on_press(move |s: &mut State| {
                                            s.request("POST", &approve, None)
                                        })
                                        .build(k)
                                        .key(format!("approve-{}", approval.id)),
                                )
                                .child(
                                    Button::new("Deny")
                                        .disabled(s.busy)
                                        .on_press(move |s: &mut State| {
                                            s.request("POST", &deny, None)
                                        })
                                        .build(k)
                                        .px(0.0)
                                        .mr(MD)
                                        .key(format!("deny-{}", approval.id)),
                                ),
                        ),
                );
            }
            list = list.child(el().h(RULE).w_full().fill(k.c.border_base));
        }
        for ((org, server), tunnels) in s.model.groups() {
            list = list.child(
                col()
                    .gap(XS)
                    .child(text(server).font(HEADING).color(k.c.text_base))
                    .child(text(org).font(LABEL).color(k.c.text_muted)),
            );
            for tunnel in tunnels {
                let path = format!(
                    "/v1/tunnels/{}/{}",
                    segment(&tunnel.name),
                    if tunnel.enabled { "down" } else { "up" }
                );
                let copy = tunnel.endpoint().to_owned();
                let url = tunnel.url.clone();
                let mut item = col()
                    .gap(SM)
                    .w_full()
                    .child(
                        row()
                            .w_full()
                            .gap(SM)
                            .center_y()
                            .child(text(&tunnel.name).font(BODY).color(k.c.text_base).grow(1.0))
                            .child(
                                Switch::new(tunnel.enabled)
                                    .label(format!("{} tunnel up", tunnel.name))
                                    .hide_label()
                                    .disabled(s.busy)
                                    .on_toggle(move |s: &mut State| s.request("POST", &path, None))
                                    .build(k)
                                    .key(format!("toggle-{}", tunnel.name)),
                            ),
                    )
                    .child(status(tunnel.state, k))
                    .child(
                        text(tunnel.endpoint())
                            .font(MONO)
                            .color(k.c.text_muted)
                            .wrap()
                            .selectable(true),
                    )
                    .child(
                        row()
                            .gap(SM)
                            .child(
                                Button::new(if tunnel.url.is_some() {
                                    "Copy URL"
                                } else {
                                    "Copy endpoint"
                                })
                                .on_press_tx(move |s: &mut State, tx| {
                                    tx.copy(&copy);
                                    s.notice = Some("Endpoint copied.".into());
                                })
                                .build(k)
                                .px(0.0)
                                .mr(MD)
                                .key(format!("copy-{}", tunnel.name)),
                            )
                            .child(
                                Button::new("Open")
                                    .disabled(url.is_none())
                                    .on_press_tx(move |s: &mut State, tx| {
                                        if let Some(url) = &url
                                            && let Err(error) = tx.open_url(url)
                                        {
                                            s.error = Some(error.to_string());
                                        }
                                    })
                                    .build(k)
                                    .key(format!("open-{}", tunnel.name)),
                            ),
                    );
                if tunnel.share.as_ref().is_some_and(|share| share.tailnet) {
                    item = item.child(text("Shared with tailnet").font(LABEL).color(k.c.info.fg));
                }
                list = list.child(item);
            }
        }
    } else {
        list = col().w_full().child(data_state(s, k));
    }
    let message = if s.phase == ViewState::Loaded {
        s.error.as_ref().or(s.notice.as_ref())
    } else {
        s.notice.as_ref()
    };
    if let Some(message) = message {
        list = list.child(
            text(message)
                .role("status")
                .font(BODY)
                .color(k.c.text_base)
                .wrap(),
        );
    }
    let viewport = col()
        .key("panel-scroll")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(list);
    let footer = row()
        .w_full()
        .gap(SM)
        .padding(LG)
        .child(
            Button::new("Open Reoclo")
                .variant(ButtonVariant::Contrast)
                .on_press_tx(|s: &mut State, tx| {
                    if let Some(handle) = &s.window_handle {
                        handle.set_dock_icon(true);
                        handle.open();
                        tx.close_window();
                    }
                })
                .build(k)
                .key("open-reoclo"),
        )
        .child(
            Button::new("Quit")
                .on_press_tx(|_: &mut State, tx| tx.quit())
                .build(k)
                .key("quit"),
        );
    col()
        .key("menu-panel")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .border(RULE, k.c.border_base)
        .child(header)
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(k.scroll_bar(viewport))
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(footer)
}
