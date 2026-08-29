use serde::{Deserialize, Serialize};

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
