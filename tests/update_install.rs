mod support;
use reoclo_desktop::update::{Error, install::Bundle};
use std::fs;

#[test]
fn disabled_outside_a_bundle_or_in_a_readonly_folder() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        Bundle::discover(&root.path().join("reoclo")),
        Err(Error::OutsideBundle)
    ));
    let executable = support::bundle(root.path(), "0.5.0");
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let result = Bundle::discover(&executable);
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(result, Err(Error::ReadOnly)));
}

#[test]
fn signed_archive_preserves_permissions_and_swaps_only_on_install() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let executable = support::bundle(root.path(), "0.5.0");
    let target = Bundle::discover(&executable).unwrap();
    let archive = support::release("0.6.0");
    let prepared = target.prepare(archive).unwrap();
    assert_eq!(fs::read(&executable).unwrap(), b"0.5.0");
    prepared
        .install(&target, |app| {
            assert_eq!(fs::read(app.join("Contents/MacOS/Reoclo"))?, b"0.6.0");
            Ok(())
        })
        .unwrap();
    assert_eq!(fs::read(&executable).unwrap(), b"0.6.0");
    assert_ne!(
        fs::metadata(&executable).unwrap().permissions().mode() & 0o111,
        0
    );
    assert_eq!(
        fs::read_link(root.path().join("Reoclo.app/Contents/Resources/current")).unwrap(),
        std::path::Path::new("../MacOS/Reoclo")
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn failed_relaunch_restores_the_previous_bundle() {
    let root = tempfile::tempdir().unwrap();
    let executable = support::bundle(root.path(), "0.5.0");
    let target = Bundle::discover(&executable).unwrap();
    let prepared = target.prepare(support::release("0.6.0")).unwrap();
    let result = prepared.install(&target, |_| Err(std::io::Error::other("launch failed")));
    assert!(matches!(result, Err(Error::Relaunch)));
    assert_eq!(fs::read(executable).unwrap(), b"0.5.0");
}

#[test]
fn mismatched_bundle_version_never_changes_the_installed_app() {
    let root = tempfile::tempdir().unwrap();
    let executable = support::bundle(root.path(), "0.5.0");
    let target = Bundle::discover(&executable).unwrap();
    let mut release = support::release("0.6.0");
    release.manifest.version = "0.7.0".parse().unwrap();
    assert!(matches!(target.prepare(release), Err(Error::Bundle)));
    assert_eq!(fs::read(executable).unwrap(), b"0.5.0");
}

#[test]
fn signed_archives_cannot_extract_links_outside_the_bundle() {
    let root = tempfile::tempdir().unwrap();
    let executable = support::bundle(root.path(), "0.5.0");
    let target = Bundle::discover(&executable).unwrap();
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    archive
        .append_link(&mut header, "Reoclo.app/escape", "../../outside")
        .unwrap();
    let bytes = archive.into_inner().unwrap().finish().unwrap();
    let (manifest, bytes, key) = support::signed(bytes, "0.6.0");
    let verified = manifest.verify(bytes, &key.verifying_key()).unwrap();
    assert!(matches!(target.prepare(verified), Err(Error::Archive)));
    assert_eq!(fs::read(executable).unwrap(), b"0.5.0");
}
