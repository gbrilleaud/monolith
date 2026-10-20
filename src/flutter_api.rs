//! Frontière FRB : pas de logique de catalogue ni d'exécution côté Dart.
use crate::{
    backend_client::BackendClient,
    cache::load_cache,
    client_config::ClientConfig,
    db::Database,
    emulator_launcher::{default_config_path, EmulatorLauncher},
    library_service::{LibrarySection, LibraryService},
    session_store::{ClientSession, SessionStore},
};
use anyhow::{bail, Context, Result};

pub fn reset_style(data_directory: String) -> Result<String> {
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::reset(&PathBuf::from(data_directory))?,
    )?)
}

pub fn style_snapshot_json(data_directory: String) -> Result<String> {
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::load(&PathBuf::from(data_directory))?,
    )?)
}

pub fn select_style(data_directory: String, id: String) -> Result<String> {
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::select(&PathBuf::from(data_directory), &id)?,
    )?)
}

pub fn set_cartridge_animation(data_directory: String, enabled: bool) -> Result<String> {
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::set_animation(&PathBuf::from(data_directory), enabled)?,
    )?)
}

pub fn save_custom_style(data_directory: String, json: String) -> Result<String> {
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::save_custom_json(&PathBuf::from(data_directory), &json)?,
    )?)
}

pub fn refresh_styles(data_directory: String) -> Result<String> {
    let url = backend_url(data_directory.clone())?;
    Ok(serde_json::to_string(
        &crate::style_config::StyleStore::refresh(&PathBuf::from(data_directory), &url)?,
    )?)
}

pub fn controller_config_text(data_directory: String) -> Result<String> {
    let root = PathBuf::from(data_directory);
    Ok(toml::to_string_pretty(
        &crate::controller_profiles::ControllerConfig::load(&root)?,
    )?)
}

pub fn save_controller_config(data_directory: String, value: String) -> Result<()> {
    if value.len() > 64 * 1024 {
        bail!("configuration des manettes trop volumineuse");
    }
    let config: crate::controller_profiles::ControllerConfig = toml::from_str(&value)?;
    config.save(&PathBuf::from(data_directory))
}

pub fn reset_controller_config(
    data_directory: String,
    system_id: i64,
    device_id: String,
) -> Result<()> {
    crate::controller_profiles::ControllerConfig::reset(
        &PathBuf::from(data_directory),
        system_id,
        &device_id,
    )
}

pub fn detected_controllers() -> Result<Vec<String>> {
    crate::controller_profiles::detected_controllers()
}

use chrono::Utc;
use std::path::{Path, PathBuf};

pub fn flutter_update_status(data_directory: String) -> Result<FlutterUpdateStatus> {
    use crate::client_update::UpdateState;
    if !cfg!(target_os = "windows") {
        return Ok(FlutterUpdateStatus {
            phase: "unsupported".into(),
            version: None,
            message: None,
        });
    }
    let root = PathBuf::from(data_directory);
    let mut guard = flutter_updater()
        .lock()
        .map_err(|_| anyhow::anyhow!("mise à jour indisponible"))?;
    if !guard.as_ref().is_some_and(|(path, _)| *path == root) {
        *guard = Some((
            root.clone(),
            crate::client_update::ClientUpdate::new_flutter(root.clone()),
        ));
    }
    let (_, updater) = guard.as_mut().context("mise à jour non initialisée")?;
    updater.poll();
    let (phase, version, message) = match updater.state() {
        UpdateState::Checking => ("checking", None, None),
        UpdateState::Unavailable => ("unavailable", None, None),
        UpdateState::Available(release) => ("available", Some(release.version.clone()), None),
        UpdateState::Downloading => ("downloading", None, None),
        UpdateState::Ready(_) => ("ready", None, None),
        UpdateState::Error(error) => ("error", None, Some(error.clone())),
    };
    let failed = root.join("updates/installation-error.txt");
    let previous_error = std::fs::read_to_string(failed).ok();
    if let Some(error) = previous_error {
        return Ok(FlutterUpdateStatus {
            phase: "error".into(),
            version: None,
            message: Some(error),
        });
    }
    Ok(FlutterUpdateStatus {
        phase: phase.into(),
        version,
        message,
    })
}

#[derive(Debug, Clone)]
pub struct FlutterUpdateStatus {
    pub phase: String,
    pub version: Option<String>,
    pub message: Option<String>,
}

fn flutter_updater(
) -> &'static std::sync::Mutex<Option<(PathBuf, crate::client_update::ClientUpdate)>> {
    static INSTANCE: std::sync::OnceLock<
        std::sync::Mutex<Option<(PathBuf, crate::client_update::ClientUpdate)>>,
    > = std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| std::sync::Mutex::new(None))
}

