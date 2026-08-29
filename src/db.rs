use crate::models::{CacheSnapshot, GameMetadata, SystemSummary, UserOverride};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Self {
            conn: Connection::open(path)?,
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let db = Self {
            conn: Connection::open_in_memory()?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS games (
               game_id INTEGER PRIMARY KEY,
               system_id INTEGER NOT NULL,
               system_name TEXT NOT NULL,
               title TEXT NOT NULL,
               description TEXT NOT NULL DEFAULT '',
               cover_art TEXT,
               language TEXT NOT NULL DEFAULT 'fr'
             );
             CREATE TABLE IF NOT EXISTS user_overrides (
               user_id INTEGER NOT NULL,
               game_id INTEGER NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
               description TEXT,
               cover_art TEXT,
               updated_at TEXT NOT NULL,
               PRIMARY KEY (user_id, game_id)
             );
             CREATE INDEX IF NOT EXISTS idx_games_system ON games(system_id, title);",
        )
    }

    pub fn upsert_game(&self, game: &GameMetadata) -> Result<()> {
        self.conn.execute(
            "INSERT INTO games(game_id, system_id, system_name, title, description, cover_art, language)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(game_id) DO UPDATE SET
               system_id=excluded.system_id, system_name=excluded.system_name,
               title=excluded.title, description=excluded.description,
               cover_art=excluded.cover_art, language=excluded.language",
            params![game.game_id, game.system_id, game.system_name, game.title,
                    game.description, game.cover_art, game.language],
        )?;
        Ok(())
    }

    pub fn save_override(&self, value: &UserOverride) -> Result<()> {
        self.conn.execute(
            "INSERT INTO user_overrides(user_id, game_id, description, cover_art, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(user_id, game_id) DO UPDATE SET
               description=excluded.description, cover_art=excluded.cover_art,
               updated_at=excluded.updated_at",
            params![
                value.user_id,
                value.game_id,
                value.description,
                value.cover_art,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn base_game(&self, game_id: i64) -> Result<Option<GameMetadata>> {
        self.conn
            .query_row(
                "SELECT game_id, system_id, system_name, title, description, cover_art, language
             FROM games WHERE game_id=?1",
                [game_id],
                map_game,
            )
            .optional()
    }

    pub fn resolved_game(&self, user_id: i64, game_id: i64) -> Result<Option<GameMetadata>> {
        self.conn.query_row(
            "SELECT g.game_id, g.system_id, g.system_name, g.title,
                    COALESCE(o.description, g.description), COALESCE(o.cover_art, g.cover_art), g.language
             FROM games g LEFT JOIN user_overrides o
               ON o.game_id=g.game_id AND o.user_id=?1
             WHERE g.game_id=?2", params![user_id, game_id], map_game,
        ).optional()
    }

    pub fn resolved_games(&self, user_id: i64) -> Result<Vec<GameMetadata>> {
        let mut statement = self.conn.prepare(
            "SELECT g.game_id, g.system_id, g.system_name, g.title,
                    COALESCE(o.description, g.description), COALESCE(o.cover_art, g.cover_art), g.language
             FROM games g LEFT JOIN user_overrides o
               ON o.game_id=g.game_id AND o.user_id=?1
             ORDER BY g.system_name, g.title")?;
        let games = statement.query_map([user_id], map_game)?.collect();
        games
    }

    pub fn systems(&self) -> Result<Vec<SystemSummary>> {
        let mut statement = self.conn.prepare(
            "SELECT system_id, system_name, COUNT(*) FROM games
             GROUP BY system_id, system_name ORDER BY system_name",
        )?;
        let systems = statement
            .query_map([], |row| {
                Ok(SystemSummary {
                    system_id: row.get(0)?,
                    name: row.get(1)?,
                    game_count: row.get::<_, i64>(2)? as usize,
                })
            })?
            .collect();
        systems
    }

    pub fn build_cache(&self, user_id: i64) -> Result<CacheSnapshot> {
        Ok(CacheSnapshot {
            user_id,
            generated_at: Utc::now().to_rfc3339(),
            games: self.resolved_games(user_id)?,
        })
    }
}

fn map_game(row: &rusqlite::Row<'_>) -> Result<GameMetadata> {
    Ok(GameMetadata {
        game_id: row.get(0)?,
        system_id: row.get(1)?,
        system_name: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        cover_art: row.get(5)?,
        language: row.get(6)?,
    })
}
