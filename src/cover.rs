use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Convertit une référence de jaquette en URI comprise par les chargeurs egui.
pub fn cover_uri(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with("file://")
    {
        return Ok(Some(value.to_owned()));
    }

    if is_windows_absolute_path(value) {
        return Ok(Some(format!("file:///{}", value.replace('\\', "/"))));
    }

    let path = Path::new(value);
    let absolute: PathBuf = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .context("répertoire courant inaccessible")?
            .join(path)
    };
    Ok(Some(format!("file://{}", absolute.display())))
}

fn is_windows_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}
