use monolith::{
    client_auth::ClientAuth, db::Database, session_store::SessionStore, sync::SyncEngine,
    ui::MonolithApp,
};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let data_dir = std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    std::fs::create_dir_all(&data_dir).expect("impossible de créer le dossier de données");
    let database =
        Database::open(&data_dir.join("monolith.db")).expect("initialisation SQLite impossible");
    let cache_path = data_dir.join("monolith_cache_data.json");
    if !cache_path.exists() {
        SyncEngine::new(&database, 1, &cache_path)
            .refresh_local_cache()
            .expect("publication du cache local impossible");
    }

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
            Ok(Box::new(MonolithApp::new(database, auth, cache_path)))
        }),
    )
}
