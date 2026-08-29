use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use clap::ValueEnum;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, DecodingKey, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::backend_config::OidcConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    Local,
    Sso,
    Hybrid,
}

impl AuthMode {
    pub fn accepts(self, source: AuthSource) -> bool {
        matches!(
            (self, source),
            (Self::Local, AuthSource::Local) | (Self::Sso, AuthSource::Sso) | (Self::Hybrid, _)
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthSource {
    Local,
    Sso,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ReadOnly,
    Standard,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Standard => "standard",
            Self::Admin => "admin",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read_only" => Some(Self::ReadOnly),
            "standard" => Some(Self::Standard),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Standard | Self::Admin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthPrincipal {
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    pub source: AuthSource,
    #[serde(skip_serializing)]
    pub token: String,
    pub expires_at: Option<i64>,
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow!(error.to_string()))
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

pub fn generate_session_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn token_fingerprint(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[derive(Debug, Deserialize)]
struct OidcClaims {
    sub: String,
    exp: i64,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifiedOidcIdentity {
    pub subject: String,
    pub username: String,
    pub role: Role,
    pub expires_at: i64,
}

pub async fn verify_oidc_jwt(token: &str, config: &OidcConfig) -> Result<VerifiedOidcIdentity> {
    let header = decode_header(token).context("en-tête JWT OIDC invalide")?;
    let kid = header.kid.context("JWT OIDC sans kid")?;
    let url = config.jwks_url.as_deref().context("jwks_url absente")?;
    let jwks: JwkSet = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("JWKS invalide")?;
    let jwk = jwks.find(&kid).context("clé kid absente du JWKS")?;
    let key = DecodingKey::from_jwk(jwk)?;
    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[config.issuer.as_deref().context("issuer absent")?]);
    validation.set_audience(&[config.audience.as_deref().context("audience absente")?]);
    validation.validate_exp = true;
    let claims = decode::<OidcClaims>(token, &key, &validation)?.claims;
    let username = claims
        .preferred_username
        .unwrap_or_else(|| claims.sub.clone());
    let role = claims
        .role
        .as_deref()
        .and_then(Role::parse)
        .unwrap_or(Role::ReadOnly);
    Ok(VerifiedOidcIdentity {
        subject: claims.sub,
        username,
        role,
        expires_at: claims.exp,
    })
}
