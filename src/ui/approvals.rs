use super::{State, primitives::*};
use crate::{control::segment, model::Approval, theme::*};
use rune::{El, Span, col, rich, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

impl State {
    /// Requests still waiting for an answer. The daemon drops a request when it expires and
    /// refuses to decide it after that, so an expired request leaves the list here too.
    pub fn pending_approvals(&self) -> impl Iterator<Item = &Approval> {
        let now = self.approval_clock();
        self.model
            .approvals
            .iter()
            .filter(move |a| a.expires_at.is_none_or(|expiry| expiry > now))
    }

    fn approval_clock(&self) -> time::OffsetDateTime {
        self.clock_epoch + time::Duration::seconds_f64(self.clock_seconds.unwrap_or(0.0))
    }
}

/// Requests from agents, which stay blocked until someone answers.
pub fn approvals(s: &State, k: &Kit) -> Option<El> {
    let pending: Vec<_> = s.pending_approvals().collect();
    if pending.is_empty() {
        return None;
    }
    let count = pending.len();
    let heading = format!(
        "{count} pending approval{}",
        if count == 1 { "" } else { "s" }
    );
    let mut list =
        col()
            .role("status")
            .w_full()
            .gap(MD)
            .child(indicator(Tone::Warning, &heading, k));
    for approval in pending {
        let approve = format!("/v1/approvals/{}/approve", segment(&approval.id));
        let deny = format!("/v1/approvals/{}/deny", segment(&approval.id));
        let words = approval
            .action
            .split(|c: char| {
                matches!(c, '_' | '-' | '.' | '\u{2013}' | '\u{2014}') || c.is_whitespace()
            })
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let mut characters = words.chars();
        let label: String = match characters.next() {
            Some(first) => first.to_uppercase().chain(characters).collect(),
            None => "Review request".into(),
        };
        let remote =
            words.contains("public") || words.contains("tailnet") || words.contains("reverse");
        let action = indicator(
            if remote {
                Tone::Warning
            } else {
                Tone::Inactive
            },
            &label,
            k,
        );
        let expiry = approval.expires_at.map(|expiry| {
            let seconds = (expiry - s.approval_clock()).whole_seconds();
            text(format!("Expires in {} min", (seconds + 59) / 60))
                .font(HINT)
                .color(k.c.text_muted)
        });
        let deny = Button::new("Deny")
            .disabled(s.approval_pending(&approval.id))
            .on_press(move |s: &mut State| s.request("POST", &deny, None));
        let approve = Button::new("Approve")
            .disabled(s.approval_pending(&approval.id))
            .on_press(move |s: &mut State| s.request("POST", &approve, None));
        // The panel keeps each request to its action line, so its buttons take the small size.
        let (deny, approve) = if s.panel {
            (
                deny.control_size(ControlSize::Small)
                    .build(k)
                    .border(RULE, k.c.border_strong),
                approve
                    .variant(ButtonVariant::Contrast)
                    .control_size(ControlSize::Small)
                    .build(k)
                    .gradient(k.c.bg_contrast, k.c.bg_contrast),
            )
        } else {
            (deny.secondary(k), approve.primary(k))
        };
        let actions = row()
            .gap(SM)
            .child(deny.key(format!("deny-{}", approval.id)))
            .child(approve.key(format!("approve-{}", approval.id)));
        list = list.child(if s.panel {
            let asked = row().w_full().gap(MD).pl(LG).child(
                text(&approval.agent)
                    .font(HINT)
                    .color(k.c.text_muted)
                    .truncate()
                    .help_when_truncated(&approval.agent)
                    .min_w(0.0)
                    .shrink(),
            );
            col()
                .w_full()
                .gap(XS)
                .child(
                    row()
                        .w_full()
                        .gap(SM)
                        .center_y()
                        .child(action.grow(1.0))
                        .child(actions),
                )
                .child(description(&approval.description, k).pl(LG))
                .child(asked.children(expiry))
        } else {
            let detail = col()
                .gap(XS)
                .min_w(0.0)
                .child(text(&approval.agent).font(BODY).color(k.c.text_base).wrap())
                .child(row().gap(MD).center_y().child(action).children(expiry))
                .child(description(&approval.description, k));
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

/// The request as one sentence with its machine values in mono. A value stays on the line of
/// the word before it, so "port 8080" never splits across lines.
fn description(value: &str, k: &Kit) -> El {
    let mut content = String::new();
    let mut spans = Vec::new();
    for word in value.split_whitespace() {
        let machine =
            word.contains('.') || word.contains(':') || word.chars().all(|c| c.is_ascii_digit());
        if !content.is_empty() {
            content.push(if machine { '\u{a0}' } else { ' ' });
        }
        let start = content.len();
        content.push_str(word);
        if machine {
            spans.push(Span::new(start, content.len()).mono().size(MONO.size));
        }
    }
    rich(content, spans)
        .font(BODY)
        .color(k.c.text_muted)
        .wrap()
        .w_full()
        .min_w(0.0)
}
