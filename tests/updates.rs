use reoclo_desktop::update::{Error, Manifest};

fn manifest(version: &str) -> serde_json::Value {
    serde_json::json!({
        "version": version,
        "pub_date": "2026-11-20T00:00:00Z",
        "notes": "Desktop improvements.",
        "url": "https://example.com/Reoclo.app.tar.gz",
        "sha256": "00".repeat(32),
        "signature": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="
    })
}

#[test]
fn manifest_requires_a_valid_release_contract() {
    let valid = manifest("0.6.0");
    assert!(Manifest::parse(&serde_json::to_vec(&valid).unwrap()).is_ok());
    for field in ["version", "pub_date", "notes", "url", "sha256", "signature"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            Manifest::parse(&serde_json::to_vec(&missing).unwrap()).is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("version", "next"),
        ("pub_date", "yesterday"),
        ("sha256", "xyz"),
        ("url", "file:///tmp/app"),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value.into();
        assert!(
            Manifest::parse(&serde_json::to_vec(&invalid).unwrap()).is_err(),
            "{field}"
        );
    }
}

#[test]
fn release_order_uses_semver_including_prereleases() {
    for (candidate, current, newer) in [
        ("0.10.0", "0.9.9", true),
        ("0.5.0", "0.5.0", false),
        ("0.4.9", "0.5.0", false),
        ("0.6.0-rc.1", "0.6.0", false),
        ("0.6.0", "0.6.0-rc.1", true),
        ("0.5.0+new", "0.5.0+old", false),
    ] {
        let release = Manifest::parse(&serde_json::to_vec(&manifest(candidate)).unwrap()).unwrap();
        assert_eq!(
            release.is_newer_than(current).unwrap(),
            newer,
            "{candidate}"
        );
    }
    let release = Manifest::parse(&serde_json::to_vec(&manifest("0.6.0")).unwrap()).unwrap();
    assert!(matches!(
        release.is_newer_than("invalid"),
        Err(Error::Version(_))
    ));
}
