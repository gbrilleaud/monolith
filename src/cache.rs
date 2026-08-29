use crate::models::CacheSnapshot;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn write_cache_atomic(path: &Path, snapshot: &CacheSnapshot) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("création de {}", parent.display()))?;
    }
    let temporary = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(snapshot)?;
    fs::write(&temporary, json).with_context(|| format!("écriture de {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| format!("publication de {}", path.display()))?;
    Ok(())
}

pub fn load_cache(path: &Path) -> Result<CacheSnapshot> {
    let bytes = fs::read(path).with_context(|| format!("lecture de {}", path.display()))?;
    serde_json::from_slice(&bytes).context("cache JSON invalide")
}
