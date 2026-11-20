use reoclo_desktop::{model::TunnelView, reducer::Model};
use time::OffsetDateTime;

fn sample(model: &mut Model, second: i64, a: u64, b: u64) {
    model
        .apply_at(
            "traffic",
            &serde_json::json!({"tunnels":[
                {"name":"db","traffic":{"bytes_in":a,"bytes_out":a * 2}},
                {"name":"web","traffic":{"bytes_in":b,"bytes_out":b * 2}}
            ]})
            .to_string(),
            OffsetDateTime::from_unix_timestamp(second).unwrap(),
        )
        .unwrap();
}

fn model() -> Model {
    Model {
        tunnels: ["db", "web"]
            .map(|name| TunnelView {
                name: name.into(),
                ..Default::default()
            })
            .into(),
        ..Default::default()
    }
}

#[test]
fn traffic_history_uses_deltas_and_sums_tunnels_at_the_same_second() {
    let mut m = model();
    sample(&mut m, 1000, 100, 200);
    sample(&mut m, 1001, 150, 210);
    let one = m.history.series(Some("db"), 1001);
    let all = m.history.series(None, 1001);
    assert_eq!((one[899].bytes_in, one[899].bytes_out), (50, 100));
    assert_eq!((all[899].bytes_in, all[899].bytes_out), (60, 120));
}

#[test]
fn reset_counters_and_missing_events_never_create_traffic_spikes() {
    let mut m = model();
    sample(&mut m, 1000, 1000, 0);
    sample(&mut m, 1001, 10, 0);
    assert_eq!(m.history.series(Some("db"), 1001)[899].bytes_in, 0);
    sample(&mut m, 1003, 30, 0);
    let series = m.history.series(Some("db"), 1003);
    assert_eq!((series[898].bytes_in, series[899].bytes_in), (0, 10));
}

#[test]
fn traffic_retains_fifteen_minutes_and_removes_deleted_tunnels() {
    let mut m = model();
    for second in 1000..2000 {
        sample(&mut m, second, u64::try_from(second).unwrap(), 0);
    }
    assert_eq!(m.history.retained("db"), 900);
    assert!(m.history.series(None, 2900).iter().all(|p| p.bytes_in == 0));
    m.apply("tunnel", r#"{"name":"db","removed":true}"#)
        .unwrap();
    assert_eq!(m.history.retained("db"), 0);
}
