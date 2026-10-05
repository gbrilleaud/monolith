use crate::{
    auth::{verify_oidc_jwt, AuthMode, AuthPrincipal, AuthSource, Role},
    backend_config::BackendConfig,
    db::Database,
    models::{CacheSnapshot, ScanRoot, UserOverride},
};
use axum::{
    body::{to_bytes, Body},
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

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
        .route("/api/v1/games/:game_id/cover", get(download_game_cover))
        .route("/api/v1/games/:game_id/rom", get(download_game_rom))
        .route("/api/v1/library/uploads", post(upload_rom))
        .route(
            "/api/v1/library/uploads/:upload_id/publish",
            post(publish_upload),
        )
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

async fn download_game_cover(
    State(state): State<BackendState>,
    headers: HeaderMap,
    Path(game_id): Path<i64>,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    let game = state
        .database()?
        .build_cache(principal.user_id)
        .map_err(ApiError::internal)?
        .games
        .into_iter()
        .find(|game| game.game_id == game_id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cover_unavailable"))?;
    let source = game
        .cover_art
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cover_unavailable"))?;
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty() && !value.contains(['\r', '\n']))
        .map(str::to_owned)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cover_unavailable"))?;
    let bytes = std::fs::read(&source)
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "cover_unavailable"))?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        "x-monolith-file-name",
        HeaderValue::from_str(&file_name).map_err(ApiError::internal)?,
    );
    Ok(response)
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

#[derive(Debug, Deserialize)]
struct UploadQuery {
    system_id: i64,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    upload_id: String,
    file_name: String,
    size_bytes: u64,
    sha256: String,
}

const MAX_ROM_UPLOAD_BYTES: usize = 8 * 1024 * 1024 * 1024;

async fn upload_rom(
    State(state): State<BackendState>,
    headers: HeaderMap,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Result<(StatusCode, Json<UploadResponse>), ApiError> {
    let principal = authenticate(&state, &headers).await?;
    if !principal.role.can_write() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "rom_upload_forbidden"));
    }
    if query.system_id <= 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_system_id"));
    }
    let file_name = headers
        .get("x-monolith-file-name")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && !value.contains(['/', '\\', '\r', '\n'])
                && std::path::Path::new(value)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == *value)
        })
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "invalid_rom_file_name"))?
        .to_owned();
    let is_zip_bundle = headers
        .get("x-monolith-upload-kind")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "zip-bundle");
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "missing_rom_extension"))?
        .to_ascii_lowercase();
    let root = state
        .config
        .library
        .roots
        .iter()
        .find(|root| root.system_id == query.system_id)
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "upload_system_not_configured"))?;
    if !is_zip_bundle && !root.extensions.iter().any(|allowed| allowed == &extension) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "rom_extension_not_allowed",
        ));
    }
    let bytes = to_bytes(body, MAX_ROM_UPLOAD_BYTES)
        .await
        .map_err(|_| ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "rom_upload_too_large"))?;
    if bytes.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "empty_rom_upload"));
    }
    let manifest = if is_zip_bundle {
        Some(validate_zip_bundle(&bytes, &root.extensions)?)
    } else {
        None
    };
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    if state
        .database()?
        .has_available_rom_sha256(&sha256)
        .map_err(ApiError::internal)?
        || inbox_contains_sha256(&state.config.library.upload_root, &sha256)
            .map_err(ApiError::internal)?
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "rom_duplicate"));
    }

    let upload_id = uuid::Uuid::new_v4().to_string();
    let directory = PathBuf::from(&state.config.library.upload_root).join(&upload_id);
    std::fs::create_dir_all(&directory).map_err(ApiError::internal)?;
    let destination = directory.join(&file_name);
    let partial = directory.join(format!(".{file_name}.partial"));
    if let Err(error) =
        std::fs::write(&partial, &bytes).and_then(|_| std::fs::rename(&partial, &destination))
    {
        let _ = std::fs::remove_file(&partial);
        let _ = std::fs::remove_dir(&directory);
        return Err(ApiError::internal(error));
    }
    if let Some(entries) = manifest {
        let manifest = json!({
            "kind": "zip-bundle",
            "system_id": query.system_id,
            "archive_sha256": sha256,
            "entries": entries,
        });
        let partial = directory.join(".manifest.json.partial");
        let final_path = directory.join("manifest.json");
        if let Err(error) = serde_json::to_vec_pretty(&manifest)
            .map_err(std::io::Error::other)
            .and_then(|body| std::fs::write(&partial, body))
            .and_then(|_| std::fs::rename(&partial, &final_path))
        {
            let _ = std::fs::remove_dir_all(&directory);
            return Err(ApiError::internal(error));
        }
    }
    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            upload_id,
            file_name,
            size_bytes: bytes.len() as u64,
            sha256,
        }),
    ))
}

