use crate::{
    backend::{HealthResponse, IdentityResponse, LoginRequest, LoginResponse},
    cache::write_cache_atomic,
    models::{CacheSnapshot, UserOverride},
    rom_bundle::RomBundle,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomDownload {
    pub path: PathBuf,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RomUpload {
    pub upload_id: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Clone)]
pub struct BackendClient {
    base_url: String,
    http: reqwest::Client,
}

impl BackendClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            anyhow::bail!("URL backend invalide");
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        self.http
            .get(format!("{}/api/v1/health", self.base_url))
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()?
            .json()
            .await
            .context("politique d’authentification invalide")
    }

    pub async fn login_local(&self, username: &str, password: &str) -> Result<LoginResponse> {
        self.http
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&LoginRequest {
                username: username.into(),
                password: password.into(),
            })
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("authentification refusée")?
            .json()
            .await
            .context("réponse login invalide")
    }

    pub async fn fetch_catalog(&self, bearer_token: &str) -> Result<CacheSnapshot> {
        self.http
            .get(format!("{}/api/v1/catalog", self.base_url))
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("catalogue refusé")?
            .json()
            .await
            .context("catalogue invalide")
    }

    pub async fn identity(&self, bearer_token: &str) -> Result<IdentityResponse> {
        self.http
            .get(format!("{}/api/v1/auth/me", self.base_url))
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("session refusée")?
            .json()
            .await
            .context("identité invalide")
    }

    pub async fn refresh_offline_cache(
        &self,
        bearer_token: &str,
        cache_path: &Path,
    ) -> Result<usize> {
        let mut snapshot = self.fetch_catalog(bearer_token).await?;
        let covers = cache_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("covers");
        for game in &mut snapshot.games {
            if game
                .cover_art
                .as_deref()
                .is_some_and(|cover| cover.starts_with("http://") || cover.starts_with("https://"))
            {
                continue;
            }
            if game.cover_art.is_some() {
                game.cover_art = self
                    .download_game_cover(bearer_token, game.game_id, &covers)
                    .await
                    .ok()
                    .map(|path| path.display().to_string());
            }
        }
        let games_written = snapshot.games.len();
        write_cache_atomic(cache_path, &snapshot)?;
        Ok(games_written)
    }

    async fn download_game_cover(
        &self,
        bearer_token: &str,
        game_id: i64,
        destination_directory: &Path,
    ) -> Result<PathBuf> {
        let response = self
            .http
            .get(format!("{}/api/v1/games/{game_id}/cover", self.base_url))
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("jaquette indisponible")?;
        let file_name = response
            .headers()
            .get("x-monolith-file-name")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty() && !value.contains(['/', '\\', '\r', '\n']))
            .context("nom de jaquette invalide")?;
        let extension = Path::new(file_name)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .context("extension de jaquette absente")?;
        let bytes = response.bytes().await.context("lecture de la jaquette")?;
        if bytes.is_empty() {
            anyhow::bail!("jaquette vide");
        }
        std::fs::create_dir_all(destination_directory)?;
        let destination = destination_directory.join(format!("{game_id}.{extension}"));
        let partial = destination_directory.join(format!(".{game_id}.{extension}.partial"));
        std::fs::write(&partial, &bytes).and_then(|_| std::fs::rename(&partial, &destination))?;
        Ok(destination)
    }

    pub async fn upload_rom(
        &self,
        bearer_token: &str,
        system_id: i64,
        source: &Path,
    ) -> Result<RomUpload> {
        if system_id <= 0 {
            anyhow::bail!("system_id doit être positif");
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty() && !name.contains(['/', '\\', '\r', '\n']))
            .context("nom de ROM local invalide")?;
        let bytes = std::fs::read(source)
            .with_context(|| format!("lecture de la ROM {}", source.display()))?;
        if bytes.is_empty() {
            anyhow::bail!("ROM locale vide");
        }
        self.http
            .post(format!(
                "{}/api/v1/library/uploads?system_id={system_id}",
                self.base_url
            ))
            .bearer_auth(bearer_token)
            .header("x-monolith-file-name", file_name)
            .body(bytes)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("import ROM refusé")?
            .json()
            .await
            .context("réponse d’import ROM invalide")
    }

    pub async fn upload_rom_bundle(
        &self,
        bearer_token: &str,
        system_id: i64,
        bundle: &RomBundle,
    ) -> Result<RomUpload> {
        if system_id <= 0 || !matches!(bundle.kind, crate::rom_bundle::BundleKind::ZipArchive) {
            anyhow::bail!("bundle ROM invalide");
        }
        let bytes = std::fs::read(&bundle.path)
            .with_context(|| format!("lecture bundle ROM {}", bundle.path.display()))?;
        self.http
            .post(format!(
                "{}/api/v1/library/uploads?system_id={system_id}",
                self.base_url
            ))
            .bearer_auth(bearer_token)
            .header("x-monolith-file-name", &bundle.display_name)
            .header("x-monolith-upload-kind", "zip-bundle")
            .body(bytes)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("import bundle ROM refusé")?
            .json()
            .await
            .context("réponse d’import bundle invalide")
    }

    pub async fn download_game_rom(
        &self,
        bearer_token: &str,
        game_id: i64,
        destination_directory: &Path,
    ) -> Result<RomDownload> {
        let response = self
            .http
            .get(format!("{}/api/v1/games/{game_id}/rom", self.base_url))
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("téléchargement ROM refusé")?;
        let file_name = response
            .headers()
            .get("x-monolith-file-name")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty() && !value.contains(['/', '\\', '\r', '\n']))
            .context("nom de ROM invalide")?
            .to_owned();
        let expected_sha256 = response
            .headers()
            .get("x-monolith-sha256")
            .and_then(|value| value.to_str().ok())
            .filter(|value| {
                value.len() == 64 && value.chars().all(|value| value.is_ascii_hexdigit())
            })
            .context("empreinte ROM invalide")?
            .to_ascii_lowercase();
        let bytes = response.bytes().await.context("lecture ROM interrompue")?;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        if sha256 != expected_sha256 {
            anyhow::bail!("empreinte SHA-256 ROM invalide");
        }
        let destination = destination_directory.join(&file_name);
        std::fs::create_dir_all(destination_directory)
            .with_context(|| format!("création de {}", destination_directory.display()))?;
        let partial = destination.with_extension(format!(
            "{}.partial",
            destination
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("download")
        ));
        std::fs::write(&partial, &bytes)
            .with_context(|| format!("écriture de {}", partial.display()))?;
        std::fs::rename(&partial, &destination)
            .with_context(|| format!("publication de {}", destination.display()))?;
        Ok(RomDownload {
            path: destination.to_path_buf(),
            file_name,
            size_bytes: bytes.len() as u64,
            sha256,
        })
    }

    pub async fn save_override(&self, bearer_token: &str, value: &UserOverride) -> Result<()> {
        self.http
            .put(format!(
                "{}/api/v1/users/{}/overrides/{}",
                self.base_url, value.user_id, value.game_id
            ))
            .bearer_auth(bearer_token)
            .json(value)
            .send()
            .await
            .context("backend inaccessible")?
            .error_for_status()
            .context("surcharge refusée")?;
        Ok(())
    }
}
