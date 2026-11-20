use super::{State, ViewState, primitives::*};
use crate::theme::*;
use rune::{El, col, row, text};
use rune_kit::{Button, Kit};

/// The sign-in code, the buttons that use it and the page to finish in. `prefix` keeps keys
/// unique when this shows over another copy of itself.
pub fn device_code(s: &State, k: &Kit, prefix: &str) -> Vec<El> {
    let Some(code) = &s.model.auth.user_code else {
        return Vec::new();
    };
    let mut items = vec![
        col()
            .gap(XS)
            .child(text("Device code").font(LABEL).color(k.c.text_muted))
            .child(
                text(code)
                    .font(CODE)
                    .color(k.c.text_base)
                    .selectable(true)
                    .selection_color(k.selection()),
            ),
    ];
    let code = code.clone();
    let mut actions = row().gap(SM);
    if let Some(url) = s.model.auth.verification_uri_complete.as_ref().or(s
        .model
        .auth
        .verification_uri
        .as_ref())
    {
        let url = url.clone();
        actions = actions.child(
            Button::new("Open sign-in page")
                .on_press_tx(move |s: &mut State, tx| {
                    if let Err(error) = tx.open_url(&url) {
                        s.error = Some(error.to_string());
                    }
                })
                .primary(k)
                .key(format!("{prefix}open-sign-in")),
        );
    }
    actions = actions.child(
        Button::new("Copy device code")
            .on_press_tx(move |s: &mut State, tx| {
                tx.copy(&code);
                s.notice = Some("Device code copied.".into());
            })
            .secondary(k)
            .key(format!("{prefix}copy-device-code")),
    );
    items.push(actions);
    if let Some(url) = &s.model.auth.verification_uri {
        items.push(field("Continue in your browser", url, k));
    }
    items
}

pub fn data_state(s: &State, k: &Kit) -> El {
    if s.phase == ViewState::Loading {
        return super::skeleton::view(s, k);
    }
    let mut body = col()
        .w_full()
        .max_w(CONTENT_MAX)
        .padding(if s.compact || s.panel { LG } else { XL })
        .gap(LG);
    if !s.panel && s.modal.is_none() {
        body = body.child(title(s.page.name(), None, k));
    }
    body.child(phase_alert(s, k))
}

pub fn phase_alert(s: &State, k: &Kit) -> El {
    let (heading, detail, action) = match s.phase {
        ViewState::Loading => ("Loading local state", "Reading the tunnel daemon.", None),
        ViewState::Empty => (
            "No saved tunnels",
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
    let mut body = col()
        .key("data-state")
        .role(
            if matches!(s.phase, ViewState::Offline | ViewState::Error) {
                "alert"
            } else {
                "status"
            },
        )
        .w_full()
        .gap(MD);
    body = body.child(
        col()
            .gap(SM)
            .child(
                text(heading)
                    .role("heading")
                    .font(HEADING)
                    .color(k.c.text_base),
            )
            .child(text(detail).font(BODY).color(k.c.text_muted).wrap()),
    );
    for item in device_code(s, k, "") {
        body = body.child(item);
    }
    if let Some(label) = action
        && !(s.phase == ViewState::SignedOut && s.model.auth.user_code.is_some())
    {
        body = body.child(
            row().child(
                Button::new(label)
                    .on_press_tx(move |s: &mut State, tx| match s.phase {
                        ViewState::Empty => {
                            s.begin_add();
                            if s.panel
                                && let Some(handle) = &s.window_handle
                            {
                                handle.update(|s, _| {
                                    s.page = super::Page::Tunnels;
                                    s.begin_add();
                                });
                                handle.set_dock_icon(true);
                                handle.open();
                                tx.close_window();
                            }
                        }
                        ViewState::SignedOut => s.request("POST", "/v1/auth/login", None),
                        ViewState::Offline => s.modal = Some("start".into()),
                        ViewState::Error => s.request("GET", "/v1/tunnels", None),
                        ViewState::Loading | ViewState::Loaded => {}
                    })
                    .primary(k)
                    .key("state-action"),
            ),
        );
    }
    if s.phase == ViewState::Offline {
        let socket = s.socket_path.display().to_string();
        let details = format!("Daemon offline. Socket: {socket}");
        body = body.child(field("Socket", &socket, k)).child(
            row().child(
                Button::new("Copy details")
                    .on_press_tx(move |_: &mut State, tx| tx.copy(&details))
                    .secondary(k)
                    .key("copy-offline-details"),
            ),
        );
    }
    if s.phase == ViewState::Error
        && let Some(tunnels) = s.socket_path.parent().and_then(std::path::Path::parent)
    {
        let log = tunnels.join("tunneld.log");
        let path = log.display().to_string();
        body = body.child(field("Daemon log", &path, k)).child(
            row().child(
                Button::new("Show log in Finder")
                    .on_press(move |s: &mut State| {
                        s.run_system(crate::system::SystemAction::ShowLog(log.clone()));
                    })
                    .secondary(k)
                    .key("show-daemon-log"),
            ),
        );
    }
    body
}
