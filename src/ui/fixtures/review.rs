use super::{State, TextEdit, TunnelState, TunnelView};

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
        s.logs.tunnel = "grafana".into();
        s.logs.query = TextEdit::new("refused");
    }
    if page == "logs" && case == "paused" {
        s.logs.paused = Some(s.model.recent_logs.iter().cloned().collect());
    }
    if matches!(case, "busy" | "quiet" | "failure") {
        crowd(s, case);
        traffic(s)?;
    }
    if matches!(
        case,
        "traffic-all" | "traffic-selected" | "traffic-terminal"
    ) {
        s.model.approvals.clear();
        traffic(s)?;
        if case == "traffic-all" {
            s.model.selected = None;
            s.table.clear_selection();
        }
        if case == "traffic-terminal" {
            s.model.selected = Some("terminal:4242:0".into());
        }
    }
    Ok(())
}

/// The panel's review states: `busy` has ten saved tunnels, a terminal session, a failure and an
/// approval; `quiet` has nothing to act on; `failure` has one failed tunnel.
fn crowd(s: &mut State, case: &str) {
    s.model.selected = None;
    s.table.clear_selection();
    if case != "busy" {
        s.model.approvals.clear();
    }
    if case == "failure"
        && let Some(postgres) = s.model.tunnels.iter_mut().find(|t| t.name == "postgres")
    {
        fail(postgres, "Port 15432 is already in use by another process.");
    }
    if case != "busy" {
        return;
    }
    let base = s.model.tunnels[0].clone();
    let tunnel = |name: &str, server: &str, address: &str| TunnelView {
        name: name.into(),
        server: server.into(),
        server_id: server.into(),
        kind: if server.is_empty() {
            "local".into()
        } else if address.starts_with("https://") {
            "https".into()
        } else {
            "tcp".into()
        },
        url: address.starts_with("https://").then(|| address.into()),
        endpoint: Some(address.into()),
        hostname: None,
        ..base.clone()
    };
    let mut billing = tunnel("billing-api", "prod-1", "127.0.0.1:19000");
    fail(
        &mut billing,
        "The upstream refused the connection on 127.0.0.1:9000.",
    );
    let mut metrics = tunnel(
        "metrics",
        "prod-1",
        "https://metrics.prod-1.reoclo.localhost:8443",
    );
    metrics.state = TunnelState::Ready;
    metrics.message = "Listening, no session open".into();
    s.model.tunnels.extend([
        tunnel("dev-gateway", "", "https://gateway.reoclo.test"),
        tunnel("dev-site", "", "https://reoclo.test"),
        tunnel("gq01-ssh", "generator-queue-01", "127.0.0.1:2201"),
        tunnel(
            "internal",
            "omo-assistant-prod-01",
            "https://internal.agents.localhost",
        ),
        tunnel("wa-feed", "omo-assistant-prod-01", "127.0.0.1:4790"),
        metrics,
        billing,
    ]);
}

fn fail(tunnel: &mut TunnelView, reason: &str) {
    tunnel.state = TunnelState::Failed;
    tunnel.enabled = true;
    tunnel.message = "The tunnel failed.".into();
    tunnel.detail = Some(reason.into());
    tunnel.action = "See the detail".into();
}

/// Fifteen minutes of one-second traffic events with periodic bursts.
fn traffic(s: &mut State) -> Result<(), serde_json::Error> {
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
    s.clock_seconds = Some(0.0);
    s.model.history.prune(now);
    Ok(())
}
