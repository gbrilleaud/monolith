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
