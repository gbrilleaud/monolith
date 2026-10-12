//! Frontière FRB : pas de logique de catalogue ni d'exécution côté Dart.
use crate::{
    db::Database,
    emulator_launcher::{default_config_path, EmulatorLauncher},
    library_service::{LibrarySection, LibraryService},
    session_store::SessionStore,
};
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;

pub fn default_data_directory() -> String {
    std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"))
        .to_string_lossy()
        .into_owned()
}

pub fn load_library(data_directory: String, query: String) -> Result<Vec<LibrarySection>> {
    let root = PathBuf::from(data_directory);
    let user_id = SessionStore::new(root.join("session.json"))
        .load_valid_at(Utc::now().timestamp())?
        .map_or(1, |session| session.user_id);
    let db = Database::open(&root.join("monolith.db"))?;
    LibraryService::new(db).sections(user_id, &query)
}

pub fn launch_library_rom(data_directory: String, path: String) -> Result<()> {
    let root = PathBuf::from(data_directory);
    let db = Database::open(&root.join("monolith.db"))?;
    let config = std::env::var_os("MONOLITH_EMULATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&root));
    let launcher = EmulatorLauncher::load(&config)?;
    LibraryService::new(db).launch_rom(&launcher, &path)
}
