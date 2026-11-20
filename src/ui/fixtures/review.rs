use super::{State, TextEdit, TunnelState};

pub(super) fn apply(s: &mut State, page: &str, case: &str) -> Result<(), serde_json::Error> {
    if case == "domain-invalid" {
        s.trust_domain = TextEdit::new("example.com");
    }
    if case == "domain-valid" {
        s.trust_domain = TextEdit::new("reoclo.test");
    }
    if case == "retrying" {
        s.model.approvals.clear();
        if let Some(tunnel) = s.model.tunnels.first_mut() {
            tunnel.state = TunnelState::ServerOffline;
            tunnel.message.clear();
            tunnel.detail = Some("The server's runner disconnected.".into());
            tunnel.action.clear();
            tunnel.retry = serde_json::from_value(serde_json::json!({
                "attempt": 4, "next_at": "2026-11-09T08:06:08Z"
            }))?;
        }
    }
    if case == "events" || page == "logs" && case != "empty" {
        for line in [
            "grafana: listener ready on port 8443",
            "pool background-worker: active",
            "grafana: upstream connection refused",
            "grafana-backup: listener stopped",
        ] {
            s.model
                .append_log(crate::reducer::LogEntry::event(line.into(), s.clock_epoch));
        }
    }
    if page == "logs" && case == "long" {
        for i in 0..10_000 {
            s.model.append_log(crate::logs::LogEntry::event(
                format!("tunnel grafana: upstream request {i} completed"),
                s.clock_epoch + time::Duration::seconds(i),
            ));
        }
    }
    if page == "logs" && case == "filtered" {
        s.logs.tunnel = TextEdit::new("grafana");
        s.logs.query = TextEdit::new("refused");
    }
    if page == "logs" && case == "paused" {
        s.logs.paused = Some(s.model.recent_logs.iter().cloned().collect());
    }
    if matches!(
        case,
        "traffic-all" | "traffic-selected" | "traffic-terminal"
    ) {
        s.model.approvals.clear();
        let now = s.clock_epoch.unix_timestamp();
        let mut incoming = 0_u64;
        let mut outgoing = 0_u64;
        for i in 0..900_u32 {
            let burst = match i % 180 {
                30..=65 => 180_000,
                90..=115 => 80_000,
                _ => 4_000,
            };
            incoming += burst + u64::from(i % 17) * 1000;
            outgoing += burst / 4 + u64::from(i % 11) * 500;
            s.model.apply_at("traffic", &serde_json::json!({"tunnels":[
                {"name":"grafana","traffic":{"bytes_in":incoming,"bytes_out":outgoing}},
                {"name":"postgres","traffic":{"bytes_in":outgoing,"bytes_out":incoming / 2}},
                {"name":"terminal:4242","traffic":{"bytes_in":outgoing / 2,"bytes_out":incoming / 4}}
            ]}).to_string(), s.clock_epoch - time::Duration::seconds(899 - i64::from(i)))?;
        }
        if case == "traffic-all" {
            s.model.selected = None;
            s.table.clear_selection();
        }
        if case == "traffic-terminal" {
            s.model.selected = Some("terminal:4242:0".into());
        }
        s.clock_seconds = Some(0.0);
        s.model.history.prune(now);
    }
    Ok(())
}
