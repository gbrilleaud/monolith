use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RomAvailability {
    Available,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RomLocation {
    pub id: Option<i64>,
    pub game_id: Option<i64>,
    pub system_id: i64,
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_at: Option<i64>,
    pub sha256: Option<String>,
    pub availability: RomAvailability,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanRoot {
    pub system_id: i64,
    pub path: String,
    pub extensions: Vec<String>,
}

impl ScanRoot {
    pub fn validate(&self) -> Result<()> {
        if self.system_id <= 0 {
            bail!("system_id doit être positif");
        }
        if !std::path::Path::new(&self.path).is_absolute() {
            bail!("le chemin de scan doit être absolu");
        }
        if self.extensions.is_empty() {
            bail!("au moins une extension est requise");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanObservation {
    pub system_id: i64,
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanIssue {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScanReport {
    pub visited: usize,
    pub accepted: usize,
    pub ignored: usize,
    pub observations: Vec<ScanObservation>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameMetadata {
    pub game_id: i64,
    pub system_id: i64,
    pub system_name: String,
    pub title: String,
    pub description: String,
    pub cover_art: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserOverride {
    pub user_id: i64,
    pub game_id: i64,
    pub description: Option<String>,
    pub cover_art: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemSummary {
    pub system_id: i64,
    pub name: String,
    pub game_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CacheSnapshot {
    pub user_id: i64,
    pub generated_at: String,
    pub games: Vec<GameMetadata>,
}
