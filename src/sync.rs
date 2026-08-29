use crate::cache::write_cache_atomic;
use crate::db::Database;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub games_written: usize,
    pub cache_path: PathBuf,
}

pub struct SyncEngine<'a> {
    database: &'a Database,
    user_id: i64,
    cache_path: PathBuf,
}

impl<'a> SyncEngine<'a> {
    pub fn new(database: &'a Database, user_id: i64, cache_path: impl AsRef<Path>) -> Self {
        Self {
            database,
            user_id,
            cache_path: cache_path.as_ref().to_path_buf(),
        }
    }

    pub fn refresh_local_cache(&self) -> Result<SyncReport> {
        let snapshot = self.database.build_cache(self.user_id)?;
        let games_written = snapshot.games.len();
        write_cache_atomic(&self.cache_path, &snapshot)?;
        Ok(SyncReport {
            games_written,
            cache_path: self.cache_path.clone(),
        })
    }
}
