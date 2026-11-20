use reoclo_desktop::{
    theme,
    ui::{State, view},
};
use rune::{Env, Runtime};

#[test]
fn a_failed_card_renders_its_reason_exactly_once() {
    for case in ["failed", "shared-failure"] {
        let state = State::fixture("tunnels", case).unwrap();
        let reason = state.selected().unwrap().detail.clone().unwrap();
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let occurrences: usize = rt
            .elements()
            .iter()
            .filter_map(|e| e.text.as_deref())
            .map(|text| text.matches(&reason).count())
            .sum();
        assert_eq!(occurrences, 1, "{case}");
        assert!(rt.find("tunnel-reason").is_some());
    }
}

#[test]
fn a_tunnel_subtitle_only_adds_information_missing_from_the_reason_and_state() {
    let reason = "Port 8443 is already in use.";
    for (message, shown) in [
        ("The tunnel failed.", false),
        (reason, false),
        ("Port 8443 is already in use", false),
        ("Failed while binding the HTTPS listener", true),
    ] {
        let mut state = State::fixture("tunnels", "failed").unwrap();
        state.model.tunnels[0].detail = Some(reason.into());
        state.model.tunnels[0].message = message.into();
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let subtitle = rt
            .elements()
            .iter()
            .any(|e| e.path.contains("/tunnel-header/") && e.text.as_deref() == Some(message));
        assert_eq!(subtitle, shown, "{message}");
    }
}

#[test]
fn approval_labels_cover_the_open_action_vocabulary_and_preserve_warning_tones() {
    let k = theme::kit(false).unwrap();
    for screen in ["tunnels", "panel"] {
        for (action, label, warning) in [
            ("save_tunnel", "Save tunnel", false),
            ("public_bind", "Public bind", true),
            ("reverse_tunnel", "Reverse tunnel", true),
            ("tailnet_share", "Tailnet share", true),
            ("hostname_change", "Hostname change", false),
            ("future.action-name", "Future action name", false),
            ("PUBLIC_BIND", "Public bind", true),
            ("", "Review request", false),
        ] {
            let mut state = State::fixture(screen, "loaded").unwrap();
            state.model.approvals[0].action = action.into();
            let mut rt = Runtime::new(state, view, (1100.0, 760.0), Env::new(false, 1.0));
            rt.frame(0.0);
            let tag = rt
                .elements()
                .into_iter()
                .find(|e| e.text.as_deref() == Some(label))
                .unwrap_or_else(|| panic!("{screen}: missing label for {action:?}"));
            assert_eq!(
                tag.text_color,
                Some(if warning {
                    k.c.warning.fg
                } else {
                    k.c.text_muted
                })
            );
        }
    }
}
