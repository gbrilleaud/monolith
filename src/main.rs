use monolith::{db::Database, sync::SyncEngine, ui::MonolithApp};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let data_dir = std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    std::fs::create_dir_all(&data_dir).expect("impossible de créer le dossier de données");
    let database =
        Database::open(&data_dir.join("monolith.db")).expect("initialisation SQLite impossible");
    let cache_path = data_dir.join("monolith_cache_data.json");
    SyncEngine::new(&database, 1, &cache_path)
        .refresh_local_cache()
        .expect("publication du cache local impossible");
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Monolith",
        options,
        Box::new(move |_cc| Ok(Box::new(MonolithApp::new(database, 1, cache_path)))),
    )
}
