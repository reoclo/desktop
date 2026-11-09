use super::{State, ViewState, primitives::*};
use crate::{control::segment, theme::*};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{
    Button, ButtonVariant, Combobox, ComboboxOption, Dialog, Kit, Select, SelectAppearance,
    TextInput,
};
mod actions;
mod controls;
use actions::{hostname, save};
use controls::{choice, label};

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
    let org = choice(s, "org", "Organization", &orgs, &s.org)
        .on_choose(|s: &mut State, v, _| {
            s.org = v.into();
            s.server.clear();
            s.servers.clear();
            s.ports.clear();
            s.selected_port = None;
            s.port_picker.close();
            s.open_select = None;
            s.request("GET", &format!("/v1/orgs/{}/servers", segment(v)), None);
        })
        .build(k, &mut selects);
    let server = choice(s, "server", "Server", &servers, selected_server)
        .disabled(s.org.is_empty())
        .on_choose(|s: &mut State, v, _| {
            if let Some(server) = s.servers.iter().find(|o| o.slug == v) {
                s.server = server.id.clone();
                s.ports.clear();
                s.selected_port = None;
                s.port_picker.close();
                s.open_select = None;
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
        })
        .build(k, &mut selects);
    let port = Combobox::new("port", &s.port_picker, &ports, |s: &mut State| {
        &mut s.port_picker
    })
    .label("Port on the server")
    .current(s.selected_port.as_deref())
    .width(320.0)
    .trigger(
        Select::new::<&str>("port-value", &[])
            .label("Port on the server")
            .appearance(SelectAppearance::Field)
            .width(480.0)
            .current(
                ports
                    .iter()
                    .find(|option| Some(&option.id) == s.selected_port.as_ref())
                    .map(|option| option.label.as_str()),
            )
            .placeholder("Choose a scanned port")
            .disabled(s.server.is_empty() || ports.is_empty())
            .trigger(k),
    )
    .placeholder("Choose a scanned port")
    .search_placeholder("Search port, bind address or process")
    .disabled(s.server.is_empty() || ports.is_empty())
    .on_choose(|s: &mut State, v, _| {
        if let Some(port) = v.parse::<usize>().ok().and_then(|index| s.ports.get(index)) {
            s.target = rune::TextEdit::new(format!("127.0.0.1:{}", port.port));
            s.selected_port = Some(v.into());
            s.port_picker.close();
        }
    })
    .build(k, &mut selects);
    let kind = choice(
        s,
        "kind",
        "Kind",
        &["HTTPS".into(), "TCP".into(), "UDP".into(), "REVERSE".into()],
        &s.kind.to_ascii_uppercase(),
    )
    .on_choose(|s: &mut State, v, _| {
        s.kind = v.to_ascii_lowercase();
        s.open_select = None;
    })
    .build(k, &mut selects);
    let via = choice(
        s,
        "via",
        "Transport",
        &["reoclo".into(), "tailscale".into(), "auto".into()],
        &s.via,
    )
    .on_choose(|s: &mut State, v, _| {
        s.via = v.into();
        s.open_select = None;
    })
    .build(k, &mut selects);
    let mut fields = col()
        .w_full()
        .gap(LG)
        .padding(XL)
        .child(label(
            "Tunnel name",
            TextInput::new(&s.name)
                .width(Some(480.0))
                .label("Tunnel name")
                .placeholder("api")
                .disabled(editing)
                .on_edit(|s: &mut State| &mut s.name)
                .build(k)
                .border(RULE, k.c.border_strong)
                .key("tunnel-name"),
            k,
        ))
        .child(label("Organization", org, k))
        .child(label("Server", server, k))
        .child(label("Port on the server", port, k))
        .child(label(
            if s.kind == "reverse" {
                "Local target on this Mac"
            } else {
                "Target"
            },
            TextInput::new(&s.target)
                .width(Some(480.0))
                .label("Target")
                .placeholder("127.0.0.1:3000")
                .on_edit(|s: &mut State| &mut s.target)
                .build(k)
                .border(RULE, k.c.border_strong)
                .key("target"),
            k,
        ))
        .child(label("Kind", kind, k))
        .child(label("Transport", via, k));
    if s.kind == "https" {
        fields = fields.child(label(
            "Hostname override (optional)",
            TextInput::new(&s.hostname)
                .width(Some(480.0))
                .label("Hostname")
                .placeholder("api.prod-1.reoclo.localhost")
                .on_edit(|s: &mut State| &mut s.hostname)
                .build(k)
                .border(RULE, k.c.border_strong)
                .key("hostname"),
            k,
        ));
    } else {
        fields = fields.child(label(
            if s.kind == "reverse" {
                "Remote port on server"
            } else {
                "Local port"
            },
            TextInput::new(&s.local_port)
                .width(Some(480.0))
                .label("Port")
                .placeholder("15432")
                .on_edit(|s: &mut State| &mut s.local_port)
                .build(k)
                .border(RULE, k.c.border_strong)
                .key("local-port"),
            k,
        ));
    }
    if s.via != "reoclo" {
        fields = fields.child(text("Tailscale needs a reachable bind address. A service bound only to server loopback requires the Reoclo relay.")
            .font(BODY).color(k.c.warning.fg).wrap());
    }
    if let Some(error) = &s.error {
        fields = fields.child(
            text(error)
                .role("alert")
                .font(BODY)
                .color(k.c.danger.fg)
                .wrap(),
        );
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
        .px(XL)
        .child(
            Button::new("Cancel")
                .on_press(|s: &mut State| s.confirm_discard())
                .build(k)
                .px(0.0)
                .key("cancel-form"),
        )
        .child(
            Button::new(if editing {
                "Save changes"
            } else {
                "Save tunnel"
            })
            .variant(ButtonVariant::Contrast)
            .disabled(s.busy || !matches!(s.phase, ViewState::Loaded | ViewState::Empty))
            .on_press(|s: &mut State| save(s))
            .build(k)
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
                .h(if preview { 380.0 } else { 420.0 })
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .scroll_edge_fade(LG)
                .scroll_margin(LG)
                .key("form-scroll")
                .child(content),
        ),
    );
    if preview {
        dialog = dialog.child(field("Hostname preview", &hostname(s), k).px(XL).py(SM));
    }
    dialog.child(actions).build(k, layer);
    layer.extend(selects);
}
