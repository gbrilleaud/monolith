//! Frontière FRB : pas de logique de catalogue ni d'exécution côté Dart.
use crate::{
    db::Database,
    emulator_launcher::{default_config_path, EmulatorLauncher},
    library_service::{LibrarySection, LibraryService},
};
use anyhow::Result;
use std::path::PathBuf;

pub fn default_data_directory() -> String {
    std::env::var_os("MONOLITH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"))
        .to_string_lossy()
        .into_owned()
}

pub fn load_library(data_directory: String, query: String) -> Result<Vec<LibrarySection>> {
    let db = Database::open(&PathBuf::from(data_directory).join("monolith.db"))?;
    // Profil local utilisé par le client actuel ; migration auth/session dans la tranche suivante.
    LibraryService::new(db).sections(1, &query)
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
