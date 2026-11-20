#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The release manifest is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("The release version is invalid: {0}")]
    Version(#[from] semver::Error),
    #[error("The release manifest has an invalid {0}.")]
    Manifest(&'static str),
    #[error("The update download failed: {0}")]
    Http(#[from] ureq::Error),
    #[error("The update could not be written or installed: {0}")]
    Io(#[from] std::io::Error),
    #[error("The update checksum does not match. The download was discarded.")]
    Checksum,
    #[error("The update signature is invalid. The download was discarded.")]
    Signature,
    #[error("The update public key is invalid.")]
    PublicKey,
    #[error("Updates are off outside a Reoclo.app bundle.")]
    OutsideBundle,
    #[error(
        "Updates are off because the app's folder is not writable. Move Reoclo to a writable Applications folder."
    )]
    ReadOnly,
    #[error("The archive contains an unsafe or unsupported entry.")]
    Archive,
    #[error("The extracted app does not match the release version or bundle identity.")]
    Bundle,
    #[error("The app bundle metadata could not be read: {0}")]
    Plist(#[from] plist::Error),
    #[error("The updated app could not be opened. The previous app was restored.")]
    Relaunch,
    #[error("The update failed and rollback failed: {0}. The previous app is in {1}.")]
    Rollback(std::io::Error, std::path::PathBuf),
    #[error("The update worker stopped.")]
    WorkerStopped,
}
