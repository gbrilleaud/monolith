use crate::{
    backend::{HealthResponse, IdentityResponse, LoginRequest, LoginResponse},
    cache::write_cache_atomic,
    models::{CacheSnapshot, UserOverride},
};
use anyhow::{Context, Result};
use std::path::Path;

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
            base_url,
            http: reqwest::Client::new(),
        })
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
        let snapshot = self.fetch_catalog(bearer_token).await?;
        let games_written = snapshot.games.len();
        write_cache_atomic(cache_path, &snapshot)?;
        Ok(games_written)
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
