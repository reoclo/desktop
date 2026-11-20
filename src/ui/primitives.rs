use crate::{model::TunnelState, theme::*};
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

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

/// A one-point horizontal rule between two regions.
pub fn rule(k: &Kit) -> El {
    el().h(RULE).w_full().fill(k.c.border_base)
}

pub fn title(name: &str, detail: Option<&str>, k: &Kit) -> El {
    let mut head = col()
        .gap(XS)
        .child(text(name).role("heading").font(TITLE).color(k.c.text_base));
    if let Some(detail) = detail {
        head = head.child(text(detail).font(BODY).color(k.c.text_muted).wrap());
    }
    head
}

/// A label over a machine value, set in mono.
pub fn field(label: &str, value: &str, k: &Kit) -> El {
    col()
        .w_full()
        .min_w(0.0)
        .gap(XS)
        .child(text(label).font(LABEL).color(k.c.text_muted))
        .child(
            text(value)
                .font(
                    if matches!(
                        value,
                        "This Mac" | "Not listening" | "Not set" | "Not available"
                    ) {
                        HINT
                    } else {
                        MONO
                    },
                )
                .color(k.c.text_base)
                .truncate_middle()
                .help_when_truncated(value)
                .w_full()
                .min_w(0.0)
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
    indicator(state_tone(state), state.label(), k)
}

/// A state's mark alone, for one-line rows. Its label is the accessible name and the tooltip.
pub fn state_mark(state: TunnelState, k: &Kit) -> El {
    row()
        .role("img")
        .label(state.label())
        .help(state.label())
        .child(mark(state_tone(state), k))
}

fn state_tone(state: TunnelState) -> Tone {
    match state {
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
    }
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

pub use super::approvals::approvals;
pub use super::data_state::{data_state, device_code};
