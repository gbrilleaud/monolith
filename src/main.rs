use monolith::{
    backend_config::BackendConfig,
    client_auth::ClientAuth,
    db::Database,
    emulator_launcher::{default_config_path, EmulatorLauncher},
    session_store::SessionStore,
    sync::SyncEngine,
    ui::MonolithApp,
};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let data_dir = std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    std::fs::create_dir_all(&data_dir).expect("impossible de créer le dossier de données");
    let database_path = data_dir.join("monolith.db");
    let database = Database::open(&database_path).expect("initialisation SQLite impossible");
    let library_config_path = std::env::var_os("MONOLITH_LIBRARY_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/backend.toml"));
    let library_roots = BackendConfig::load(&library_config_path)
        .map(|config| config.library.roots)
        .unwrap_or_default();
    let cache_path = data_dir.join("monolith_cache_data.json");
    if !cache_path.exists() {
        SyncEngine::new(&database, 1, &cache_path)
            .refresh_local_cache()
            .expect("publication du cache local impossible");
    }

    let emulator_config_path = std::env::var_os("MONOLITH_EMULATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&data_dir));
    let emulator_launcher = EmulatorLauncher::load(&emulator_config_path)
        .unwrap_or_else(|error| panic!("configuration émulateurs impossible : {error}"));

    let backend_url =
        std::env::var("MONOLITH_BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    let session_store = SessionStore::new(data_dir.join("session.json"));
    let mut auth = ClientAuth::new(
        backend_url,
        session_store,
        cache_path.clone(),
        chrono::Utc::now().timestamp(),
    )
    .expect("chargement de la session locale impossible");
    let _ = auth.probe_policy();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Monolith",
        options,
        Box::new(move |creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            Ok(Box::new(MonolithApp::new(
                database,
                auth,
                emulator_launcher,
                database_path,
                library_roots,
                cache_path,
            )))
        }),
    )
}
