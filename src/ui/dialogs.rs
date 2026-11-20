mod actions;
use super::{State, modal::Dialog, primitives::*};
use crate::{control::segment, system::SystemAction, theme::*};
use rune::{El, col, row, text};
use rune_kit::{Button, Kit};

pub fn confirmations(s: &State, k: &Kit, layer: &mut Vec<El>) {
    let Some(modal) = s.modal.as_deref() else {
        return;
    };
    let link_text = s.link_description();
    let remove_title = s
        .selected()
        .map(|t| format!("Remove {}?", t.name))
        .unwrap_or_else(|| "Remove tunnel?".into());
    let remove_body = s.selected().map(|t| match t.hostname.as_deref() {
        Some(host) => format!("The tunnel closes, and its saved configuration and the hostname {host} are removed."),
        None => "The tunnel closes and its saved configuration is removed.".into(),
    }).unwrap_or_default();
    let stop_title = s
        .model
        .foreground
        .iter()
        .find(|t| Some(&t.id) == s.stop_target.as_ref())
        .map(|t| format!("Stop {}?", t.server))
        .unwrap_or_else(|| "Stop this tunnel?".into());
    let (title, description, action, destructive) = match modal {
        "open-link" => (
            "Open this link?",
            match s.link.as_ref().map(|p| p.link.kind) {
                Some(crate::link::LinkKind::Tcp) => {
                    "This link opens a tunnel and copies its local address."
                }
                _ => "This link opens a tunnel and then its address in your browser.",
            },
            "Continue",
            false,
        ),
        "remove" => (
            remove_title.as_str(),
            remove_body.as_str(),
            "Remove tunnel",
            true,
        ),
        "stop-terminal" => (
            stop_title.as_str(),
            "The reoclo process running in your terminal exits and its ports close.",
            "Stop tunnel",
            true,
        ),
        "discard" => (
            "Discard changes?",
            "The tunnel has not been saved.",
            "Discard changes",
            true,
        ),
        "start" => (
            "Start local daemon?",
            "Reoclo will install its background service with the reoclo CLI on this Mac. It starts now and every time you log in. Your saved tunnels remain owned by the daemon.",
            "Start daemon",
            false,
        ),
        "remove-suffix" => (
            "Remove this domain?",
            "Names under it stop getting certificates. The CA is issued again without it, and macOS asks for your password to trust the new one. Remove tunnels and hostnames under it first.",
            "Remove domain",
            true,
        ),
        "relay-on" => (
            "Turn on the port 443 relay?",
            "HTTPS addresses drop the :8443. macOS asks for administrator approval to reinstall the hosts helper. The relay waits while another program, such as a local Caddy, answers on port 443.",
            "Turn on relay",
            false,
        ),
        "relay-off" => (
            "Turn off the port 443 relay?",
            "HTTPS addresses include the daemon's port again. macOS asks for administrator approval to reinstall the hosts helper.",
            "Turn off relay",
            false,
        ),
        "setup" => (
            "Set up managed hosts?",
            "macOS will ask for administrator approval to install the hosts helper. It only manages Reoclo's loopback entries.",
            "Set up helper",
            false,
        ),
        "trust" => (
            "Trust local certificate authority?",
            "The daemon will ask macOS to trust its local CA for Reoclo hostnames.",
            "Trust CA",
            false,
        ),
        "untrust" => (
            "Remove CA trust?",
            "Local HTTPS tunnels will no longer be trusted by browsers.",
            "Untrust CA",
            true,
        ),
        _ => return,
    };
    let modal = modal.to_owned();
    let mut body = col()
        .w_full()
        .gap(LG)
        .padding(LG)
        .child(text(description).font(BODY).color(k.c.text_muted).wrap());
    if modal == "open-link"
        && let Some(pending) = &s.link
    {
        let link = &pending.link;
        body = body
            .description(link_text.unwrap_or_default())
            .child(
                col()
                    .equal_columns(2)
                    .gap(LG)
                    .w_full()
                    .child(field("Organization", &link.org, k))
                    .child(field("Server", &link.server, k))
                    .child(field("Target", &link.target(), k))
                    .child(field(
                        "Opens as",
                        match link.kind {
                            crate::link::LinkKind::Https => "HTTPS",
                            crate::link::LinkKind::Tcp => "Local TCP port",
                        },
                        k,
                    )),
            )
            .child(
                text("Only continue if you trust where the link came from.")
                    .font(BODY)
                    .color(k.c.text_muted)
                    .wrap(),
            );
    }
    if let Some(error) = &s.error {
        body = body.child(alert(error, k));
    }
    let confirm = Button::new(action)
        .disabled(s.busy)
        .on_press(move |s: &mut State| match modal.as_str() {
            "remove" => {
                if let Some(t) = s.selected() {
                    s.request("DELETE", &format!("/v1/tunnels/{}", segment(&t.name)), None);
                }
            }
            "stop-terminal" => {
                if let Some(id) = s.stop_target.clone() {
                    s.request(
                        "POST",
                        &format!("/v1/foreground/{}/stop", segment(&id)),
                        None,
                    );
                }
            }
            "discard" => {
                s.modal = None;
                s.return_modal = None;
                s.open_select = None;
            }
            "trust" => s.request("POST", "/v1/ca/trust", None),
            "untrust" => s.request("POST", "/v1/ca/untrust", None),
            "start" => s.run_system(SystemAction::InstallDaemon),
            "open-link" => s.confirm_link(),
            "setup" => {
                let setup = s.helper_setup(s.relay_enabled());
                s.run_system(SystemAction::SetupHosts(setup));
            }
            "remove-suffix" => {
                if let Some(target) = s.suffix_target.clone() {
                    let rest: Vec<String> = s
                        .settings
                        .ca_suffixes
                        .iter()
                        .filter(|suffix| **suffix != target)
                        .cloned()
                        .collect();
                    s.request(
                        "PATCH",
                        "/v1/settings",
                        Some(serde_json::json!({ "ca_suffixes": rest })),
                    );
                }
            }
            "relay-on" | "relay-off" => {
                let setup = s.helper_setup(modal == "relay-on");
                s.run_system(SystemAction::SetupHosts(setup));
            }
            _ => {}
        });
    body = body.child(
        row()
            .w_full()
            .gap(SM)
            .child(spacer())
            .child(
                Button::new("Cancel")
                    .on_press(|s: &mut State| s.cancel_confirmation())
                    .secondary(k)
                    .key("cancel-confirmation"),
            )
            .child(
                if destructive {
                    confirm.destructive(k)
                } else {
                    confirm.primary(k)
                }
                .key("confirm-action"),
            ),
    );
    Dialog::new("confirmation", title)
        .width(if s.panel { 308.0 } else { 480.0 })
        .on_close(|s: &mut State| s.cancel_confirmation())
        .child(body)
        .build(k, layer);
}
