use super::{State, primitives::*};
use crate::{
    control::segment,
    model::{TunnelState, TunnelView},
    theme::*,
};
use rune::{El, col, el, row, text};
use rune_kit::{Button, Kit};

pub fn view(t: &TunnelView, s: &State, k: &Kit) -> El {
    let mut name = col().grow(1.0).min_w(0.0).gap(XS).child(
        text(&t.name)
            .role("heading")
            .font(HEADING)
            .color(k.c.text_base),
    );
    if s.current_data()
        && !t.state.available()
        && let Some(subtitle) = t.subtitle()
    {
        name = name.child(text(subtitle).font(HINT).color(k.c.text_muted).wrap());
    }
    let mut header = row().w_full().center_y().gap(LG).child(name);
    if t.share.as_ref().is_some_and(|share| share.tailnet) {
        header = header.child(indicator(Tone::Info, "Shared with tailnet", k));
    }
    header = header.child(status(
        if s.current_data() {
            t.state
        } else {
            TunnelState::Unknown
        },
        k,
    ));
    let mut details = col()
        .gap(MD)
        .w_full()
        .child(el().w_full().h(RULE).fill(k.c.border_base))
        .child(header.key("tunnel-header"));
    let mut reason = col().w_full().gap(SM);
    if s.current_data() {
        if let Some(retry) = &t.retry {
            reason = reason.child(
                text(format!(
                    "Retry attempt {} at {:02}:{:02}:{:02} UTC.",
                    retry.attempt,
                    retry.next_at.hour(),
                    retry.next_at.minute(),
                    retry.next_at.second()
                ))
                .font(MONO)
                .color(k.c.info.fg)
                .role("status")
                .key("tunnel-retry-state"),
            );
        }
        if let Some(detail) = &t.detail {
            reason = reason.child(alert(detail, k).key("tunnel-reason"));
        }
        if !matches!(
            t.action.trim().to_ascii_lowercase().as_str(),
            "" | "none" | "see the detail" | "see detail" | "see details"
        ) {
            reason = reason.child(text(&t.action).font(BODY).color(k.c.text_muted).wrap());
        }
    }
    let facts = facts(t, s, k);
    if t.state.available() && s.current_data() {
        details = details.child(facts).child(reason).child(actions(t, s, k));
    } else {
        details = details.child(reason).child(actions(t, s, k)).child(facts);
    }
    details
}

fn facts(t: &TunnelView, s: &State, k: &Kit) -> El {
    let fields = col()
        .equal_columns(if s.compact { 2 } else { 4 })
        .gap(LG)
        .w_full()
        .max_w(if s.compact {
            FACT_WIDTH * 2.0 + LG
        } else {
            FACT_WIDTH * 4.0 + LG * 3.0
        })
        .child(field(
            "Target",
            t.target
                .as_deref()
                .or(t.local_target.as_deref())
                .unwrap_or("Not set"),
            k,
        ))
        .child(field("Server", t.place(), k))
        .child(field(
            "Transport",
            &format!(
                "{} · {}",
                t.kind.to_uppercase(),
                t.via.as_deref().unwrap_or("reoclo")
            ),
            k,
        ))
        .child(field("Streams", &t.traffic.streams.to_string(), k));
    col()
        .key("tunnel-facts")
        .gap(MD)
        .w_full()
        .child(field("Endpoint", t.endpoint(), k))
        .child(fields)
}

fn actions(t: &TunnelView, s: &State, k: &Kit) -> El {
    let mut actions = row().w_full().gap(SM).flex_wrap();
    if s.current_data() {
        if t.state == TunnelState::NeedsSignIn {
            actions = actions.child(
                Button::new("Sign in")
                    .on_press(|s: &mut State| s.request("POST", "/v1/auth/login", None))
                    .primary(k)
                    .key("tunnel-sign-in"),
            );
        } else if t.state.failure() || t.retry.is_some() {
            let path = format!("/v1/tunnels/{}/up", segment(&t.name));
            actions = actions.child(
                Button::new("Retry now")
                    .disabled(s.tunnel_pending(&t.name))
                    .on_press(move |s: &mut State| s.request("POST", &path, None))
                    .primary(k)
                    .key("retry-tunnel"),
            );
        }
        if t.kind == "tcp" {
            let (org, server, name, target) = (
                t.org.clone(),
                t.server_id.clone(),
                format!("{}-https", t.name),
                t.target.clone().unwrap_or_default(),
            );
            actions = actions.child(
                Button::new("Add HTTPS")
                    .on_press(move |s: &mut State| {
                        s.begin_https(&org, &server, &target, name.clone())
                    })
                    .secondary(k)
                    .key("https-tunnel"),
            );
        }
        if t.state.available()
            && let Some(url) = t.url.clone()
        {
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
    }
    let copy = t.endpoint().to_owned();
    actions = actions.child(
        Button::new("Copy endpoint")
            .on_press_tx(move |s: &mut State, tx| {
                tx.copy(&copy);
                s.notice = Some("Endpoint copied.".into());
            })
            .secondary(k)
            .key("copy-endpoint"),
    );
    let name = t.name.clone();
    actions = actions.child(
        Button::new("View logs")
            .on_press(move |s: &mut State| s.show_logs(&name))
            .secondary(k)
            .key("view-tunnel-logs"),
    );
    if s.current_data() {
        if t.kind != "local" {
            actions = actions.child(
                Button::new("Edit tunnel")
                    .on_press(|s: &mut State| s.edit_selected())
                    .secondary(k)
                    .key("edit-tunnel"),
            );
        }
        actions = actions.child(spacer()).child(
            Button::new("Remove")
                .on_press(|s: &mut State| s.modal = Some("remove".into()))
                .danger(k)
                .key("remove-tunnel"),
        );
    }
    actions
}