pub fn check_flutter_update(data_directory: String) -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("mise à jour intégrée réservée à Windows");
    }
    let _ = flutter_update_status(data_directory.clone())?;
    let _ =
        std::fs::remove_file(PathBuf::from(data_directory).join("updates/installation-error.txt"));
    let mut guard = flutter_updater()
        .lock()
        .map_err(|_| anyhow::anyhow!("mise à jour indisponible"))?;
    guard
        .as_mut()
        .context("mise à jour non initialisée")?
        .1
        .check();
    Ok(())
}

pub fn download_flutter_update(data_directory: String) -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("mise à jour intégrée réservée à Windows");
    }
    let _ = flutter_update_status(data_directory)?;
    let mut guard = flutter_updater()
        .lock()
        .map_err(|_| anyhow::anyhow!("mise à jour indisponible"))?;
    guard
        .as_mut()
        .context("mise à jour non initialisée")?
        .1
        .download()
}

pub fn install_flutter_update(data_directory: String) -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("mise à jour intégrée réservée à Windows");
    }
    let _ = flutter_update_status(data_directory.clone())?;
    let _ =
        std::fs::remove_file(PathBuf::from(data_directory).join("updates/installation-error.txt"));
    let guard = flutter_updater()
        .lock()
        .map_err(|_| anyhow::anyhow!("mise à jour indisponible"))?;
    guard
        .as_ref()
        .context("mise à jour non initialisée")?
        .1
        .install()
}

pub fn default_data_directory() -> String {
    std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from(r"C:\MONOLITH\data")
            }
            #[cfg(not(target_os = "windows"))]
            {
                PathBuf::from("data")
            }
        })
        .to_string_lossy()
        .into_owned()
}

pub fn load_library(data_directory: String, query: String) -> Result<Vec<LibrarySection>> {
    let root = PathBuf::from(data_directory);
    let user_id = active_session(&root)?.map_or(1, |session| session.user_id);
    let db = Database::open(&root.join("monolith.db"))?;
    LibraryService::new(db).sections(user_id, &query)
}

fn active_session(root: &Path) -> Result<Option<ClientSession>> {
    let store = SessionStore::new(root.join("session.json"));
    let Some(session) = store.load_valid_at(Utc::now().timestamp())? else {
        return Ok(None);
    };
    if session.backend_url != backend_url(root.to_string_lossy().into_owned())? {
        store.clear()?;
        return Ok(None);
    }
    Ok(Some(session))
}

pub fn session_identity(data_directory: String) -> Result<Option<FlutterIdentity>> {
    let root = PathBuf::from(data_directory);
    Ok(active_session(&root)?.map(|session| FlutterIdentity {
        user_id: session.user_id,
        username: session.username,
        role: session.role.as_str().to_owned(),
    }))
}

