use super::{State, ViewState, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{
    Button, Combobox, ComboboxOption, Dialog, Kit, Select, SelectAppearance, TextInput,
};
pub(super) mod actions;
mod controls;
use actions::{hostname, save};
use controls::{Picker, label, options, picker};

pub fn sheet(s: &State, k: &Kit, layer: &mut Vec<El>) {
    if !matches!(s.modal.as_deref(), Some("add" | "edit")) {
        return;
    }
    let mut selects = Vec::new();
    let editing = s.modal.as_deref() == Some("edit");
    let orgs: Vec<_> = s.orgs.iter().map(|o| o.slug.clone()).collect();
    let servers: Vec<_> = s.servers.iter().map(|o| o.slug.clone()).collect();
    let selected_server = s
        .servers
        .iter()
        .find(|o| o.id == s.server)
        .map(|o| o.slug.as_str())
        .unwrap_or("");
    let ports: Vec<_> = s
        .ports
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let mut option = ComboboxOption::new(
                index.to_string(),
                format!("{} {}", p.port, p.protocol.to_ascii_uppercase()),
            );
            option.detail = format!("{} / {}", p.bind_address, p.process);
            option
        })
        .collect();
    let org = picker(
        Picker {
            key: "org",
            label: "Organization",
            placeholder: "Choose an organization",
            search: "Search organizations",
            options: &options(orgs),
            current: Some(s.org.as_str()).filter(|v| !v.is_empty()),
            disabled: false,
        },
        &s.org_picker,
        |s: &mut State| &mut s.org_picker,
        |s: &mut State, v, _| {
            s.org = v.into();
            s.server.clear();
            s.servers.clear();
            s.ports.clear();
            s.selected_port = None;
            s.close_pickers();
            s.request("GET", &format!("/v1/orgs/{}/servers", segment(v)), None);
        },
        k,
        &mut selects,
    );
    let server = picker(
        Picker {
            key: "server",
            label: "Server",
            placeholder: "Choose a server",
            search: "Search servers",
            options: &options(servers),
            current: Some(selected_server).filter(|v| !v.is_empty()),
            disabled: s.org.is_empty(),
        },
        &s.server_picker,
        |s: &mut State| &mut s.server_picker,
        |s: &mut State, v, _| {
            if let Some(server) = s.servers.iter().find(|o| o.slug == v) {
                s.server = server.id.clone();
                s.ports.clear();
                s.selected_port = None;
                s.close_pickers();
                s.request(
                    "GET",
                    &format!(
                        "/v1/orgs/{}/servers/{}/ports",
                        segment(&s.org),
                        segment(&s.server)
                    ),
                    None,
                );
            }
        },
        k,
        &mut selects,
    );
    let port = Combobox::new("port", &s.port_picker, &ports, |s: &mut State| {
        &mut s.port_picker
    })
    .label("Port on the server")
    .current(s.selected_port.as_deref())
    .width(FORM_COLUMN)
    .trigger(
        Select::new::<&str>("port-value", &[])
            .label("Port on the server")
            .appearance(SelectAppearance::Field)
            .width(FORM_COLUMN)
            .current(
                ports
                    .iter()
                    .find(|option| Some(&option.id) == s.selected_port.as_ref())
                    .map(|option| option.label.as_str()),
            )
            .placeholder("Choose a scanned port")
            .disabled(s.server.is_empty() || ports.is_empty())
            .trigger(k)
            .h(FIELD_HEIGHT)
            .border(RULE, k.c.border_strong),
    )
    .placeholder("Choose a scanned port")
    .search_placeholder("Search port, bind address or process")
    .disabled(s.server.is_empty() || ports.is_empty())
    .on_choose(|s: &mut State, v, _| {
        if let Some(port) = v.parse::<usize>().ok().and_then(|index| s.ports.get(index)) {
            s.target = rune::TextEdit::new(format!("127.0.0.1:{}", port.port));
            s.selected_port = Some(v.into());
            s.close_pickers();
        }
    })
    .build(k, &mut selects);
    let current_kind = s.kind.to_ascii_uppercase();
    let kind = picker(
        Picker {
            key: "kind",
            label: "Kind",
            placeholder: "Choose a kind",
            search: "Search kinds",
            options: &options(["HTTPS", "TCP", "UDP", "REVERSE"].map(String::from)),
            current: Some(current_kind.as_str()),
            disabled: false,
        },
        &s.kind_picker,
        |s: &mut State| &mut s.kind_picker,
        |s: &mut State, v, _| {
            s.kind = v.to_ascii_lowercase();
            s.close_pickers();
        },
        k,
        &mut selects,
    );
    let via = picker(
        Picker {
            key: "via",
            label: "Transport",
            placeholder: "Choose a transport",
            search: "Search transports",
            options: &options(["reoclo", "tailscale", "auto"].map(String::from)),
            current: Some(s.via.as_str()),
            disabled: false,
        },
        &s.via_picker,
        |s: &mut State| &mut s.via_picker,
        |s: &mut State, v, _| {
            s.via = v.into();
            s.close_pickers();
        },
        k,
        &mut selects,
    );
    let pair = |left: El, right: El| row().w_full().gap(LG).child(left).child(right);
    let name = TextInput::new(&s.name)
        .width(Some(FORM_COLUMN))
        .label("Tunnel name")
        .placeholder("api")
        .disabled(editing)
        .on_edit(|s: &mut State| &mut s.name)
        .on_submit(|s: &mut State, _| save(s))
        .build(k)
        .border(RULE, k.c.border_strong)
        .key("tunnel-name");
    let target = TextInput::new(&s.target)
        .width(Some(FORM_COLUMN))
        .label("Target")
        .placeholder("127.0.0.1:3000")
        .on_edit(|s: &mut State| &mut s.target)
        .on_submit(|s: &mut State, _| save(s))
        .build(k)
        .border(RULE, k.c.border_strong)
        .key("target");
    let domains = s.trusted_domains();
    let (fourth, fifth) = if s.kind == "https" {
        let current_domain = if s.domain.is_empty() {
            "reoclo.localhost".to_owned()
        } else {
            s.domain.clone()
        };
        let domain = picker(
            Picker {
                key: "domain",
                label: "Domain",
                placeholder: "Choose a trusted domain",
                search: "Search trusted domains",
                options: &options(domains),
                current: Some(current_domain.as_str()),
                disabled: false,
            },
            &s.domain_picker,
            |s: &mut State| &mut s.domain_picker,
            |s: &mut State, v, _| {
                s.domain = if v == "reoclo.localhost" {
                    String::new()
                } else {
                    v.into()
                };
                s.close_pickers();
            },
            k,
            &mut selects,
        );
        let subdomain = TextInput::new(&s.hostname)
            .width(Some(FORM_COLUMN))
            .label("Subdomain")
            .placeholder(
                hostname(s)
                    .strip_suffix(&format!(".{current_domain}"))
                    .unwrap_or_default()
                    .to_owned(),
            )
            .on_edit(|s: &mut State| &mut s.hostname)
            .on_submit(|s: &mut State, _| save(s))
            .build(k)
            .border(RULE, k.c.border_strong)
            .key("hostname");
        (
            pair(
                label("Subdomain (optional)", subdomain, k),
                label("Domain", domain, k),
            ),
            Some(pair(label("Transport", via, k), el().w(FORM_COLUMN))),
        )
    } else {
        (
            pair(
                label("Transport", via, k),
                label(
                    if s.kind == "reverse" {
                        "Remote port on server"
                    } else {
                        "Local port"
                    },
                    TextInput::new(&s.local_port)
                        .width(Some(FORM_COLUMN))
                        .label("Port")
                        .placeholder("15432")
                        .on_edit(|s: &mut State| &mut s.local_port)
                        .on_submit(|s: &mut State, _| save(s))
                        .build(k)
                        .border(RULE, k.c.border_strong)
                        .key("local-port"),
                    k,
                ),
            ),
            None,
        )
    };
    let mut fields = col()
        .w_full()
        .gap(LG)
        .padding(LG)
        .child(pair(label("Tunnel name", name, k), label("Kind", kind, k)))
        .child(pair(
            label("Organization", org, k),
            label("Server", server, k),
        ))
        .child(pair(
            label("Port on the server", port, k),
            label(
                if s.kind == "reverse" {
                    "Local target on this Mac"
                } else {
                    "Target"
                },
                target,
                k,
            ),
        ))
        .child(fourth);
    if let Some(fifth) = fifth {
        fields = fields.child(fifth);
    }
    if s.via != "reoclo" {
        fields = fields.child(text("Tailscale needs a reachable bind address. A service bound only to server loopback requires the Reoclo relay.")
            .font(BODY).color(k.c.warning.fg).wrap());
    }
    let content = if matches!(s.phase, ViewState::Loaded | ViewState::Empty) {
        fields
    } else {
        data_state(s, k)
    };
    let actions = row()
        .w_full()
        .gap(SM)
        .padding(LG)
        .child(spacer())
        .child(
            Button::new("Cancel")
                .on_press(|s: &mut State| s.confirm_discard())
                .secondary(k)
                .key("cancel-form"),
        )
        .child(
            Button::new(if editing {
                "Save changes"
            } else {
                "Save tunnel"
            })
            .disabled(s.busy || !matches!(s.phase, ViewState::Loaded | ViewState::Empty))
            .on_press(|s: &mut State| save(s))
            .primary(k)
            .key("save-tunnel"),
        );
    let preview = s.kind == "https" && matches!(s.phase, ViewState::Loaded | ViewState::Empty);
    let mut dialog = Dialog::new(
        "tunnel-sheet",
        if editing { "Edit tunnel" } else { "Add tunnel" },
    )
    .open(true)
    .width(560.0)
    .fit()
    .click_outside(false)
    .on_close(|s: &mut State| s.confirm_discard())
    .child(
        k.scroll_bar(
            col()
                .w_full()
                .h(if s.kind == "https" { 380.0 } else { 304.0 })
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .scroll_edge_fade(LG)
                .scroll_margin(LG)
                .key("form-scroll")
                .child(content),
        ),
    );
    // Outside the scrolling body, so it can never be scrolled past or push the fields up.
    let form_shown = matches!(s.phase, ViewState::Loaded | ViewState::Empty);
    if form_shown && s.kind == "https" && !s.ca_ready() {
        dialog = dialog.child(
            row()
                .w_full()
                .gap(MD)
                .center_y()
                .px(LG)
                .pb(MD)
                .child(
                    text("HTTPS needs the local certificate authority, and this Mac does not trust it yet.")
                        .font(BODY)
                        .color(k.c.text_muted)
                        .wrap()
                        .grow(1.0)
                        .min_w(0.0),
                )
                .child(
                    Button::new("Trust local CA")
                        .disabled(s.busy)
                        .on_press(|s: &mut State| s.request("POST", "/v1/ca/trust", None))
                        .secondary(k)
                        .key("trust-local-ca"),
                ),
        );
    }
    if let Some(error) = s.error.as_ref().filter(|_| form_shown) {
        dialog = dialog.child(alert(error, k).px(LG).pb(MD));
    }
    if preview {
        dialog = dialog
            .child(el().h(RULE).w_full().fill(k.c.border_base))
            .child(field("Hostname preview", &hostname(s), k).px(LG).py(MD));
    }
    dialog.child(actions).build(k, layer);
    layer.extend(selects);
}
