use std::path::{Path, PathBuf};

/// Match the CLI's POSIX config resolution without reading credentials.
pub fn socket(home: &Path, config_override: Option<&Path>, xdg_home: Option<&Path>) -> PathBuf {
    let config = if let Some(path) = config_override.filter(|p| !p.as_os_str().is_empty()) {
        path.to_path_buf()
    } else if let Some(path) = xdg_home.filter(|p| !p.as_os_str().is_empty()) {
        path.join("reoclo")
    } else {
        let legacy = home.join(".reoclo");
        let xdg = home.join(".config/reoclo");
        if !legacy.join("config.json").exists() && xdg.join("config.json").exists() {
            xdg
        } else {
            legacy
        }
    };
    config.join("tunnels/run/tunneld.sock")
}
