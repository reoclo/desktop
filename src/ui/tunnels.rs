use super::{State, primitives::*};
use crate::{control::segment, model::TunnelState, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Cell, Kit, Switch, Table, TableColumn, TableStyle};
use std::borrow::Cow;

const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 36.0;
const MAX_VISIBLE_ROWS: usize = 8;

pub fn view(s: &State, k: &Kit) -> El {
    let saved = s.model.tunnels.len();
    let active = s
        .model
        .tunnels
        .iter()
        .filter(|t| t.state == TunnelState::Active)
        .count();
    let terminal = s.model.foreground.len();
    let summary = match (saved, terminal) {
        (0, _) => format!("{terminal} in a terminal"),
        (_, 0) => format!("{saved} saved, {active} active"),
        _ => format!("{saved} saved, {active} active, {terminal} in a terminal"),
    };
    let head = row()
        .w_full()
        .center_y()
        .gap(LG)
        .child(title("Tunnels", Some(&summary), k).grow(1.0))
        .child(
            Button::new("Add tunnel")
                .shortcut("cmd+n")
                .on_press(|s: &mut State| s.begin_add())
                .primary(k)
                .key("add-tunnel"),
        );
    let cell_kit = k.clone();
    let table_style = TableStyle {
        selected: k.c.layer_03,
        row_height: ROW_HEIGHT,
        header_height: HEADER_HEIGHT,
        header_font: LABEL,
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
    .column(TableColumn::new("state", "State").width(104.0))
    .column(
        TableColumn::new("name", "Name")
            .flex(1.0)
            .min(84.0)
            .sortable(),
    )
    .column(TableColumn::new("server", "Server").width(88.0).sortable())
    .column(TableColumn::new("kind", "Kind").width(64.0))
    .column(
        TableColumn::new("endpoint", "Endpoint")
            .flex(3.0)
            .min(120.0),
    )
    .column(TableColumn::new("up", "Up").width(48.0))
    .cell(move |s: &State, i, column| {
        let t = &s.model.tunnels[i];
        match column {
            "state" => Cell::el(status(t.state, &cell_kit)),
            "name" => Cell::text(&t.name),
            "server" => Cell::text(t.place()).mono(),
            "kind" => Cell::text(t.kind.to_uppercase()),
            "up" => {
                let path = format!(
                    "/v1/tunnels/{}/{}",
                    segment(&t.name),
                    if t.enabled { "down" } else { "up" }
                );
                Cell::el(
                    Switch::new(t.enabled)
                        .label(format!("{} tunnel up", t.name))
                        .hide_label()
                        .disabled(s.busy)
                        .on_toggle(move |s: &mut State| s.request("POST", &path, None))
                        .build(&cell_kit)
                        .key(format!("toggle-row-{}", t.name)),
                )
            }
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
    .h(HEADER_HEIGHT + saved.clamp(1, MAX_VISIBLE_ROWS) as f32 * ROW_HEIGHT + RULE)
    .w_full();
    let mut body = col()
        .key("tunnels-page")
        .w_full()
        .padding(XL)
        .gap(XL)
        .child(head);
    if let Some(error) = s.error.as_ref().filter(|_| s.modal.is_none()) {
        body = body.child(alert(error, k));
    }
    if let Some(requests) = approvals(s, k) {
        body = body.child(
            requests
                .padding(LG)
                .border(RULE, k.c.border_base)
                .radius(k.radius.xl),
        );
    }
    if let Some(running) = terminal_tunnels(s, k) {
        body = body.child(
            running
                .padding(LG)
                .border(RULE, k.c.border_base)
                .radius(k.radius.xl),
        );
    }
    if saved > 0 {
        body = body.child(table);
    }
    if let Some(t) = s.selected() {
        let up_path = format!("/v1/tunnels/{}/up", segment(&t.name));
        let copy = t.endpoint().to_owned();
        let url = t.url.clone();
        let mut name = col()
            .grow(1.0)
            .min_w(0.0)
            .gap(XS)
            .child(text(&t.name).font(HEADING).color(k.c.text_base));
        if !t.message.eq_ignore_ascii_case(t.state.label()) {
            name = name.child(text(&t.message).font(BODY).color(k.c.text_muted).wrap());
        }
        let mut header = row().w_full().center_y().gap(LG).child(name);
        if t.share.as_ref().is_some_and(|share| share.tailnet) {
            header = header.child(indicator(Tone::Info, "Shared with tailnet", k));
        }
        header = header.child(status(t.state, k));
        let mut details = col()
            .gap(LG)
            .w_full()
            .child(el().w_full().h(RULE).fill(k.c.border_base))
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
                            "Traffic in / out",
                            &format!(
                                "{} / {}",
                                bytes(t.traffic.bytes_in),
                                bytes(t.traffic.bytes_out)
                            ),
                            k,
                        )
                        .grow(1.0),
                    )
                    .child(field("Streams", &t.traffic.streams.to_string(), k)),
            );
        if let Some(detail) = &t.detail {
            details = details.child(alert(detail, k));
        }
        if t.action != "None" && !t.action.is_empty() {
            details = details.child(text(&t.action).font(BODY).color(k.c.text_muted).wrap());
        }
        let mut actions = row().w_full().gap(SM);
        if t.state == TunnelState::NeedsSignIn {
            actions = actions.child(
                Button::new("Sign in")
                    .on_press(|s: &mut State| s.request("POST", "/v1/auth/login", None))
                    .primary(k)
                    .key("tunnel-sign-in"),
            );
        }
        if matches!(
            t.state,
            TunnelState::Failed
                | TunnelState::Expired
                | TunnelState::ClosedByOperator
                | TunnelState::ServerOffline
                | TunnelState::QuotaReached
        ) {
            actions = actions.child(
                Button::new("Try again")
                    .disabled(s.busy)
                    .on_press(move |s: &mut State| s.request("POST", &up_path, None))
                    .primary(k)
                    .key("retry-tunnel"),
            );
        }
        if t.kind == "tcp" {
            let (org, server, name) = (
                t.org.clone(),
                t.server_id.clone(),
                format!("{}-https", t.name),
            );
            let target = t.target.clone().unwrap_or_default();
            actions = actions.child(
                Button::new("Add HTTPS")
                    .help("Reach this service at a local HTTPS address")
                    .on_press(move |s: &mut State| {
                        s.begin_https(&org, &server, &target, name.clone())
                    })
                    .secondary(k)
                    .key("https-tunnel"),
            );
        }
        if let Some(url) = url {
            actions = actions.child(
                Button::new("Open")
                    .on_press_tx(move |s: &mut State, tx| {
                        if let Err(error) = tx.open_url(&url) {
                            s.error = Some(error.to_string());
                        }
                    })
                    .secondary(k)
                    .key("open-endpoint"),
            );
        }
        details = details.child(
            actions
                .child(
                    Button::new("Copy endpoint")
                        .on_press_tx(move |s: &mut State, tx| {
                            tx.copy(&copy);
                            s.notice = Some("Endpoint copied.".into());
                        })
                        .secondary(k)
                        .key("copy-endpoint"),
                )
                .child(if t.kind == "local" {
                    // The form edits server tunnels; local routes change with reoclo tunnel edit.
                    el()
                } else {
                    Button::new("Edit tunnel")
                        .on_press(|s: &mut State| s.edit_selected())
                        .secondary(k)
                        .key("edit-tunnel")
                })
                .child(spacer())
                .child(
                    Button::new("Remove")
                        .on_press(|s: &mut State| s.modal = Some("remove".into()))
                        .danger(k)
                        .key("remove-tunnel"),
                ),
        );
        if !s.model.recent_logs.is_empty() {
            let mut events = col().w_full().gap(XS).child(
                text("Recent daemon events")
                    .font(LABEL)
                    .color(k.c.text_muted),
            );
            for line in s.model.recent_logs.iter().rev().take(3) {
                events = events.child(
                    text(line)
                        .font(MONO)
                        .color(k.c.text_base)
                        .wrap()
                        .selectable(true),
                );
            }
            details = details.child(events);
        }
        body = body.child(details);
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
