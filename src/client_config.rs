use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub backend_url: String,
    #[serde(default)]
    pub install_root: Option<String>,
}

impl ClientConfig {
    pub fn new(backend_url: impl AsRef<str>) -> Result<Self> {
        let backend_url = normalize_backend_url(backend_url.as_ref())?;
        Ok(Self {
            backend_url,
            install_root: None,
        })
    }

    pub fn with_install_root(mut self, install_root: &Path) -> Result<Self> {
        if !install_root.is_absolute() {
            bail!("le répertoire d’installation doit être absolu");
        }
        self.install_root = Some(install_root.display().to_string());
        Ok(self)
    }

    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Self::new("http://127.0.0.1:8787");
        }
        let raw =
            fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
        let raw_config: Self = toml::from_str(&raw)
            .with_context(|| format!("configuration client invalide : {}", path.display()))?;
        let mut config = Self::new(raw_config.backend_url)?;
        if let Some(install_root) = raw_config.install_root.as_deref() {
            config = config.with_install_root(Path::new(install_root))?;
        }
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .context("le chemin de configuration client n’a pas de dossier parent")?;
        fs::create_dir_all(parent).with_context(|| format!("création de {}", parent.display()))?;
        let mut normalized = Self::new(&self.backend_url)?;
        if let Some(install_root) = self.install_root.as_deref() {
            normalized = normalized.with_install_root(Path::new(install_root))?;
        }
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
