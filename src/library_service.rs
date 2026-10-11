//! Catalogue local-first partagé par egui et le futur client FRB/Flutter.
use crate::{
    catalog_query::{
        filter_and_sort_catalog_rows, CatalogRow, CatalogSort, CatalogSortColumn, SortDirection,
    },
    cover::cover_uri,
    db::Database,
    emulator_launcher::{EmulatorLauncher, LaunchCommand},
    local_rom::{command_for_local_rom, launch_local_rom},
    models::{GameMetadata, RomAvailability, RomLocation},
};
use anyhow::{Context, Result};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone)]
pub struct LibraryGame {
    pub game_id: Option<i64>,
    pub system_id: i64,
    pub title: String,
    pub path: String,
    pub cover_uri: Option<String>,
    pub available: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct LibrarySection {
    pub system_id: i64,
    pub name: String,
    pub games: Vec<LibraryGame>,
}

pub struct LibraryService {
    db: Database,
}

impl LibraryService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn sections(&self, user_id: i64, query: &str) -> Result<Vec<LibrarySection>> {
        let systems = self.db.systems()?;
        let names: BTreeMap<_, _> = systems.into_iter().map(|s| (s.system_id, s.name)).collect();
        let games = self.db.resolved_games(user_id)?;
        let descriptions: BTreeMap<_, _> = games
            .iter()
            .map(|g| (g.game_id, g.description.clone()))
            .collect();
        let mut rows: BTreeMap<i64, Vec<CatalogRow>> = BTreeMap::new();
        for game in games {
            rows.entry(game.system_id)
                .or_default()
                .push(catalog_row_for_game(game));
        }
        for location in self.db.unlinked_rom_locations()? {
            rows.entry(location.system_id)
                .or_default()
                .push(catalog_row_for_local_rom(location));
        }
        let mut result = Vec::new();
        for (system_id, entries) in rows {
            let entries = filter_and_sort_catalog_rows(
                &entries,
                query,
                CatalogSort::new(CatalogSortColumn::Title, SortDirection::Ascending),
            );
            if entries.is_empty() {
                continue;
            }
            result.push(LibrarySection {
                system_id,
                name: names
                    .get(&system_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Système {system_id}")),
                games: entries
                    .into_iter()
                    .map(|row| {
                        let cover = row
                            .cover_art
                            .as_deref()
                            .and_then(|value| cover_uri(value).ok().flatten());
                        LibraryGame {
                            game_id: row.game_id,
                            system_id,
                            title: row.title,
                            path: row.path,
                            cover_uri: cover,
                            available: row.available,
                            description: row
                                .game_id
                                .and_then(|id| descriptions.get(&id).cloned())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name).then(a.system_id.cmp(&b.system_id)));
        Ok(result)
    }

    /// Le chemin sélectionné doit exister dans SQLite ; aucune commande libre n'est acceptée.
    pub fn command_for_rom(
        &self,
        launcher: &EmulatorLauncher,
        path: &str,
    ) -> Result<LaunchCommand> {
        let location = self
            .db
            .rom_locations()?
            .into_iter()
            .find(|row| row.path == path && row.availability == RomAvailability::Available)
            .context("ROM indisponible dans la bibliothèque locale")?;
        if location.game_id.is_none() {
            command_for_local_rom(launcher, &location)
        } else {
            launcher.command_for(location.system_id, Path::new(&location.path))
        }
    }

    pub fn launch_rom(&self, launcher: &EmulatorLauncher, path: &str) -> Result<()> {
        let location = self
            .db
            .rom_locations()?
            .into_iter()
            .find(|row| row.path == path && row.availability == RomAvailability::Available)
            .context("ROM indisponible dans la bibliothèque locale")?;
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            if location.game_id.is_none() {
                launch_local_rom(launcher, &location)?;
            } else {
                launcher.launch(location.system_id, Path::new(&location.path))?;
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            let _ = (launcher, location);
            anyhow::bail!("lancement mobile non pris en charge : adaptateur Android/iOS requis")
        }
    }
}

pub fn catalog_row_for_game(game: GameMetadata) -> CatalogRow {
    let path = game
        .launch_availability
        .preferred_path
        .clone()
        .unwrap_or_default();
    let file_name = Path::new(&path)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or(&game.title)
        .to_owned();
    let extension = Path::new(&path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    CatalogRow {
        system_id: game.system_id,
        title: game.title,
        file_name,
        path,
        extension,
        favourite: false,
        added_at: String::new(),
        size_bytes: 0,
        rating: None,
        available: game.launch_availability.available,
        cover_art: game.cover_art,
        game_id: Some(game.game_id),
    }
}

pub fn catalog_row_for_local_rom(location: RomLocation) -> CatalogRow {
    let file_name = Path::new(&location.path)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or(&location.path)
        .to_owned();
    let title = Path::new(&file_name)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or(&file_name)
        .to_owned();
    CatalogRow {
        system_id: location.system_id,
        title,
        file_name,
        path: location.path,
        extension: location.extension,
        favourite: false,
        added_at: location.last_seen_at,
        size_bytes: location.size_bytes,
        rating: None,
        available: location.availability == RomAvailability::Available,
        cover_art: None,
        game_id: location.game_id,
    }
}
