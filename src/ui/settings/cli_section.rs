use super::*;
use crate::cli::{self, Cli, MIN_VERSION};

pub(super) fn view(s: &State, k: &Kit) -> El {
    let (path, version) = match &s.cli {
        Some(Cli::Ready {
            path,
            version,
            bundled,
        }) => (
            if *bundled {
                format!("{} (bundled with the app)", path.display())
            } else {
                path.display().to_string()
            },
            version.clone(),
        ),
        Some(Cli::TooOld { path, version }) => (path.display().to_string(), version.clone()),
        Some(Cli::Missing) => ("Not found".into(), "None".into()),
        None => ("Looking for the CLI".into(), "Unknown".into()),
    };
    let helper = match s.model.status.hosts.as_ref() {
        Some(hosts) if hosts.installed => hosts
            .version
            .clone()
            .unwrap_or_else(|| "Older than 0.103.1".into()),
        _ => "Not installed".into(),
    };
    let mut section = col()
        .w_full()
        .gap(LG)
        .child(text("Command-line tool").role("heading").font(HEADING).color(k.c.text_base))
        .child(
            text("Reoclo runs the reoclo CLI installed on this Mac, so upgrading the CLI also updates the daemon. The copy bundled with the app is used only when none is installed.")
                .font(HINT)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(field("Reoclo CLI", &path, k).key("cli-path"))
        .child(
            row()
                .w_full()
                .gap(XL)
                .child(field("CLI version", &version, k).grow(1.0))
                .child(field("Hosts helper", &helper, k).grow(1.0)),
        );
    if let Some(Cli::TooOld { path, .. }) = &s.cli {
        let message = format!(
            "This CLI is older than the app needs ({MIN_VERSION} or later). Run {} to update it.",
            cli::upgrade_command(path)
        );
        section = section.child(alert(&message, k));
    }
    if s.daemon_behind() {
        let hint = format!(
            "The daemon runs {}. Restart it to use the CLI's version. Tunnels reconnect on their own.",
            s.model.status.version
        );
        section = section.child(setting(
            "Restart the daemon",
            Some(&hint),
            Button::new("Restart daemon")
                .disabled(s.busy)
                .on_press(|s: &mut State| s.run_system(SystemAction::RestartDaemon))
                .secondary(k)
                .key("restart-daemon"),
            k,
        ));
    }
    if let Some(helper) = s.helper_behind() {
        let hint = format!(
            "The hosts helper is {helper}. Updating reinstalls it from the CLI and asks for an administrator password."
        );
        section = section.child(setting(
            "Update the hosts helper",
            Some(&hint),
            Button::new("Update helper")
                .disabled(s.busy)
                .on_press(|s: &mut State| {
                    let setup = s.helper_setup(s.relay_enabled());
                    s.run_system(SystemAction::SetupHosts(setup));
                })
                .secondary(k)
                .key("update-helper"),
            k,
        ));
    }
    section
}
