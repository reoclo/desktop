mod cli_section;
pub(super) mod updates_section;
use super::{State, primitives::*};
use crate::{model::MenuBarStyle, system::SystemAction, theme::*};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, Kit, Select, SelectAppearance, Switch, TextInput};
use serde_json::json;

const NUMBER_WIDTH: f32 = 120.0;
const MENU_BAR_WIDTH: f32 = 200.0;

fn setting(label: &str, hint: Option<&str>, control: El, k: &Kit) -> El {
    let mut copy = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(XS)
        .child(text(label).font(BODY).color(k.c.text_base));
    if let Some(hint) = hint {
        copy = copy.child(text(hint).font(HINT).color(k.c.text_muted).wrap());
    }
    row().w_full().gap(XL).center_y().child(copy).child(control)
}

fn appearance_label(value: &str) -> &'static str {
    match value {
        "light" => "Light",
        "dark" => "Dark",
        _ => "System",
    }
}

pub fn view(s: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let appearance = Select::new("appearance", &["System", "Light", "Dark"])
        .label("Appearance")
        .appearance(SelectAppearance::Field)
        .width(NUMBER_WIDTH)
        .current(Some(appearance_label(&s.appearance)))
        .open(s.open_select.as_deref() == Some("appearance"))
        .on_toggle(|s: &mut State| s.open_select = Some("appearance".into()))
        .on_close(|s: &mut State| s.open_select = None)
        .on_choose(|s: &mut State, value, _| {
            s.appearance = value.to_ascii_lowercase();
            s.open_select = None;
        })
        .build(k, layer)
        .h(FIELD_HEIGHT)
        .border(RULE, k.c.border_strong);
    let menu_bar_labels = MenuBarStyle::ALL.map(MenuBarStyle::label);
    let menu_bar = Select::new("menu-bar", &menu_bar_labels)
        .label("Menu bar")
        .appearance(SelectAppearance::Field)
        .width(MENU_BAR_WIDTH)
        .current(Some(s.menu_bar.label()))
        .open(s.open_select.as_deref() == Some("menu-bar"))
        .on_toggle(|s: &mut State| s.open_select = Some("menu-bar".into()))
        .on_close(|s: &mut State| s.open_select = None)
        .on_choose(|s: &mut State, value, _| {
            if let Some(style) = MenuBarStyle::from_label(value) {
                s.menu_bar = style;
                s.save_preferences();
                s.sync_tray();
            }
            s.open_select = None;
        })
        .build(k, layer)
        .h(FIELD_HEIGHT)
        .border(RULE, k.c.border_strong);
    let mut body = col()
        .w_full()
        .max_w(CONTENT_MAX)
        .padding(if s.compact { LG } else { XL })
        .gap(XL)
        .child(title("Settings", None, k));
    if let Some(error) = s.error.as_ref().filter(|_| s.modal.is_none()) {
        body = body.child(alert(error, k));
    }
    body = body
        .child(updates_section::view(s, k))
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(
            col()
                .w_full()
                .gap(LG)
                .child(text("Desktop app").role("heading").font(HEADING).color(k.c.text_base))
                .child(setting(
                    "Start Reoclo at login",
                    Some("Applies at your next sign-in to macOS."),
                    Switch::new(s.start_at_login)
                        .label("Start Reoclo at login")
                        .hide_label()
                        .disabled(s.busy)
                        .on_toggle(|s: &mut State| {
                            s.run_system(SystemAction::StartAtLogin(!s.start_at_login))
                        })
                        .build(k)
                        .key("start-at-login"),
                    k,
                ))
                .child(setting(
                    "Get started",
                    Some("Repeat setup for this Mac."),
                    Button::new("Run setup again")
                        .on_press(|s: &mut State| s.begin_setup())
                        .secondary(k)
                        .key("run-setup"),
                    k,
                ))
                .child(setting("Appearance", None, appearance, k))
                .child(setting(
                    "Menu bar",
                    Some("The meter shows solid active bars and short failure stubs."),
                    menu_bar,
                    k,
                )),
        )
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(
            col()
                .w_full()
                .gap(LG)
                .child(text("Tunnel daemon").role("heading").font(HEADING).color(k.c.text_base))
                .child(setting(
                    "Idle timeout (seconds)",
                    Some("Close idle sessions; keep local listeners ready."),
                    TextInput::new(&s.idle)
                        .label("Idle timeout in seconds")
                        .placeholder("600")
                        .width(Some(NUMBER_WIDTH))
                        .on_edit(|s: &mut State| &mut s.idle)
                        .build(k)
                        .border(RULE, k.c.border_strong)
                        .key("idle-timeout"),
                    k,
                ))
                .child(setting(
                    "HTTPS port",
                    Some("Default 8443. The daemon reports bind failures."),
                    TextInput::new(&s.https_port)
                        .label("HTTPS port")
                        .placeholder("8443")
                        .width(Some(NUMBER_WIDTH))
                        .on_edit(|s: &mut State| &mut s.https_port)
                        .build(k)
                        .border(RULE, k.c.border_strong)
                        .key("https-port"),
                    k,
                ))
                .child(setting(
                    "Require agent approvals",
                    Some("Review reverse tunnels, public binds, hostname changes and tailnet shares."),
                    Switch::new(s.approvals_required)
                        .label("Require agent approvals")
                        .hide_label()
                        .on_toggle(|s: &mut State| s.approvals_required = !s.approvals_required)
                        .build(k)
                        .key("require-approvals"),
                    k,
                )),
        )
        .child(
            row()
                .w_full()
                .gap(LG)
                .center_y()
                .child(field("Connected daemon", &s.model.status.version, k).grow(1.0))
                .child(
                    Button::new("Save daemon settings")
                        .disabled(s.busy)
                        .on_press(|s: &mut State| {
                            let Ok(idle) = s.idle.text.parse::<u32>() else {
                                s.error = Some("Enter the idle timeout as a positive number of seconds.".into());
                                return;
                            };
                            let Ok(port) = s.https_port.text.parse::<u16>() else {
                                s.error = Some("Enter an HTTPS port from 1 to 65535.".into());
                                return;
                            };
                            if idle == 0 || port == 0 {
                                s.error = Some("Use a positive idle timeout and an HTTPS port from 1 to 65535.".into());
                                return;
                            }
                            s.request(
                                "PATCH",
                                "/v1/settings",
                                Some(json!({"idle_timeout_seconds":idle,"https_port":port,"agent_approvals":if s.approvals_required { "required" } else { "off" }})),
                            );
                        })
                        .primary(k)
                        .key("save-settings"),
                ),
        )
        .child(el().h(RULE).w_full().fill(k.c.border_base))
        .child(cli_section::view(s, k));
    k.scroll_bar(
        col()
            .key("settings-scroll")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    )
}
