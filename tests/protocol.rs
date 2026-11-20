use reoclo_desktop::model::{Status, TunnelState, TunnelView};
use reoclo_desktop::reducer::Model;

#[test]
fn phase_one_tunnel_decodes_without_later_phase_fields() {
    let tunnel: TunnelView = serde_json::from_str(
        r#"{"name":"db","org":"demo","server":"prod","server_id":"s1","kind":"tcp",
        "state":"active","enabled":true,"message":"Session open","action":"None",
        "traffic":{"bytes_in":42,"bytes_out":7,"streams":1},"future":true}"#,
    )
    .unwrap();
    assert_eq!(tunnel.state, TunnelState::Active);
    assert_eq!(tunnel.traffic.bytes_in, 42);
    assert_eq!(tunnel.url, None);
}

#[test]
fn phase_one_status_has_optional_https_and_ca() {
    let status: Status =
        serde_json::from_str(r#"{"version":"0.100.0","pid":123,"future":{}}"#).unwrap();
    assert_eq!(status.version, "0.100.0");
    assert!(status.ca.is_none());
    assert!(status.https.is_none());
}

#[test]
fn events_update_traffic_and_removal_clears_selection() {
    let mut model = Model::default();
    model.apply("tunnel", r#"{"type":"tunnel","name":"db","tunnel":{"name":"db","org":"demo","server":"prod","server_id":"s1","kind":"tcp","state":"ready"}}"#).unwrap();
    model.selected = Some("db".into());
    model
        .apply(
            "traffic",
            r#"{"tunnels":[{"name":"db","traffic":{"bytes_in":64,"bytes_out":12,"streams":2}}]}"#,
        )
        .unwrap();
    assert_eq!(model.tunnels[0].traffic.bytes_in, 64);
    model
        .apply("tunnel", r#"{"type":"tunnel","name":"db","removed":true}"#)
        .unwrap();
    assert!(model.tunnels.is_empty());
    assert_eq!(model.selected, None);
}

#[test]
fn future_events_and_states_do_not_break_the_stream() {
    let state: TunnelState = serde_json::from_str("\"future_state\"").unwrap();
    assert_eq!(state, TunnelState::Unknown);
    let mut model = Model::default();
    model.apply("future_event", r#"{"anything":true}"#).unwrap();
    assert!(model.tunnels.is_empty());
}

#[test]
fn sorting_orders_tunnels_by_organization_then_name() {
    let mut model = Model::default();
    for (org, name) in [("zeta", "db"), ("alpha", "cache"), ("alpha", "api")] {
        model.tunnels.push(TunnelView {
            org: org.into(),
            server: "prod".into(),
            name: name.into(),
            ..Default::default()
        });
    }
    model.sort();
    assert_eq!(
        model
            .tunnels
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        vec!["api", "cache", "db"]
    );
}

#[test]
fn reconnect_backoff_is_bounded_without_wall_clock_sleeps() {
    use reoclo_desktop::connection::backoff;
    assert_eq!(
        [0, 1, 2, 5, 30].map(|n| backoff(n).as_secs()),
        [1, 2, 4, 32, 32]
    );
}

#[test]
fn real_phase_three_hosts_shape_decodes_numeric_entries_and_relay_object() {
    let status: Status = serde_json::from_str(r#"{"version":"0.102.0","signed_in":false,"hosts":{"installed":true,"entries":2,"suffixes":["reoclo.localhost"],"relay":{"enabled":true,"listening":false,"port":443,"error":"Port is busy"}}}"#).unwrap();
    assert_eq!(status.signed_in, Some(false));
    let hosts = status.hosts.unwrap();
    assert_eq!(hosts.entries, 2);
    assert!(hosts.relay.enabled);
    assert!(!hosts.relay.listening);
}
