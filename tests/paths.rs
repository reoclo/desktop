use reoclo_desktop::paths::socket;

#[test]
fn socket_discovery_matches_cli_legacy_xdg_and_explicit_precedence() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path();
    let suffix = "tunnels/run/tunneld.sock";
    assert_eq!(socket(root, None, None), root.join(".reoclo").join(suffix));
    std::fs::create_dir_all(root.join(".config/reoclo")).unwrap();
    std::fs::write(root.join(".config/reoclo/config.json"), "{}").unwrap();
    assert_eq!(
        socket(root, None, None),
        root.join(".config/reoclo").join(suffix)
    );
    std::fs::create_dir_all(root.join(".reoclo")).unwrap();
    std::fs::write(root.join(".reoclo/config.json"), "{}").unwrap();
    assert_eq!(socket(root, None, None), root.join(".reoclo").join(suffix));
    let xdg = root.join("xdg");
    assert_eq!(
        socket(root, None, Some(&xdg)),
        xdg.join("reoclo").join(suffix)
    );
    let explicit = root.join("explicit");
    assert_eq!(
        socket(root, Some(&explicit), Some(&xdg)),
        explicit.join(suffix)
    );
}
