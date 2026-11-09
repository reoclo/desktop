use super::{State, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Cell, Kit, Switch, Table, TableColumn, TableStyle};
use std::borrow::Cow;

pub fn view(s: &State, k: &Kit) -> El {
    let head = row()
        .w_full()
        .center_y()
        .gap(LG)
        .child(title("Tunnels", "Your servers, reachable from this Mac.", k).grow(1.0))
        .child(
            Button::new("Add tunnel")
                .variant(ButtonVariant::Contrast)
                .on_press(|s: &mut State| s.begin_add())
                .build(k)
                .key("add-tunnel"),
        );
    let badge_kit = k.clone();
    let table_style = TableStyle {
        selected: k.c.layer_03,
        ..TableStyle::from_kit(k)
    };
    let table = Table::new(
        |s: &State| &s.table,
        |s: &mut State| &mut s.table,
        |s: &State| s.model.tunnels.len(),
        |s: &State, i| Cow::Borrowed(s.model.tunnels[i].name.as_str()),
    )
    .name("Saved tunnels")
    .style(table_style)
    .column(TableColumn::new("state", "State").width(112.0))
    .column(
        TableColumn::new("name", "Name")
            .flex(1.0)
            .min(80.0)
            .sortable(),
    )
    .column(TableColumn::new("server", "Server").width(96.0).sortable())
    .column(TableColumn::new("kind", "Kind").width(64.0))
    .column(TableColumn::new("endpoint", "Endpoint").flex(2.0).min(80.0))
    .cell(move |s: &State, i, column| {
        let t = &s.model.tunnels[i];
        match column {
            "state" => Cell::el(status(t.state, &badge_kit)),
            "name" => Cell::text(&t.name),
            "server" => Cell::text(&t.server).mono(),
            "kind" => Cell::text(t.kind.to_uppercase()),
            _ => Cell::text(t.endpoint()).mono(),
        }
    })
    .on_select(|s: &mut State, name, _| s.model.selected = Some(name.into()))
    .on_sort(|s: &mut State, column, direction, _| {
        s.model.tunnels.sort_by(|a, b| {
            direction.apply(if column == "server" {
                a.server.cmp(&b.server)
            } else {
                a.name.cmp(&b.name)
            })
        });
    })
    .build(s, k)
    .h(224.0)
    .w_full();
    let mut body = col()
        .key("tunnels-page")
        .w_full()
        .padding(XL)
        .gap(XL)
        .child(head)
        .child(table);
    if let Some(t) = s.selected() {
        let name = t.name.clone();
        let toggle_path = format!(
            "/v1/tunnels/{}/{}",
            segment(&name),
            if t.enabled { "down" } else { "up" }
        );
        let enabled = t.enabled;
        let copy = t.endpoint().to_owned();
        let url = t.url.clone();
        let header = row()
            .w_full()
            .center_y()
            .gap(LG)
            .child(text(&t.name).font(HEADING).color(k.c.text_base).grow(1.0))
            .child(status(t.state, k))
            .child(
                Switch::new(enabled)
                    .label("Tunnel up")
                    .disabled(s.busy)
                    .on_toggle(move |s: &mut State| s.request("POST", &toggle_path, None))
                    .build(k)
                    .key("toggle-selected"),
            );
        let mut details = col()
            .gap(LG)
            .w_full()
            .child(rune::el().w_full().h(RULE).fill(k.c.border_base))
            .child(header)
            .child(field("Endpoint", t.endpoint(), k))
            .child(
                row()
                    .gap(XXL)
                    .child(
                        field(
                            "Target",
                            t.target
                                .as_deref()
                                .or(t.local_target.as_deref())
                                .unwrap_or("Not set"),
                            k,
                        )
                        .grow(1.0),
                    )
                    .child(field("Transport", t.via.as_deref().unwrap_or("reoclo"), k).grow(1.0))
                    .child(
                        field(
                            "Traffic received / sent",
                            &format!(
                                "{} / {}",
                                bytes(t.traffic.bytes_in),
                                bytes(t.traffic.bytes_out)
                            ),
                            k,
                        )
                        .grow(1.0),
                    )
                    .child(field("Open streams", &t.traffic.streams.to_string(), k)),
            )
            .child(text(&t.message).font(BODY).color(k.c.text_base).wrap());
        if let Some(detail) = &t.detail {
            details = details.child(
                text(detail)
                    .font(BODY)
                    .color(k.c.danger.fg)
                    .wrap()
                    .selectable(true),
            );
        }
        if t.action != "None" && !t.action.is_empty() {
            details = details.child(text(&t.action).font(BODY).color(k.c.text_muted).wrap());
        }
        details = details.child(
            row()
                .gap(SM)
                .child(
                    Button::new("Copy endpoint")
                        .on_press_tx(move |s: &mut State, tx| {
                            tx.copy(&copy);
                            s.notice = Some("Endpoint copied.".into());
                        })
                        .build(k)
                        .px(0.0)
                        .mr(MD)
                        .key("copy-endpoint"),
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
                        .px(0.0)
                        .mr(MD)
                        .key("open-endpoint"),
                )
                .child(
                    Button::new("Edit tunnel")
                        .on_press(|s: &mut State| s.edit_selected())
                        .build(k)
                        .px(0.0)
                        .mr(MD)
                        .key("edit-tunnel"),
                )
                .child(
                    Button::new("Remove")
                        .variant(ButtonVariant::Danger)
                        .on_press(|s: &mut State| s.modal = Some("remove".into()))
                        .build(k)
                        .px(0.0)
                        .key("remove-tunnel"),
                ),
        );
        if t.state == crate::model::TunnelState::NeedsSignIn {
            details = details.child(
                Button::new("Sign in")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut State| s.request("POST", "/v1/auth/login", None))
                    .build(k),
            );
        }
        if t.share.as_ref().is_some_and(|share| share.tailnet) {
            details = details.child(text("Shared with tailnet").font(LABEL).color(k.c.info.fg));
        }
        if !s.model.recent_logs.is_empty() {
            details = details.child(
                text("Recent daemon events")
                    .font(LABEL)
                    .color(k.c.text_muted),
            );
            for line in s.model.recent_logs.iter().rev().take(3) {
                details = details.child(
                    text(line)
                        .font(MONO)
                        .color(k.c.text_base)
                        .wrap()
                        .selectable(true),
                );
            }
        }
        body = body.child(details);
    }
    if let Some(error) = &s.error {
        body = body.child(
            text(error)
                .role("alert")
                .font(BODY)
                .color(k.c.danger.fg)
                .wrap(),
        );
    }
    if let Some(notice) = &s.notice {
        body = body.child(
            text(notice)
                .role("status")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap(),
        );
    }
    k.scroll_bar(
        col()
            .key("tunnels-scroll")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    )
}

fn bytes(value: u64) -> String {
    if value >= 1024 * 1024 {
        format!("{} MiB", value / (1024 * 1024))
    } else if value >= 1024 {
        format!("{} KiB", value / 1024)
    } else {
        format!("{value} B")
    }
}
