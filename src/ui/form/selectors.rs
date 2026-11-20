use super::*;

pub(super) fn directory(s: &State, k: &Kit, selects: &mut Vec<El>) -> (El, El, El) {
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
        selects,
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
        selects,
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
    .build(k, selects);
    (org, server, port)
}

pub(super) fn transport(s: &State, k: &Kit, selects: &mut Vec<El>) -> (El, El) {
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
        selects,
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
        selects,
    );
    (kind, via)
}
