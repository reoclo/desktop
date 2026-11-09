use super::{State, ViewState};
use crate::{
    control::segment,
    model::{ForegroundForward, TunnelState},
    theme::*,
};
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, glyphs};

/// What a status mark says. Each tone has its own shape, so color is never the only signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Success,
    Warning,
    Info,
    Danger,
    Ready,
    Inactive,
}

/// The four button ranks. Every action in the app uses one of these.
pub trait Rank {
    fn primary(self, k: &Kit) -> El;
    fn secondary(self, k: &Kit) -> El;
    /// A destructive action that opens a confirmation.
    fn danger(self, k: &Kit) -> El;
    /// The confirming button of a destructive dialog.
    fn destructive(self, k: &Kit) -> El;
}

impl Rank for Button {
    fn primary(self, k: &Kit) -> El {
        self.variant(ButtonVariant::Contrast)
            .control_size(ControlSize::Large)
            .build(k)
            .gradient(k.c.bg_contrast, k.c.bg_contrast)
    }

    fn secondary(self, k: &Kit) -> El {
        self.control_size(ControlSize::Large)
            .build(k)
            .border(RULE, k.c.border_strong)
    }

    fn danger(self, k: &Kit) -> El {
        self.variant(ButtonVariant::Danger)
            .control_size(ControlSize::Large)
            .build(k)
            .border(RULE, k.c.danger.fg)
    }

    fn destructive(self, k: &Kit) -> El {
        self.variant(ButtonVariant::Contrast)
            .control_size(ControlSize::Large)
            .build(k)
            .fill(k.c.danger.fg)
            .gradient(k.c.danger.fg, k.c.danger.fg)
    }
}

/// The Reoclo mark from assets/mark.svg, a pointy-top hexagon.
pub fn logo(height: f32, k: &Kit) -> El {
    let u = height / 50.0;
    rune::canvas(rune::Shape::new().fill(
        vec![
            [22.5 * u, 0.0],
            [45.0 * u, 12.5 * u],
            [45.0 * u, 37.5 * u],
            [22.5 * u, 50.0 * u],
            [0.0, 37.5 * u],
            [0.0, 12.5 * u],
        ],
        k.c.text_base,
    ))
    .size(45.0 * u, height)
    .a11y_hidden()
}

pub fn wordmark(k: &Kit) -> El {
    row()
        .gap(SM)
        .center_y()
        .child(logo(20.0, k))
        .child(text("Reoclo").font(WORDMARK).color(k.c.text_base))
}

pub fn spacer() -> El {
    el().grow(1.0)
}

pub fn title(name: &str, detail: Option<&str>, k: &Kit) -> El {
    let mut head = col()
        .gap(XS)
        .child(text(name).font(TITLE).color(k.c.text_base));
    if let Some(detail) = detail {
        head = head.child(text(detail).font(BODY).color(k.c.text_muted).wrap());
    }
    head
}

/// A label over a machine value, set in mono.
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

fn tone_color(tone: Tone, k: &Kit) -> rune::Color {
    match tone {
        Tone::Success => k.c.success.fg,
        Tone::Warning => k.c.warning.fg,
        Tone::Info => k.c.info.fg,
        Tone::Danger => k.c.danger.fg,
        Tone::Ready | Tone::Inactive => k.c.text_muted,
    }
}

fn mark(tone: Tone, k: &Kit) -> El {
    let color = tone_color(tone, k);
    let mark = match tone {
        Tone::Warning => {
            rune::canvas(rune::Shape::new().fill(vec![[XS, 0.0], [SM, SM], [0.0, SM]], color))
                .size(SM, SM)
        }
        Tone::Success => el().size(SM, SM).fill(color),
        Tone::Danger => el()
            .size(SM, SM)
            .rotate(std::f32::consts::FRAC_PI_4)
            .fill(color),
        Tone::Info | Tone::Ready => el().size(SM, SM).radius(XS).fill(color),
        Tone::Inactive => el().size(SM, SM).radius(XS).border(RULE, color),
    };
    mark.a11y_hidden()
}

