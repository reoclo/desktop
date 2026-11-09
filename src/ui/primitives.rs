use super::{Page, State, ViewState};
use crate::{model::TunnelState, theme::*};
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn title(name: &str, detail: &str, k: &Kit) -> El {
    col()
        .gap(SM)
        .child(text(name).font(TITLE).color(k.c.text_base))
        .child(text(detail).font(BODY).color(k.c.text_muted).wrap())
}

pub fn field(label: &str, value: &str, k: &Kit) -> El {
    col()
        .gap(XS)
        .child(text(label).font(LABEL).color(k.c.text_muted))
        .child(
            text(value)
                .font(MONO)
                .color(k.c.text_base)
                .wrap()
                .selectable(true)
                .selection_color(k.selection()),
        )
}

pub fn status(state: TunnelState, k: &Kit) -> El {
    let (color, radius, angle, hollow) = match state {
        TunnelState::Active => (k.c.success.fg, 0.0, 0.0, false),
        TunnelState::Connecting | TunnelState::Reconnecting => (k.c.info.fg, XS, 0.0, false),
        TunnelState::NeedsSignIn | TunnelState::QuotaReached | TunnelState::ServerOffline => {
            (k.c.warning.fg, 0.0, std::f32::consts::FRAC_PI_4, false)
        }
        TunnelState::Failed
        | TunnelState::Forbidden
        | TunnelState::Expired
        | TunnelState::ClosedByOperator => (k.c.danger.fg, 0.0, std::f32::consts::FRAC_PI_4, false),
        TunnelState::Ready => (k.c.text_muted, XS, 0.0, false),
        TunnelState::Down | TunnelState::Unknown => (k.c.text_muted, XS, 0.0, true),
    };
    let warning = matches!(
        state,
        TunnelState::NeedsSignIn | TunnelState::QuotaReached | TunnelState::ServerOffline
    );
    let mark = if warning {
        rune::canvas(rune::Shape::new().fill(vec![[XS, 0.0], [SM, SM], [0.0, SM]], color))
            .size(SM, SM)
            .a11y_hidden()
    } else {
        el().size(SM, SM).radius(radius).rotate(angle).a11y_hidden()
    };
    row()
        .gap(SM)
        .center_y()
        .min_w(0.0)
        .shrink()
        .child(if warning {
            mark
        } else if hollow {
            mark.border(RULE, color)
        } else {
            mark.fill(color)
        })
        .child(
            text(state.label())
                .font(LABEL)
                .color(color)
                .truncate()
                .min_w(0.0)
                .shrink()
                .help_when_truncated(state.label()),
        )
}

pub fn data_state(s: &State, k: &Kit) -> El {
    if s.phase == ViewState::Loading {
        return super::skeleton::view(s, k);
    }
    let (heading, detail, action) = match s.phase {
        ViewState::Loading => ("Loading local state", "Reading the tunnel daemon.", None),
        ViewState::Empty => (
            "No local tunnels",
            "Add a tunnel to reach a service on one of your servers.",
            Some("Add tunnel"),
        ),
        ViewState::Offline => (
            "Daemon not running",
            "Start the local daemon to use your saved tunnels.",
            Some("Start daemon"),
        ),
        ViewState::SignedOut => (
            "Sign in to Reoclo",
            "Authorize this Mac in your browser. Tokens stay in the daemon.",
            Some("Sign in"),
        ),
        ViewState::Error => (
            "Could not load local state",
            s.error
                .as_deref()
                .unwrap_or("The daemon did not return a valid response."),
            Some("Retry"),
        ),
        ViewState::Loaded => ("", "", None),
    };
    let compact = s.panel || s.modal.is_some();
    let mut body = col()
        .key("data-state")
        .role("status")
        .w_full()
        .padding(if compact { LG } else { XL })
        .gap(XL);
    if !compact {
        body = body.child(title(s.page.name(), description(s.page), k));
    }
    body = body.child(
        col()
            .gap(SM)
            .child(text(heading).font(HEADING).color(k.c.text_base))
            .child(text(detail).font(BODY).color(k.c.text_muted).wrap()),
    );
    if s.phase == ViewState::Loading {
        for _ in 0..3 {
            body = body.child(
                el().h(CONTROL)
                    .w_full()
                    .fill(k.c.layer_02)
                    .radius(k.radius.sm),
            );
        }
    }
    if let Some(code) = &s.model.auth.user_code {
        body = body.child(field("Device code", code, k));
        let code = code.clone();
        body = body.child(
            Button::new("Copy device code")
                .on_press_tx(move |s: &mut State, tx| {
                    tx.copy(&code);
                    s.notice = Some("Device code copied.".into());
                })
                .build(k)
                .w(180.0)
                .key("copy-device-code"),
        );
        if let Some(url) = &s.model.auth.verification_uri {
            body = body.child(field("Continue in your browser", url, k));
            let url = s
                .model
                .auth
                .verification_uri_complete
                .as_ref()
                .unwrap_or(url)
                .clone();
            body = body.child(
                Button::new("Open sign-in page")
                    .variant(ButtonVariant::Contrast)
                    .on_press_tx(move |s: &mut State, tx| {
                        if let Err(error) = tx.open_url(&url) {
                            s.error = Some(error.to_string());
                        }
                    })
                    .build(k)
                    .w(180.0)
                    .key("open-sign-in"),
            );
        }
    }
    if let Some(label) = action
        && !(s.panel && s.phase == ViewState::Empty)
        && !(s.phase == ViewState::SignedOut && s.model.auth.user_code.is_some())
    {
        body = body.child(
            Button::new(label)
                .variant(ButtonVariant::Contrast)
                .on_press(move |s: &mut State| match s.phase {
                    ViewState::Empty => s.begin_add(),
                    ViewState::SignedOut => s.request("POST", "/v1/auth/login", None),
                    ViewState::Offline => s.modal = Some("start".into()),
                    ViewState::Error => s.request("GET", "/v1/tunnels", None),
                    ViewState::Loading | ViewState::Loaded => {}
                })
                .build(k)
                .w(180.0)
                .key("state-action"),
        );
    }
    body
}

pub fn description(page: Page) -> &'static str {
    match page {
        Page::Tunnels => "Your servers, reachable from this Mac.",
        Page::Hosts => "Local names for services on your servers.",
        Page::Certificates => "Trust for local HTTPS, scoped to Reoclo hostnames.",
        Page::Settings => "Preferences for this Mac and its tunnel daemon.",
    }
}
