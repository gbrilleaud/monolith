use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub backend_url: String,
}

impl ClientConfig {
    pub fn new(backend_url: impl AsRef<str>) -> Result<Self> {
        let backend_url = normalize_backend_url(backend_url.as_ref())?;
        Ok(Self { backend_url })
    }

    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Self::new("http://127.0.0.1:8787");
        }
        let raw =
            fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
        let config: Self = toml::from_str(&raw)
            .with_context(|| format!("configuration client invalide : {}", path.display()))?;
        Self::new(config.backend_url)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .context("le chemin de configuration client n’a pas de dossier parent")?;
        fs::create_dir_all(parent).with_context(|| format!("création de {}", parent.display()))?;
        let normalized = Self::new(&self.backend_url)?;
        let rendered = toml::to_string_pretty(&normalized).context("sérialisation client.toml")?;
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, rendered)
            .with_context(|| format!("écriture de {}", temporary.display()))?;
        fs::rename(&temporary, path)
            .with_context(|| format!("publication de {}", path.display()))?;
        Ok(())
    }
}

pub fn normalize_backend_url(value: &str) -> Result<String> {
    let normalized = value.trim().trim_end_matches('/');
    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        bail!("l’URL du backend doit commencer par http:// ou https://");
    }
    let authority = normalized
        .split_once("://")
        .map(|(_, authority)| authority)
        .unwrap_or_default();
    if authority.is_empty() || authority.contains(char::is_whitespace) {
        bail!("l’URL du backend est invalide");
    }
    Ok(normalized.into())
}
