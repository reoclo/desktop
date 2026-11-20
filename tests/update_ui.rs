use reoclo_desktop::{
    model::Preferences,
    ui::{State, read_preferences, view},
    update::{Phase, worker::Command},
};
use rune::{Env, Runtime};
use std::sync::mpsc;

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
}

#[test]
fn automatic_checks_default_on_for_existing_preferences_and_persist_choice() {
    let old: Preferences = serde_json::from_str(r#"{"onboarded":true}"#).unwrap();
    assert!(old.automatic_updates);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let mut state = State::fixture("settings", "loaded").unwrap();
    state.preferences_path = Some(path.clone());
    let mut rt = Runtime::new(state, view, (1100.0, 1600.0), Env::new(false, 1.0));
    rt.frame(0.0);
    click(&mut rt, "automatic-updates");
    assert!(!read_preferences(&path).automatic_updates);
}

#[test]
fn ready_update_restarts_only_after_click_in_either_surface() {
    for (page, size) in [("settings", (1100.0, 760.0)), ("panel", (340.0, 620.0))] {
        let mut state = State::fixture(page, "update-ready").unwrap();
        let (send, receive) = mpsc::channel();
        state.updater = Some(send);
        let mut rt = Runtime::new(state, view, size, Env::new(false, 1.0));
        rt.frame(0.0);
        assert!(receive.try_recv().is_err());
        click(&mut rt, "restart-update");
        assert!(matches!(receive.try_recv(), Ok(Command::Restart)));
    }
}

#[test]
fn available_download_has_no_restart_and_disabled_updates_have_no_check_action() {
    for page in ["settings", "panel"] {
        let state = State::fixture(page, "update-available").unwrap();
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
        rt.frame(0.0);
        assert!(rt.find("restart-update").is_none());
        rt.state.updates.phase = Phase::Disabled("Read-only disk image.".into());
        rt.frame(0.02);
        assert!(rt.find("check-update").is_none());
    }
}

#[test]
fn updater_starts_after_the_first_frame_and_only_once() {
    let mut state = State::default();
    let (send, receive) = mpsc::channel();
    state.updater = Some(send);
    let mut tx = rune::Tx::default();
    state.poll(2.0, &mut tx);
    assert!(receive.try_recv().is_err());
    state.poll(2.01, &mut tx);
    assert!(matches!(receive.try_recv(), Ok(Command::Start)));
    state.poll(3.0, &mut tx);
    assert!(receive.try_recv().is_err());
}