async fn publish_upload(
    State(state): State<BackendState>,
    headers: HeaderMap,
    Path(upload_id): Path<String>,
    Query(query): Query<UploadQuery>,
) -> Result<StatusCode, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    if !matches!(principal.role, crate::auth::Role::Admin) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "rom_publish_forbidden",
        ));
    }
    let upload_id = uuid::Uuid::parse_str(&upload_id)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_upload_id"))?
        .to_string();
    let root = state
        .config
        .library
        .roots
        .iter()
        .find(|root| root.system_id == query.system_id)
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "upload_system_not_configured"))?;
    let inbox = PathBuf::from(&state.config.library.upload_root).join(upload_id);
    if inbox.join("manifest.json").is_file() {
        let database = state.database()?;
        publish_zip_bundle(&inbox, root, &database)?;
        std::fs::remove_dir_all(&inbox).map_err(ApiError::internal)?;
        return Ok(StatusCode::CREATED);
    }
    let mut files = std::fs::read_dir(&inbox)
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "upload_not_found"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_none_or(|extension| extension != "partial")
        })
        .collect::<Vec<_>>();
    if files.len() != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "upload_contents_invalid",
        ));
    }
    let source = files.pop().expect("exactly one upload file");
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !root.extensions.iter().any(|allowed| allowed == &extension) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "rom_extension_not_allowed",
        ));
    }
    let file_name = source
        .file_name()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "upload_contents_invalid"))?;
    let destination_directory = PathBuf::from(&root.path);
    let destination = destination_directory.join(file_name);
    if destination.exists() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "rom_destination_exists",
        ));
    }
    let bytes = std::fs::read(&source).map_err(ApiError::internal)?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    if state
        .database()?
        .has_available_rom_sha256(&sha256)
        .map_err(ApiError::internal)?
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "rom_duplicate"));
    }
    std::fs::create_dir_all(&destination_directory).map_err(ApiError::internal)?;
    std::fs::rename(&source, &destination).map_err(ApiError::internal)?;
    if let Err(error) = state.database()?.record_published_rom(
        query.system_id,
        &destination,
        bytes.len() as u64,
        &sha256,
    ) {
        let _ = std::fs::rename(&destination, &source);
        return Err(ApiError::internal(error));
    }
    let _ = std::fs::remove_dir(&inbox);
    Ok(StatusCode::CREATED)
}

fn inbox_contains_sha256(root: &str, expected: &str) -> std::io::Result<bool> {
    let root = std::path::Path::new(root);
    if !root.exists() {
        return Ok(false);
    }
    for directory in std::fs::read_dir(root)? {
        let directory = directory?.path();
        if !directory.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if !path.is_file()
                || path
                    .extension()
                    .is_some_and(|extension| extension == "partial")
            {
                continue;
            }
            let bytes = std::fs::read(path)?;
            if format!("{:x}", Sha256::digest(&bytes)) == expected {
                return Ok(true);
            }
        }
    }
    Ok(false)
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

fn publish_zip_bundle(
    inbox: &std::path::Path,
    root: &ScanRoot,
    database: &Database,
) -> Result<(), ApiError> {
    let archive_path = std::fs::read_dir(inbox)
        .map_err(ApiError::internal)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.extension().is_some_and(|extension| extension == "zip"))
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "bundle_archive_missing"))?;
    let bytes = std::fs::read(&archive_path).map_err(ApiError::internal)?;
    let entries = validate_zip_bundle(&bytes, &root.extensions)?;
    let destination = PathBuf::from(&root.path);
    std::fs::create_dir_all(&destination).map_err(ApiError::internal)?;
    for entry in &entries {
        let name = entry["file_name"]
            .as_str()
            .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "bundle_manifest_invalid"))?;
        if destination.join(name).exists() {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "rom_destination_exists",
            ));
        }
    }
    let staging = destination.join(format!(".monolith-bundle-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging).map_err(ApiError::internal)?;
    let mut moved = Vec::with_capacity(entries.len());
    let result = (|| -> Result<(), ApiError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| ApiError::new(StatusCode::CONFLICT, "bundle_archive_invalid"))?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(ApiError::internal)?;
            let name = std::path::Path::new(entry.name())
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "bundle_entry_path_invalid"))?;
            let mut target =
                std::fs::File::create(staging.join(name)).map_err(ApiError::internal)?;
            std::io::copy(&mut entry, &mut target).map_err(ApiError::internal)?;
        }
        for entry in &entries {
            let name = entry["file_name"]
                .as_str()
                .expect("validated manifest name");
            let published = destination.join(name);
            std::fs::rename(staging.join(name), &published).map_err(ApiError::internal)?;
            moved.push((published.clone(), staging.join(name)));
            let size_bytes = entry["size_bytes"]
                .as_u64()
                .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "bundle_manifest_invalid"))?;
            let sha256 = entry["sha256"]
                .as_str()
                .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "bundle_manifest_invalid"))?;
            database
                .record_published_rom(root.system_id, &published, size_bytes, sha256)
                .map_err(ApiError::internal)?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (published, staged) in moved.into_iter().rev() {
            let _ = std::fs::rename(published, staged);
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn validate_zip_bundle(
    bytes: &[u8],
    allowed_extensions: &[String],
) -> Result<Vec<serde_json::Value>, ApiError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_rom_bundle"))?;
    if archive.is_empty() || archive.len() > 128 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "bundle_entry_count_invalid",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_rom_bundle"))?;
        let name = entry.name().to_owned();
        let basename = std::path::Path::new(&name)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| *value == name)
            .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bundle_entry_path_invalid"))?;
        if entry.is_dir() || entry.size() == 0 || !names.insert(basename.to_ascii_lowercase()) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "bundle_entry_invalid",
            ));
        }
        let extension = std::path::Path::new(basename)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !allowed_extensions
            .iter()
            .any(|allowed| allowed == &extension)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "bundle_extension_not_allowed",
            ));
        }
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(ApiError::internal)?;
        entries.push(json!({
            "file_name": basename,
            "size_bytes": contents.len(),
            "sha256": format!("{:x}", Sha256::digest(&contents)),
        }));
    }
    Ok(entries)
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
