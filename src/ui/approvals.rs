use super::{State, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, col, row, text};
use rune_kit::{Button, Kit};

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
    let mut list =
        col()
            .role("status")
            .w_full()
            .gap(MD)
            .child(indicator(Tone::Warning, &heading, k));
    for approval in &s.model.approvals {
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
        let mut metadata = row().gap(MD).center_y().child(indicator(
            if remote {
                Tone::Warning
            } else {
                Tone::Inactive
            },
            &label,
            k,
        ));
        if let Some(expiry) = approval.expires_at {
            let now = s.clock_epoch + time::Duration::seconds_f64(s.clock_seconds.unwrap_or(0.0));
            let seconds = (expiry - now).whole_seconds().max(0);
            metadata = metadata.child(
                text(if seconds == 0 {
                    "Expired".into()
                } else {
                    format!("Expires in {} min", (seconds + 59) / 60)
                })
                .font(HINT)
                .color(k.c.text_muted),
            );
        }
        let detail = col()
            .gap(XS)
            .min_w(0.0)
            .child(text(&approval.agent).font(BODY).color(k.c.text_base).wrap())
            .child(metadata)
            .child(description(&approval.description, k));
        let actions = row()
            .gap(SM)
            .child(
                Button::new("Deny")
                    .disabled(s.approval_pending(&approval.id))
                    .on_press(move |s: &mut State| s.request("POST", &deny, None))
                    .secondary(k)
                    .key(format!("deny-{}", approval.id)),
            )
            .child(
                Button::new("Approve")
                    .disabled(s.approval_pending(&approval.id))
                    .on_press(move |s: &mut State| s.request("POST", &approve, None))
                    .primary(k)
                    .key(format!("approve-{}", approval.id)),
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

fn description(value: &str, k: &Kit) -> El {
    let mut line = row().gap(XS).flex_wrap().w_full();
    for word in value.split_whitespace() {
        let machine =
            word.contains('.') || word.contains(':') || word.chars().all(|c| c.is_ascii_digit());
        line = line.child(
            text(word)
                .font(if machine { MONO } else { BODY })
                .color(k.c.text_muted),
        );
    }
    line
}
