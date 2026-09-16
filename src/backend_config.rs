use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, net::SocketAddr, path::Path};

use crate::{auth::AuthMode, models::ScanRoot};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BackendConfig {
    pub listen: SocketAddr,
    pub database_path: String,
    pub auth: AuthConfig,
    pub library: LibraryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct LibraryConfig {
    pub roots: Vec<ScanRoot>,
}

impl LibraryConfig {
    fn normalize_and_validate(&mut self) -> Result<()> {
        for root in &mut self.roots {
            root.extensions = root
                .extensions
                .iter()
                .map(|extension| {
                    extension
                        .trim()
                        .trim_start_matches('.')
                        .to_ascii_lowercase()
                })
                .filter(|extension| !extension.is_empty())
                .collect();
            root.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub session_ttl_seconds: i64,
    pub oidc: OidcConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct OidcConfig {
    pub issuer: Option<String>,
    pub audience: Option<String>,
    pub jwks_url: Option<String>,
    pub auto_provision: bool,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8787".parse().expect("adresse par défaut valide"),
            database_path: "data/backend.db".into(),
            auth: AuthConfig::default(),
            library: LibraryConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Local,
            session_ttl_seconds: 86_400,
            oidc: OidcConfig::default(),
        }
    }
}

impl BackendConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
        let mut config: Self = toml::from_str(&raw).context("configuration TOML invalide")?;
        config.normalize_and_validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut config = self.clone();
        config.normalize_and_validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, toml::to_string_pretty(&config)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    fn normalize_and_validate(&mut self) -> Result<()> {
        self.library.normalize_and_validate()?;
        self.validate()
    }

    pub fn validate(&self) -> Result<()> {
        self.library.clone().normalize_and_validate()?;
        if self.database_path.trim().is_empty() {
            bail!("database_path ne peut pas être vide");
        }
        if self.auth.session_ttl_seconds <= 0 {
            bail!("session_ttl_seconds doit être positif");
        }
        if matches!(self.auth.mode, AuthMode::Sso | AuthMode::Hybrid) {
            if self
                .auth
                .oidc
                .issuer
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                bail!("OIDC issuer est obligatoire en mode SSO/hybrid");
            }
            if self
                .auth
                .oidc
                .audience
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                bail!("OIDC audience est obligatoire en mode SSO/hybrid");
            }
            if self
                .auth
                .oidc
                .jwks_url
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                bail!("OIDC jwks_url est obligatoire en mode SSO/hybrid");
            }
        }
        Ok(())
    }
}