#[derive(Debug, Clone)]
pub struct FlutterIdentity {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

pub fn backend_url(data_directory: String) -> Result<String> {
    match std::env::var("MONOLITH_BACKEND_URL") {
        Ok(value) => Ok(ClientConfig::new(value)?.backend_url),
        Err(_) => Ok(ClientConfig::load_or_default(
            &PathBuf::from(data_directory).join("client.toml"),
        )?
        .backend_url),
    }
}

pub fn save_backend_url(data_directory: String, value: String) -> Result<()> {
    if std::env::var_os("MONOLITH_BACKEND_URL").is_some() {
        bail!("MONOLITH_BACKEND_URL est imposée par l'administrateur");
    }
    let root = PathBuf::from(data_directory);
    let path = root.join("client.toml");
    let mut config = ClientConfig::load_or_default(&path)?;
    let next = ClientConfig::new(value)?.backend_url;
    if config.backend_url != next {
        SessionStore::new(root.join("session.json")).clear()?;
        config.backend_url = next;
        config.save(&path)?;
    }
    Ok(())
}

pub fn login_local(
    data_directory: String,
    username: String,
    password: String,
) -> Result<FlutterIdentity> {
    let username = username.trim();
    if username.is_empty() || password.is_empty() {
        bail!("identifiant et mot de passe requis");
    }
    let root = PathBuf::from(data_directory);
    let backend_url = match std::env::var("MONOLITH_BACKEND_URL") {
        Ok(value) => ClientConfig::new(value)?.backend_url,
        Err(_) => ClientConfig::load_or_default(&root.join("client.toml"))?.backend_url,
    };
    let cache_path = root.join("monolith_cache_data.json");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("initialisation réseau")?;
    let login = runtime.block_on(async {
        let client = BackendClient::new(&backend_url)?;
        let login = client.login_local(username, &password).await?;
        client
            .refresh_offline_cache(&login.access_token, &cache_path)
            .await?;
        Ok::<_, anyhow::Error>(login)
    })?;
    let snapshot = load_cache(&cache_path)?;
    if snapshot.user_id != login.user_id {
        bail!("cache catalogue associé à un autre utilisateur");
    }
    Database::open(&root.join("monolith.db"))?.replace_catalog_snapshot(&snapshot)?;
    let identity = FlutterIdentity {
        user_id: login.user_id,
        username: login.username.clone(),
        role: login.role.as_str().to_owned(),
    };
    SessionStore::new(root.join("session.json")).save(&ClientSession {
        backend_url,
        access_token: login.access_token,
        user_id: login.user_id,
        username: login.username,
        role: login.role,
        expires_at: login.expires_at,
    })?;
    Ok(identity)
}

pub fn logout(data_directory: String) -> Result<()> {
    SessionStore::new(PathBuf::from(data_directory).join("session.json")).clear()
}

pub fn download_library_rom(data_directory: String, game_id: i64) -> Result<String> {
    let root = PathBuf::from(data_directory);
    let session =
        active_session(&root)?.context("connexion au backend requise pour télécharger")?;
    if !session.role.can_write() {
        bail!("téléchargement réservé aux comptes autorisés");
    }
    let db = Database::open(&root.join("monolith.db"))?;
    let cache_path = root.join("monolith_cache_data.json");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("initialisation réseau")?;
    let client = BackendClient::new(&session.backend_url)?;
    runtime.block_on(client.refresh_offline_cache(&session.access_token, &cache_path))?;
    let snapshot = load_cache(&cache_path)?;
    if snapshot.user_id != session.user_id {
        bail!("cache catalogue associé à un autre utilisateur");
    }
    db.replace_catalog_snapshot(&snapshot)?;
    let game = db
        .resolved_games(session.user_id)?
        .into_iter()
        .find(|game| game.game_id == game_id)
        .context("jeu absent du catalogue local")?;
    let destination = root.join("roms").join(game.system_id.to_string());
    let download =
        runtime.block_on(client.download_game_rom(&session.access_token, game_id, &destination))?;
    db.record_downloaded_rom(
        game_id,
        &download.path,
        download.size_bytes,
        &download.sha256,
    )?;
    Ok(download.path.to_string_lossy().into_owned())
}

pub fn emulator_config_text(data_directory: String) -> Result<String> {
    let root = PathBuf::from(data_directory);
    let path = std::env::var_os("MONOLITH_EMULATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&root));
    if path.exists() {
        Ok(std::fs::read_to_string(&path)?)
    } else {
        Ok(format!("# Configuration locale : {}\n# Exemple Mega Drive (system_id 11) : adaptez les chemins au cœur installé.\n# [[profile]]\n# system_id = 11\n# executable = 'C:\\MONOLITH\\tools\\retroarch\\retroarch.exe'\n# arguments = ['-L', 'C:\\MONOLITH\\tools\\retroarch\\cores\\genesis_plus_gx_libretro.dll', '{{rom}}']\n", path.display()))
    }
}

pub fn save_emulator_config(data_directory: String, value: String) -> Result<()> {
    let config: crate::emulator_launcher::EmulatorConfig =
        toml::from_str(&value).context("TOML des émulateurs invalide")?;
    EmulatorLauncher::from_config(config)?;
    let root = PathBuf::from(data_directory);
    let path = std::env::var_os("MONOLITH_EMULATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&root));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension("toml.partial");
    std::fs::write(&partial, value)?;
    std::fs::rename(&partial, &path)?;
    Ok(())
}

pub fn install_emulator(data_directory: String, name: String) -> Result<String> {
    use crate::retroarch_installer::{RetroArchInstallState, RetroArchInstaller};
    if !cfg!(target_os = "windows") {
        bail!("installation automatique disponible uniquement sous Windows");
    }
    let root = PathBuf::from(data_directory);
    let config = ClientConfig::load_or_default(&root.join("client.toml"))?;
    let install_root = std::env::var_os("MONOLITH_INSTALL_ROOT")
        .map(PathBuf::from)
        .or_else(|| config.install_root.map(PathBuf::from))
        .unwrap_or_else(|| {
            root.parent()
                .filter(|path| path.is_absolute())
                .map(PathBuf::from)
                .unwrap_or_else(|| root.clone())
        });
    let mut installer = RetroArchInstaller::new();
    match name.as_str() {
        "RetroArch" => installer.start(&install_root)?,
        "PCSX2" => installer.start_pcsx2(&install_root)?,
        "Xenia" => installer.start_xenia(&install_root)?,
        _ => bail!("émulateur non reconnu"),
    }
    loop {
        match installer.poll() {
            Some(RetroArchInstallState::InstallerStarted(directory)) => {
                return Ok(format!(
                "Installeur lancé vers {} ; terminez l’installation, puis configurez le profil.",
                directory.display()
            ))
            }
            Some(RetroArchInstallState::Error(error)) => bail!("{error}"),
            _ => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }
}

pub fn launch_library_rom(data_directory: String, path: String) -> Result<()> {
    let root = PathBuf::from(data_directory);
    let db = Database::open(&root.join("monolith.db"))?;
    let config = std::env::var_os("MONOLITH_EMULATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&root));
    let install_root =
        crate::emulator_launcher::resolve_install_root(&root, &std::env::current_exe()?)?;
    let launcher = EmulatorLauncher::load_managed(&config, &install_root, &root)?;
    LibraryService::new(db).launch_rom(&launcher, &path)
}