/// A status mark with its text label.
pub fn indicator(tone: Tone, label: &str, k: &Kit) -> El {
    row()
        .gap(SM)
        .center_y()
        .min_w(0.0)
        .shrink()
        .child(mark(tone, k))
        .child(
            text(label)
                .font(LABEL)
                .color(tone_color(tone, k))
                .truncate()
                .min_w(0.0)
                .shrink()
                .help_when_truncated(label),
        )
}

pub fn status(state: TunnelState, k: &Kit) -> El {
    let tone = match state {
        TunnelState::Active => Tone::Success,
        TunnelState::Connecting | TunnelState::Reconnecting => Tone::Info,
        TunnelState::NeedsSignIn | TunnelState::QuotaReached | TunnelState::ServerOffline => {
            Tone::Warning
        }
        TunnelState::Failed
        | TunnelState::Forbidden
        | TunnelState::Expired
        | TunnelState::ClosedByOperator => Tone::Danger,
        TunnelState::Ready => Tone::Ready,
        TunnelState::Down | TunnelState::Unknown => Tone::Inactive,
    };
    indicator(tone, state.label(), k)
}

/// A failure the person should read, placed where it cannot be scrolled past.
pub fn alert(message: &str, k: &Kit) -> El {
    row()
        .role("alert")
        .w_full()
        .gap(SM)
        .child(mark(Tone::Danger, k).mt(XS + 2.0))
        .child(
            text(message)
                .font(BODY)
                .color(k.c.danger.fg)
                .wrap()
                .selectable(true)
                .grow(1.0)
                .min_w(0.0),
        )
}

/// Requests from agents, which stay blocked until someone answers.
pub fn approvals(s: &State, k: &Kit) -> Option<El> {
    let count = s.model.approvals.len();
    if count == 0 {
        return None;
    }
    let heading = format!(
        "{count} pending approval{}",
        if count == 1 { "" } else { "s" }
    );
    let mut list = col()
        .w_full()
        .gap(MD)
        .child(indicator(Tone::Warning, &heading, k));
    for approval in &s.model.approvals {
        let approve = format!("/v1/approvals/{}/approve", segment(&approval.id));
        let deny = format!("/v1/approvals/{}/deny", segment(&approval.id));
        let detail = col()
            .gap(XS)
            .min_w(0.0)
            .child(text(&approval.agent).font(BODY).color(k.c.text_base).wrap())
            .child(
                text(&approval.description)
                    .font(BODY)
                    .color(k.c.text_muted)
                    .wrap(),
            );
        let actions = row()
            .gap(SM)
            .child(
                Button::new("Approve")
                    .disabled(s.busy)
                    .on_press(move |s: &mut State| s.request("POST", &approve, None))
                    .primary(k)
                    .key(format!("approve-{}", approval.id)),
            )
            .child(
                Button::new("Deny")
                    .disabled(s.busy)
                    .on_press(move |s: &mut State| s.request("POST", &deny, None))
                    .secondary(k)
                    .key(format!("deny-{}", approval.id)),
            );
        list = list.child(if s.panel {
            col().w_full().gap(SM).child(detail).child(actions)
        } else {
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(detail.grow(1.0))
                .child(actions)
        });
    }
    Some(list)
}

fn forward_line(f: &ForegroundForward) -> String {
    let line = format!("{} \u{2192} {}", f.listen, f.target);
    if f.proto == "udp" {
        format!("{line} (udp)")
    } else {
        line
    }
}

fn ask_to_stop(s: &mut State, id: &str) {
    s.stop_target = Some(id.to_owned());
    s.modal = Some("stop-terminal".into());
}

