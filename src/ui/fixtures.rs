use super::{Page, State, ViewState, links::PendingLink};
use crate::{cli::Cli, fake::Fixture, link, model::*, reducer::Model};
use rune::TextEdit;

impl State {
    pub fn fixture(page: &str, case: &str) -> Result<Self, serde_json::Error> {
        let f = Fixture::load()?;
        let cli = Cli::Ready {
            path: "/opt/homebrew/bin/reoclo".into(),
            version: f.status.version.clone(),
            bundled: false,
        };
        let mut s = Self {
            clock_epoch: time::OffsetDateTime::from_unix_timestamp(1794211560)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
            socket_path: "/Users/example/.config/reoclo/tunnels/run/tunneld.sock".into(),
            ca_path: Some("/Users/example/.config/reoclo/tunnels/ca/rootCA.pem".into()),
            cli: Some(cli),
            model: Model {
                status: f.status,
                tunnels: f.tunnels,
                foreground: f.foreground,
                approvals: f.approvals,
                host_entries: f.host_entries,
                selected: Some("grafana".into()),
                ..Default::default()
            },
            orgs: f.orgs,
            servers: f.servers,
            ports: f.ports,
            org: "demo".into(),
            server: "server-1".into(),
            phase: ViewState::Loaded,
            ..Default::default()
        };
        s.page = match page {
            "hosts" => Page::Hosts,
            "certificates" => Page::Certificates,
            "settings" => Page::Settings,
            _ => Page::Tunnels,
        };
        s.panel = page == "panel";
        s.updates = crate::update::Status {
            automatic: true,
            last_checked: Some("2026-11-20 04:00 UTC".into()),
            phase: match case {
                "update-available" => crate::update::Phase::Available {
                    version: "0.6.0".into(),
                },
                "update-ready" => crate::update::Phase::Ready {
                    version: "0.6.0".into(),
                    notes: "More reliable tunnel recovery. Improved keyboard navigation.".into(),
                },
                "update-error" => crate::update::Phase::Failed(
                    "The update signature is invalid. The download was discarded.".into(),
                ),
                "update-disabled" => crate::update::Phase::Disabled(
                    "Move Reoclo out of the disk image to a writable Applications folder.".into(),
                ),
                _ => crate::update::Phase::Idle,
            },
        };
        if page == "add" {
            s.modal = Some("add".into());
            s.name = TextEdit::new("api");
        }
        if page == "edit" {
            s.edit_selected();
        }
        if matches!(
            page,
            "remove" | "discard" | "start" | "setup" | "trust" | "untrust"
        ) {
            s.modal = Some(page.into());
            if page == "discard" {
                s.return_modal = Some("edit".into());
            }
            if page == "setup" {
                s.page = Page::Hosts;
            }
            if matches!(page, "trust" | "untrust") {
                s.page = Page::Certificates;
            }
        }
        s.phase = match case {
            "loading" => ViewState::Loading,
            "empty" | "terminal" => ViewState::Empty,
            "offline" => ViewState::Offline,
            "signed-out" | "device-code" => ViewState::SignedOut,
            "error" => ViewState::Error,
            _ => ViewState::Loaded,
        };
        if case == "terminal" {
            s.model.tunnels.clear();
            s.model.approvals.clear();
            s.model.selected = None;
        }
        if case == "empty" {
            s.model.foreground.clear();
            s.model.tunnels.clear();
            s.model.approvals.clear();
            s.model.selected = None;
            s.model.host_entries.clear();
            if let Some(hosts) = &mut s.model.status.hosts {
                hosts.installed = false;
                hosts.entries = 0;
            }
            if let Some(ca) = &mut s.model.status.ca {
                ca.exists = false;
                ca.trusted = false;
                ca.not_after = None;
                ca.suffixes.clear();
            }
            if s.page == Page::Settings {
                s.notice = Some("No saved preferences. Default daemon settings are shown.".into());
            }
        }
        if let Ok(state) = serde_json::from_value::<TunnelState>(serde_json::json!(case))
            && state != TunnelState::Unknown
            && case != "loaded"
            && let Some(tunnel) = s.model.tunnels.first_mut()
        {
            tunnel.state = state;
            tunnel.message = state.label().into();
            tunnel.detail = Some("The daemon reported this tunnel state.".into());
            tunnel.action = "Review the state, then retry or sign in as needed.".into();
        }
        if case == "outdated" {
            s.cli = Some(Cli::Ready {
                path: "/opt/homebrew/bin/reoclo".into(),
                version: "0.104.0".into(),
                bundled: false,
            });
            s.model.status.version = "0.103.2".into();
            if let Some(hosts) = &mut s.model.status.hosts {
                hosts.version = Some("0.103.1".into());
            }
        }
        if case == "cli-old" {
            s.cli = Some(Cli::TooOld {
                path: "/usr/local/bin/reoclo".into(),
                version: "0.98.0".into(),
            });
        }
        if case == "error" {
            s.error = Some(
                "The daemon rejected the request. Check the local daemon log and retry.".into(),
            );
        }
        if case == "events" {
            s.model.recent_logs = [
                "grafana: listener ready on port 8443",
                "pool background-worker: active",
                "grafana: upstream connection refused",
                "grafana-backup: listener stopped",
            ]
            .into_iter()
            .map(|line| crate::reducer::LogEntry {
                received: s.clock_epoch,
                line: line.into(),
            })
            .collect();
        }
        if case == "shared-failure" || case == "long-names" {
            for tunnel in &mut s.model.tunnels {
                tunnel.server = "agent-worker-prod-01".into();
            }
            let mut local = s.model.tunnels[0].clone();
            local.name = "workspace-console".into();
            local.kind = "local".into();
            local.server.clear();
            local.server_id.clear();
            local.url = Some("https://console.example.test".into());
            local.local_target = Some("127.0.0.1:43080".into());
            s.model.tunnels.push(local);
        }
        if case == "shared-failure" {
            for tunnel in &mut s.model.tunnels {
                if tunnel.name != "redis" {
                    if tunnel.kind == "tcp" {
                        tunnel.kind = "https".into();
                        tunnel.url = Some(format!("https://{}.example.test:8443", tunnel.name));
                    }
                    tunnel.state = TunnelState::Failed;
                    tunnel.message = "The tunnel failed.".into();
                    tunnel.enabled = true;
                    tunnel.detail = Some("port 8443 is already in use, so HTTPS tunnels cannot listen; free it or set https_port in tunnels.yaml and restart the daemon".into());
                    tunnel.action = "See the detail".into();
                    tunnel.traffic = Default::default();
                }
            }
        }
        if case == "device-code" {
            s.model.auth = AuthEvent {
                user_code: Some("DEMO-7294".into()),
                verification_uri: Some("https://auth.reoclo.com/device".into()),
                status: "pending".into(),
                ..Default::default()
            };
        }
        if page.starts_with("get-started") {
            s.modal = Some("get-started".into());
            match page {
                "get-started" => {
                    s.phase = ViewState::Offline;
                    s.model.status = Status::default();
                }
                "get-started-progress" => {
                    s.service_installed = true;
                    s.phase = ViewState::SignedOut;
                    s.model.status.signed_in = Some(false);
                    s.model.auth = AuthEvent {
                        user_code: Some("DEMO-7294".into()),
                        verification_uri: Some("https://auth.reoclo.com/device".into()),
                        status: "pending".into(),
                        ..Default::default()
                    };
                    if let Some(ca) = &mut s.model.status.ca {
                        ca.trusted = false;
                    }
                }
                _ => s.service_installed = true,
            }
        }
        if page == "open-link" {
            s.link = link::parse(
                "reoclo://open?v=1&org=demo&server=prod-1&port=3000&kind=https&name=grafana-web",
            )
            .ok()
            .map(PendingLink::new);
            s.modal = Some("open-link".into());
        }
        s.table.select("grafana");
        Ok(s)
    }
}
