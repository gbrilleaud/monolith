use crate::{
    auth::{verify_oidc_jwt, AuthMode, AuthPrincipal, AuthSource, Role},
    backend_config::BackendConfig,
    db::Database,
    models::{CacheSnapshot, UserOverride},
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct BackendState {
    pub config: Arc<BackendConfig>,
    pub database_path: PathBuf,
}

impl BackendState {
    pub fn new(config: BackendConfig, config_directory: &std::path::Path) -> anyhow::Result<Self> {
        config.validate()?;
        let configured = PathBuf::from(&config.database_path);
        let database_path = if configured.is_absolute() {
            configured
        } else {
            config_directory.join(configured)
        };
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Database::open(&database_path)?;
        Ok(Self {
            config: Arc::new(config),
            database_path,
        })
    }

    fn database(&self) -> Result<Database, ApiError> {
        Database::open(&self.database_path).map_err(ApiError::internal)
    }
}

pub fn router(state: BackendState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/login", post(local_login))
        .route("/api/v1/auth/me", get(identity))
        .route("/api/v1/catalog", get(catalog))
        .route("/api/v1/games/:game_id/rom", get(download_game_rom))
        .route(
            "/api/v1/users/:user_id/overrides/:game_id",
            put(save_override),
        )
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub auth_mode: AuthMode,
}

async fn health(State(state): State<BackendState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        auth_mode: state.config.auth.mode,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityResponse {
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    pub source: AuthSource,
    pub expires_at: Option<i64>,
}

async fn local_login(
    State(state): State<BackendState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    if !state.config.auth.mode.accepts(AuthSource::Local) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "local_auth_disabled"));
    }
    let principal = state
        .database()?
        .authenticate_local(
            &request.username,
            &request.password,
            state.config.auth.session_ttl_seconds,
        )
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_credentials"))?;
    Ok(Json(LoginResponse {
        access_token: principal.token,
        token_type: "Bearer".into(),
        user_id: principal.user_id,
        username: principal.username,
        role: principal.role,
        expires_at: principal
            .expires_at
            .expect("une session locale doit expirer"),
    }))
}

async fn identity(
    State(state): State<BackendState>,
    headers: HeaderMap,
) -> Result<Json<IdentityResponse>, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    Ok(Json(IdentityResponse {
        user_id: principal.user_id,
        username: principal.username,
        role: principal.role,
        source: principal.source,
        expires_at: principal.expires_at,
    }))
}

async fn catalog(
    State(state): State<BackendState>,
    headers: HeaderMap,
) -> Result<Json<CacheSnapshot>, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    let snapshot = state
        .database()?
        .build_cache(principal.user_id)
        .map_err(ApiError::internal)?;
    Ok(Json(snapshot))
}

async fn download_game_rom(
    State(state): State<BackendState>,
    headers: HeaderMap,
    Path(game_id): Path<i64>,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    if !principal.role.can_write() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "rom_download_forbidden",
        ));
    }
    let location = state
        .database()?
        .available_rom_location_for_game(game_id)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "rom_unavailable"))?;
    let source = std::path::Path::new(&location.path);
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty() && !value.contains(['\r', '\n']))
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "rom_unavailable"))?;
    let bytes = std::fs::read(source)
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "rom_unavailable"))?;
    if bytes.len() as u64 != location.size_bytes {
        return Err(ApiError::new(StatusCode::CONFLICT, "rom_changed"));
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        "x-monolith-file-name",
        HeaderValue::from_str(file_name).map_err(ApiError::internal)?,
    );
    response.headers_mut().insert(
        "x-monolith-sha256",
        HeaderValue::from_str(&sha256).map_err(ApiError::internal)?,
    );
    Ok(response)
}

async fn save_override(
    State(state): State<BackendState>,
    headers: HeaderMap,
    Path((user_id, game_id)): Path<(i64, i64)>,
    Json(mut value): Json<UserOverride>,
) -> Result<StatusCode, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    if !principal.role.can_write() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "read_only"));
    }
    if principal.user_id != user_id && principal.role != Role::Admin {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "foreign_profile"));
    }
    value.user_id = user_id;
    value.game_id = game_id;
    state
        .database()?
        .save_override(&value)
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn authenticate(
    state: &BackendState,
    headers: &HeaderMap,
) -> Result<AuthPrincipal, ApiError> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "missing_bearer"))?;

    if state.config.auth.mode.accepts(AuthSource::Local) {
        if let Some(principal) = state
            .database()?
            .resolve_local_session(token)
            .map_err(ApiError::internal)?
        {
            return Ok(principal);
        }
    }
    if state.config.auth.mode.accepts(AuthSource::Sso) {
        let identity = verify_oidc_jwt(token, &state.config.auth.oidc)
            .await
            .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_sso_token"))?;
        let mut principal = state
            .database()?
            .upsert_sso_user(
                &identity.subject,
                &identity.username,
                identity.role,
                state.config.auth.oidc.auto_provision,
            )
            .map_err(ApiError::internal)?
            .ok_or_else(|| ApiError::new(StatusCode::FORBIDDEN, "sso_user_not_provisioned"))?;
        principal.token = token.into();
        principal.expires_at = Some(identity.expires_at);
        return Ok(principal);
    }
    Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_token"))
}

struct ApiError {
    status: StatusCode,
    code: String,
}

impl ApiError {
    fn new(status: StatusCode, code: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        eprintln!("backend error: {error}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.code }))).into_response()
    }
}

pub fn authentication_mode(config: &BackendConfig) -> AuthMode {
    config.auth.mode
}
