use super::{Error, Manifest, VerifiedArchive};
use flate2::read::GzDecoder;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use tempfile::TempDir;

#[derive(Clone, Debug)]
pub struct Bundle {
    path: PathBuf,
}

impl Bundle {
    pub fn discover(executable: &Path) -> Result<Self, Error> {
        let macos = executable.parent().ok_or(Error::OutsideBundle)?;
        let contents = macos.parent().ok_or(Error::OutsideBundle)?;
        let path = contents.parent().ok_or(Error::OutsideBundle)?;
        if macos.file_name().is_none_or(|s| s != "MacOS")
            || contents.file_name().is_none_or(|s| s != "Contents")
            || path.file_name().is_none_or(|s| s != "Reoclo.app")
            || executable.file_name().is_none_or(|s| s != "Reoclo")
        {
            return Err(Error::OutsideBundle);
        }
        let path = path.canonicalize()?;
        let parent = path.parent().ok_or(Error::OutsideBundle)?;
        if fs::metadata(parent)?.permissions().readonly() {
            return Err(Error::ReadOnly);
        }
        tempfile::Builder::new()
            .prefix(".reoclo-probe-")
            .tempdir_in(parent)
            .map_err(|_| Error::ReadOnly)?
            .close()?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn prepare(&self, archive: VerifiedArchive) -> Result<Prepared, Error> {
        let parent = self.path.parent().ok_or(Error::OutsideBundle)?;
        let stage = tempfile::Builder::new()
            .prefix(".reoclo-update-")
            .tempdir_in(parent)?;
        fs::write(stage.path().join("archive.tar.gz"), &archive.bytes)?;
        fs::write(
            stage.path().join("manifest.json"),
            serde_json::to_vec(&archive.manifest)?,
        )?;
        let mut tar = tar::Archive::new(GzDecoder::new(archive.bytes.as_slice()));
        tar.set_preserve_permissions(false);
        let mut total = 0_u64;
        for entry in tar.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.into_owned();
            if !safe_path(&path) {
                return Err(Error::Archive);
            }
            let kind = entry.header().entry_type();
            if !(kind.is_file() || kind.is_dir() || kind.is_symlink()) {
                return Err(Error::Archive);
            }
            total = total.checked_add(entry.size()).ok_or(Error::Archive)?;
            if total > 2 * 1024 * 1024 * 1024 {
                return Err(Error::Archive);
            }
            if kind.is_symlink() {
                let target = entry.link_name()?.ok_or(Error::Archive)?;
                let parent = path.parent().ok_or(Error::Archive)?;
                if !safe_link(parent, &target) {
                    return Err(Error::Archive);
                }
            }
            if !entry.unpack_in(stage.path())? {
                return Err(Error::Archive);
            }
        }
        validate_bundle(&stage.path().join("Reoclo.app"), &archive.manifest)?;
        Ok(Prepared {
            stage,
            manifest: archive.manifest,
        })
    }
}

fn safe_path(path: &Path) -> bool {
    path.starts_with("Reoclo.app") && path.components().all(|c| matches!(c, Component::Normal(_)))
}

fn safe_link(parent: &Path, target: &Path) -> bool {
    let mut depth = parent.components().count();
    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir if depth > 1 => depth -= 1,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn validate_bundle(app: &Path, manifest: &Manifest) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    let value = plist::Value::from_file(app.join("Contents/Info.plist"))?;
    let dict = value.as_dictionary().ok_or(Error::Bundle)?;
    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(plist::Value::as_string);
    let identity = dict
        .get("CFBundleIdentifier")
        .and_then(plist::Value::as_string);
    let executable = dict
        .get("CFBundleExecutable")
        .and_then(plist::Value::as_string);
    let metadata = fs::symlink_metadata(app.join("Contents/MacOS/Reoclo"))?;
    if version != Some(manifest.version.to_string().as_str())
        || identity != Some("com.reoclo.desktop")
        || executable != Some("Reoclo")
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(Error::Bundle);
    }
    Ok(())
}

pub struct Prepared {
    pub(super) stage: TempDir,
    pub manifest: Manifest,
}

impl Prepared {
    /// Called only after the old process exits. Both renames stay on the target volume.
    pub fn install(
        self,
        target: &Bundle,
        launch: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<(), Error> {
        let replacement = self.stage.path().join("Reoclo.app");
        validate_bundle(&replacement, &self.manifest)?;
        let backup = self.stage.path().join("previous.app");
        fs::rename(target.path(), &backup)?;
        if let Err(error) = fs::rename(&replacement, target.path()) {
            if let Err(rollback) = fs::rename(&backup, target.path()) {
                // Keep the only surviving old app for manual recovery.
                let recovery = self.stage.keep().join("previous.app");
                return Err(Error::Rollback(rollback, recovery));
            }
            return Err(Error::Io(error));
        }
        if launch(target.path()).is_err() {
            if let Err(rollback) = fs::rename(target.path(), &replacement)
                .and_then(|()| fs::rename(&backup, target.path()))
            {
                let recovery = self.stage.keep().join("previous.app");
                return Err(Error::Rollback(rollback, recovery));
            }
            return Err(Error::Relaunch);
        }
        self.stage.close()?;
        Ok(())
    }
}
