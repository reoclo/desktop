use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Tree {
    viewport: [f64; 2],
    scale: f64,
    elements: Vec<Element>,
}
#[derive(Deserialize)]
struct Element {
    #[serde(default)]
    role: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("Pass snapshots directory")?);
    let mut checked = 0;
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let tree: Tree = serde_json::from_slice(&std::fs::read(&path)?)?;
        let image = std::fs::read(path.with_extension("png"))?;
        if image.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
            return Err(format!("Invalid PNG: {}", path.display()).into());
        }
        let width = u32::from_be_bytes(image[16..20].try_into()?);
        let height = u32::from_be_bytes(image[20..24].try_into()?);
        if (f64::from(width) - tree.viewport[0] * tree.scale).abs() > 0.5
            || (f64::from(height) - tree.viewport[1] * tree.scale).abs() > 0.5
        {
            return Err(format!("Capture dimensions do not match tree: {}", path.display()).into());
        }
        if tree.elements.is_empty() {
            return Err(format!("Blank element tree: {}", path.display()).into());
        }
        for element in &tree.elements {
            let copy = element.label.as_deref().unwrap_or("");
            let text = element.text.as_deref().unwrap_or("");
            if copy.contains(['\u{2013}', '\u{2014}']) || text.contains(['\u{2013}', '\u{2014}']) {
                return Err(
                    format!("Forbidden dash in user-facing copy: {}", path.display()).into(),
                );
            }
        }
        let name = path
            .file_name()
            .ok_or("Missing filename")?
            .to_string_lossy();
        let role = if name.starts_with("add-") {
            "dialog"
        } else if name.contains("loading") {
            "status"
        } else {
            "button"
        };
        if !tree.elements.iter().any(|e| e.role == role) {
            return Err(format!("Missing {role} role: {}", path.display()).into());
        }
        checked += 1;
    }
    if checked < 84 {
        return Err(format!("Only {checked} captures; the required matrix has 84").into());
    }
    println!(
        "PASS: {checked} PNG signatures, viewport sizes, element trees, roles, and copy checked."
    );
    Ok(())
}
