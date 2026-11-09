use super::{State, primitives::*};
use crate::theme::*;
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(s: &State, k: &Kit) -> El {
    let ca = s.model.status.ca.as_ref();
    let trusted = ca.is_some_and(|ca| ca.trusted);
    let exists = ca.is_some_and(|ca| ca.exists);
    let mut body = col().w_full().padding(XL).gap(XL)
        .child(title("Certificates", "Trust for local HTTPS, scoped to Reoclo hostnames.", k))
        .child(row().w_full().gap(LG).center_y()
            .child(text("Local certificate authority").font(HEADING).color(k.c.text_base).grow(1.0))
            .child(text(if trusted { "Trusted" } else if exists { "Not trusted" } else { "Not created" })
                .font(LABEL).color(if trusted { k.c.success.fg } else { k.c.warning.fg })))
        .child(text("The daemon keeps the CA key on this Mac. Its name constraints limit certificates to the suffixes listed below.")
            .font(BODY).color(k.c.text_muted).wrap().max_w(640.0))
        .child(el().w_full().h(RULE).fill(k.c.border_base));
    if let Some(ca) = ca {
        body = body.child(
            row()
                .gap(XXL)
                .child(
                    field(
                        "Expires",
                        &ca.not_after
                            .as_deref()
                            .map(expiry)
                            .unwrap_or_else(|| "Not available".into()),
                        k,
                    )
                    .grow(1.0),
                )
                .child(
                    field(
                        "Allowed suffixes",
                        &if ca.suffixes.is_empty() {
                            "Not configured".into()
                        } else {
                            ca.suffixes.join("\n")
                        },
                        k,
                    )
                    .grow(1.0),
                ),
        );
    } else {
        body = body.child(text("This daemon does not report certificate status. Update the bundled CLI to manage local trust.")
            .font(BODY).color(k.c.warning.fg).wrap());
    }
    body = body.child(row().gap(SM)
        .child(Button::new("Trust CA").variant(ButtonVariant::Contrast).disabled(trusted || s.busy)
            .on_press(|s: &mut State| s.modal = Some("trust".into())).build(k).key("trust-ca"))
        .child(Button::new("Untrust CA").variant(ButtonVariant::Danger).disabled(!trusted || s.busy)
            .on_press(|s: &mut State| s.modal = Some("untrust".into())).build(k).key("untrust-ca")))
        .child(text("Trust opens the system authentication prompt. Command-line tools may also need NODE_EXTRA_CA_CERTS; get the file path with reoclo tunnel ca-path.")
            .font(BODY).color(k.c.text_muted).wrap().max_w(640.0));
    if let Some(message) = s.error.as_ref().or(s.notice.as_ref()) {
        body = body.child(
            text(message)
                .role("status")
                .font(BODY)
                .color(k.c.text_base)
                .wrap(),
        );
    }
    k.scroll_bar(
        col()
            .key("certificates-scroll")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    )
}

fn expiry(value: &str) -> String {
    let date = value.split('T').next().unwrap_or(value);
    let parts: Vec<_> = date.split('-').collect();
    if let [year, month, day] = parts.as_slice() {
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        if let Some(month) = month
            .parse::<usize>()
            .ok()
            .and_then(|m| m.checked_sub(1))
            .and_then(|i| months.get(i))
        {
            return format!("{} {month} {year}", day.trim_start_matches('0'));
        }
    }
    "Not available".into()
}
