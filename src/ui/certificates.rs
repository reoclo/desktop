use super::{State, primitives::*};
use crate::theme::*;
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ControlSize, Kit, TextInput};

pub fn view(s: &State, k: &Kit) -> El {
    let ca = s.model.status.ca.as_ref();
    let trusted = ca.is_some_and(|ca| ca.trusted);
    let exists = ca.is_some_and(|ca| ca.exists);
    let mut body = col()
        .w_full()
        .padding(XL)
        .gap(XL)
        .child(title("Certificates", None, k));
    if let Some(error) = s.error.as_ref().filter(|_| s.modal.is_none()) {
        body = body.child(alert(error, k));
    }
    let mut authority = col()
        .w_full()
        .gap(MD)
        .child(
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(
                    text("Local certificate authority")
                        .font(HEADING)
                        .color(k.c.text_base)
                        .grow(1.0),
                )
                .child(if trusted {
                    indicator(Tone::Success, "Trusted", k)
                } else if exists {
                    indicator(Tone::Warning, "Not trusted", k)
                } else {
                    indicator(Tone::Inactive, "Not created", k)
                }),
        )
        .child(
            text("The daemon keeps the CA key on this Mac. Its name constraints limit certificates to the suffixes listed below.")
                .font(BODY)
                .color(k.c.text_muted)
                .wrap()
                .max_w(640.0),
        );
    if let Some(ca) = ca {
        authority = authority.child(
            row()
                .gap(XXL)
                .mt(SM)
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
                .child(suffix_list(&ca.suffixes, s, k).grow(1.0)),
        );
    } else {
        authority = authority.child(
            text("This daemon does not report certificate status. Update the bundled CLI to manage local trust.")
                .font(BODY)
                .color(k.c.warning.fg)
                .wrap(),
        );
    }
    let stale = ca.is_some_and(|ca| ca.stale);
    let action = if stale {
        Button::new("Trust new CA")
            .disabled(s.busy)
            .on_press(|s: &mut State| s.modal = Some("trust".into()))
            .primary(k)
            .key("trust-ca")
    } else if trusted {
        Button::new("Untrust CA")
            .disabled(s.busy)
            .on_press(|s: &mut State| s.modal = Some("untrust".into()))
            .danger(k)
            .key("untrust-ca")
    } else {
        Button::new("Trust CA")
            .disabled(s.busy)
            .on_press(|s: &mut State| s.modal = Some("trust".into()))
            .primary(k)
            .key("trust-ca")
    };
    body = body.child(authority).child(row().child(action)).child(
        text(if trusted {
            "Command-line tools may also need NODE_EXTRA_CA_CERTS; get the file path with reoclo tunnel ca-path."
        } else {
            "Trust opens the system authentication prompt. Command-line tools may also need NODE_EXTRA_CA_CERTS; get the file path with reoclo tunnel ca-path."
        })
            .font(HINT)
            .color(k.c.text_muted)
            .wrap()
            .max_w(640.0),
    );
    if ca.is_some() {
        let ready = !s.busy && !s.trust_domain.text.trim().is_empty();
        body = body.child(
            col()
                .w_full()
                .gap(MD)
                .child(text("Trust another domain").font(HEADING).color(k.c.text_base))
                .child(
                    text("Let the CA sign for names under a domain you choose, such as internal or agents.localhost. macOS asks for your password.")
                        .font(BODY)
                        .color(k.c.text_muted)
                        .wrap()
                        .max_w(640.0),
                )
                .child(
                    row()
                        .gap(SM)
                        .center_y()
                        .child(
                            TextInput::new(&s.trust_domain)
                                .label("Domain to trust")
                                .placeholder("internal")
                                .width(Some(FORM_COLUMN))
                                .on_edit(|s: &mut State| &mut s.trust_domain)
                                .on_submit(|s: &mut State, _| trust_domain(s))
                                .build(k)
                                .border(RULE, k.c.border_strong)
                                .key("trust-domain-input"),
                        )
                        .child(
                            Button::new("Trust domain")
                                .disabled(!ready)
                                .on_press(trust_domain)
                                .secondary(k)
                                .key("trust-domain"),
                        ),
                ),
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

fn trust_domain(s: &mut State) {
    let domain = s.trust_domain.text.trim().to_ascii_lowercase();
    if !domain.is_empty() {
        s.request(
            "POST",
            "/v1/ca/trust",
            Some(serde_json::json!({ "domains": [domain] })),
        );
    }
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

/// Every suffix the CA may sign for. The built-in one stays; the ones you added can be removed.
fn suffix_list(suffixes: &[String], s: &State, k: &Kit) -> El {
    let mut list = col()
        .gap(XS)
        .child(text("Allowed suffixes").font(LABEL).color(k.c.text_muted));
    for suffix in suffixes {
        let mut line = row().gap(MD).center_y().child(
            text(suffix)
                .font(MONO)
                .color(k.c.text_base)
                .selectable(true),
        );
        if suffix != "reoclo.localhost" {
            let target = suffix.clone();
            line = line.child(
                Button::new("Remove")
                    .control_size(ControlSize::Small)
                    .disabled(s.busy)
                    .on_press(move |s: &mut State| {
                        s.suffix_target = Some(target.clone());
                        s.modal = Some("remove-suffix".into());
                    })
                    .build(k)
                    .border(RULE, k.c.border_strong)
                    .key(format!("remove-suffix-{suffix}")),
            );
        }
        list = list.child(line);
    }
    list
}