/// Tunnels started with `reoclo tunnel` in a terminal, reported by the daemon.
pub fn terminal_tunnels(s: &State, k: &Kit) -> Option<El> {
    if s.model.foreground.is_empty() {
        return None;
    }
    let mut section = col().key("terminal-tunnels").w_full().gap(MD).child(
        text("Running in a terminal")
            .font(LABEL)
            .color(k.c.text_base),
    );
    for tunnel in &s.model.foreground {
        let mut forwards = col().w_full().min_w(0.0).gap(XS);
        for (index, forward) in tunnel.forwards.iter().enumerate() {
            let line = forward_line(forward);
            let mut entry = row().w_full().gap(MD).center_y().min_w(0.0).child(
                text(&line)
                    .font(MONO)
                    .color(k.c.text_muted)
                    .truncate()
                    .selectable(true)
                    .help_when_truncated(&line)
                    .min_w(0.0)
                    .shrink(),
            );
            if !s.panel && forward.kind == "forward" && forward.proto == "tcp" {
                let org = tunnel.org.clone();
                let server = tunnel.server.clone();
                let target = forward.target.clone();
                let name = format!("web-{}", target.rsplit(':').next().unwrap_or("tunnel"));
                entry = entry.child(
                    Button::new("Add HTTPS")
                        .control_size(ControlSize::Small)
                        .help("Reach this service at a local HTTPS address")
                        .on_press(move |s: &mut State| {
                            s.begin_https(&org, &server, &target, name.clone())
                        })
                        .build(k)
                        .border(RULE, k.c.border_strong)
                        .key(format!("https-terminal-{}-{index}", tunnel.id)),
                );
            }
            forwards = forwards.child(entry);
        }
        let id = tunnel.id.clone();
        let key = format!("stop-terminal-{}", tunnel.id);
        let name = row()
            .gap(SM)
            .center_y()
            .min_w(0.0)
            .shrink()
            .child(
                text(&tunnel.server)
                    .font(BODY)
                    .color(k.c.text_base)
                    .truncate()
                    .help_when_truncated(&tunnel.server)
                    .min_w(0.0)
                    .shrink(),
            )
            .child(text(&tunnel.org).font(HINT).color(k.c.text_muted));
        section = section.child(if s.panel {
            col()
                .w_full()
                .gap(XS)
                .child(
                    row()
                        .w_full()
                        .gap(SM)
                        .center_y()
                        .child(name.grow(1.0))
                        .child(status(tunnel.state, k))
                        .child(
                            IconButton::new(&glyphs::CLOSE)
                                .variant(IconButtonVariant::GhostMuted)
                                .label(format!("Stop {}", tunnel.server))
                                .disabled(s.busy)
                                .on_press(move |s: &mut State| ask_to_stop(s, &id))
                                .build(k)
                                .key(key),
                        ),
                )
                .child(forwards)
        } else {
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(XS)
                        .child(name)
                        .child(forwards),
                )
                .child(status(tunnel.state, k))
                .child(
                    Button::new("Stop")
                        .disabled(s.busy)
                        .on_press(move |s: &mut State| ask_to_stop(s, &id))
                        .danger(k)
                        .key(key),
                )
        });
    }
    Some(section)
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
        .gap(if compact { LG } else { XL });
    if !compact {
        body = body.child(title(s.page.name(), None, k));
    }
    body = body.child(
        col()
            .gap(SM)
            .child(text(heading).font(HEADING).color(k.c.text_base))
            .child(text(detail).font(BODY).color(k.c.text_muted).wrap()),
    );
    if let Some(code) = &s.model.auth.user_code {
        body = body.child(
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
        );
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
                    .key("open-sign-in"),
            );
        }
        actions = actions.child(
            Button::new("Copy device code")
                .on_press_tx(move |s: &mut State, tx| {
                    tx.copy(&code);
                    s.notice = Some("Device code copied.".into());
                })
                .secondary(k)
                .key("copy-device-code"),
        );
        body = body.child(actions);
        if let Some(url) = &s.model.auth.verification_uri {
            body = body.child(field("Continue in your browser", url, k));
        }
    }
    if let Some(label) = action
        && !(s.panel && s.phase == ViewState::Empty)
        && !(s.phase == ViewState::SignedOut && s.model.auth.user_code.is_some())
    {
        body = body.child(
            row().child(
                Button::new(label)
                    .on_press(move |s: &mut State| match s.phase {
                        ViewState::Empty => s.begin_add(),
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
    body
}
