use crate::auth::{
    generate_session_token, hash_password, token_fingerprint, verify_password, AuthPrincipal,
    AuthSource, Role,
};
use crate::models::{
    CacheSnapshot, GameMetadata, LaunchAvailability, RomAvailability, RomLocation, ScanObservation,
    ScanRoot, SystemSummary, UserOverride,
};
use anyhow::{anyhow, bail, Result as AnyResult};
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
             CREATE TABLE IF NOT EXISTS users (
               user_id INTEGER PRIMARY KEY AUTOINCREMENT,
               username TEXT NOT NULL UNIQUE,
               password_hash TEXT,
               sso_subject TEXT UNIQUE,
               role TEXT NOT NULL DEFAULT 'read_only',
               enabled INTEGER NOT NULL DEFAULT 1,
               created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sessions (
               token_hash TEXT PRIMARY KEY,
               user_id INTEGER NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
               expires_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
             CREATE INDEX IF NOT EXISTS idx_games_system ON games(system_id, title);
             CREATE TABLE IF NOT EXISTS rom_locations (
               id INTEGER PRIMARY KEY,
               game_id INTEGER REFERENCES games(game_id),
               system_id INTEGER NOT NULL,
               path TEXT NOT NULL UNIQUE,
               extension TEXT NOT NULL,
               size_bytes INTEGER NOT NULL,
               modified_at INTEGER,
               sha256 TEXT,
               availability TEXT NOT NULL CHECK (availability IN ('available', 'missing')),
               last_seen_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_rom_locations_system_availability
               ON rom_locations(system_id, availability);",
        )
    }

    pub fn sync_rom_inventory(
        &self,
        root: &ScanRoot,
        observations: &[ScanObservation],
    ) -> Result<usize> {
        let normalized_root = root.path.trim_end_matches('/');
        let root_prefix = format!("{normalized_root}/");
        let now = Utc::now().to_rfc3339();
        let transaction = self.conn.unchecked_transaction()?;

        transaction.execute(
            "UPDATE rom_locations
             SET availability='missing'
             WHERE system_id=?1 AND availability='available'
               AND (path=?2 OR substr(path, 1, length(?3))=?3)",
            params![root.system_id, normalized_root, root_prefix],
        )?;

        for observation in observations {
            transaction.execute(
                "INSERT INTO rom_locations(
                    game_id, system_id, path, extension, size_bytes, modified_at, sha256,
                    availability, last_seen_at
                 ) VALUES (NULL, ?1, ?2, ?3, ?4, ?5, NULL, 'available', ?6)
                 ON CONFLICT(path) DO UPDATE SET
                    system_id=excluded.system_id,
                    extension=excluded.extension,
                    size_bytes=excluded.size_bytes,
                    modified_at=excluded.modified_at,
                    availability='available',
                    last_seen_at=excluded.last_seen_at",
                params![
                    observation.system_id,
                    observation.path,
                    observation.extension,
                    observation.size_bytes,
                    observation.modified_at,
                    now,
                ],
            )?;
        }

        let missing = transaction.query_row(
            "SELECT COUNT(*) FROM rom_locations
             WHERE system_id=?1 AND availability='missing'
               AND (path=?2 OR substr(path, 1, length(?3))=?3)",
            params![root.system_id, normalized_root, root_prefix],
            |row| row.get::<_, i64>(0),
        )? as usize;
        transaction.commit()?;
        Ok(missing)
    }

    pub fn link_rom_location_to_game(&self, path: &str, game_id: i64) -> AnyResult<()> {
        let location_system_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT system_id FROM rom_locations WHERE path=?1",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        let location_system_id =
            location_system_id.ok_or_else(|| anyhow!("ROM introuvable : {path}"))?;
        let game_system_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT system_id FROM games WHERE game_id=?1",
                [game_id],
                |row| row.get(0),
            )
            .optional()?;
        let game_system_id =
            game_system_id.ok_or_else(|| anyhow!("jeu introuvable : {game_id}"))?;
        if location_system_id != game_system_id {
            bail!(
                "association refusée : ROM (système {location_system_id}) et jeu (système {game_system_id}) appartiennent à des systèmes différents"
            );
        }
        self.conn.execute(
            "UPDATE rom_locations SET game_id=?1 WHERE path=?2",
            params![game_id, path],
        )?;
        Ok(())
    }

    pub fn unlink_rom_location(&self, path: &str) -> AnyResult<()> {
        let changed = self.conn.execute(
            "UPDATE rom_locations SET game_id=NULL WHERE path=?1 AND game_id IS NOT NULL",
            [path],
        )?;
        if changed == 0 {
            bail!("association introuvable pour la ROM : {path}");
        }
        Ok(())
    }

    pub fn unlinked_rom_locations(&self) -> Result<Vec<RomLocation>> {
        let mut statement = self.conn.prepare(
            "SELECT id, game_id, system_id, path, extension, size_bytes, modified_at, sha256,
                    availability, last_seen_at
             FROM rom_locations WHERE game_id IS NULL ORDER BY system_id, path",
        )?;
        let locations = statement.query_map([], map_rom_location)?.collect();
        locations
    }

    pub fn rom_locations(&self) -> Result<Vec<RomLocation>> {
        let mut statement = self.conn.prepare(
            "SELECT id, game_id, system_id, path, extension, size_bytes, modified_at, sha256,
                    availability, last_seen_at
             FROM rom_locations ORDER BY path",
        )?;
        let locations = statement.query_map([], map_rom_location)?.collect();
        locations
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
                "SELECT g.game_id, g.system_id, g.system_name, g.title, g.description, g.cover_art, g.language,
                        EXISTS(SELECT 1 FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                        (SELECT COUNT(*) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                        (SELECT MIN(path) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available')
                 FROM games g WHERE g.game_id=?1",
                [game_id],
                map_game,
            )
            .optional()
    }

    pub fn resolved_game(&self, user_id: i64, game_id: i64) -> Result<Option<GameMetadata>> {
        self.conn.query_row(
            "SELECT g.game_id, g.system_id, g.system_name, g.title,
                    COALESCE(o.description, g.description), COALESCE(o.cover_art, g.cover_art), g.language,
                    EXISTS(SELECT 1 FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                    (SELECT COUNT(*) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                    (SELECT MIN(path) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available')
             FROM games g LEFT JOIN user_overrides o
               ON o.game_id=g.game_id AND o.user_id=?1
             WHERE g.game_id=?2", params![user_id, game_id], map_game,
        ).optional()
    }

    pub fn resolved_games(&self, user_id: i64) -> Result<Vec<GameMetadata>> {
        let mut statement = self.conn.prepare(
            "SELECT g.game_id, g.system_id, g.system_name, g.title,
                    COALESCE(o.description, g.description), COALESCE(o.cover_art, g.cover_art), g.language,
                    EXISTS(SELECT 1 FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                    (SELECT COUNT(*) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available'),
                    (SELECT MIN(path) FROM rom_locations r WHERE r.game_id=g.game_id AND r.availability='available')
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

    pub fn create_local_user(&self, username: &str, password: &str, role: Role) -> AnyResult<i64> {
        if username.trim().is_empty() {
            bail!("le nom utilisateur est vide");
        }
        if password.len() < 8 {
            bail!("le mot de passe doit contenir au moins 8 caractères");
        }
        let password_hash = hash_password(password)?;
        self.conn.execute(
            "INSERT INTO users(username, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                username.trim(),
                password_hash,
                role.as_str(),
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn authenticate_local(
        &self,
        username: &str,
        password: &str,
        ttl_seconds: i64,
    ) -> AnyResult<Option<AuthPrincipal>> {
        let record: Option<(i64, String, String, bool)> = self
            .conn
            .query_row(
                "SELECT user_id, password_hash, role, enabled FROM users WHERE username=?1",
                [username],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let Some((user_id, encoded, role, enabled)) = record else {
            return Ok(None);
        };
        if !enabled || !verify_password(password, &encoded) {
            return Ok(None);
        }
        let role = Role::parse(&role).ok_or_else(|| anyhow!("rôle invalide en base"))?;
        let token = generate_session_token();
        let expires_at = Utc::now().timestamp() + ttl_seconds;
        self.conn.execute(
            "INSERT INTO sessions(token_hash, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![token_fingerprint(&token), user_id, expires_at],
        )?;
        Ok(Some(AuthPrincipal {
            user_id,
            username: username.into(),
            role,
            source: AuthSource::Local,
            token,
            expires_at: Some(expires_at),
        }))
    }

    pub fn resolve_local_session(&self, token: &str) -> AnyResult<Option<AuthPrincipal>> {
        let record: Option<(i64, String, String, i64)> = self
            .conn
            .query_row(
                "SELECT u.user_id, u.username, u.role, s.expires_at FROM sessions s
                 JOIN users u ON u.user_id=s.user_id
                 WHERE s.token_hash=?1 AND s.expires_at>?2 AND u.enabled=1",
                params![token_fingerprint(token), Utc::now().timestamp()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        record
            .map(|(user_id, username, role, expires_at)| {
                Ok(AuthPrincipal {
                    user_id,
                    username,
                    role: Role::parse(&role).ok_or_else(|| anyhow!("rôle invalide en base"))?,
                    source: AuthSource::Local,
                    token: token.into(),
                    expires_at: Some(expires_at),
                })
            })
            .transpose()
    }

    pub fn set_user_enabled(&self, user_id: i64, enabled: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET enabled=?1 WHERE user_id=?2",
            params![enabled, user_id],
        )?;
        if !enabled {
            self.conn
                .execute("DELETE FROM sessions WHERE user_id=?1", [user_id])?;
        }
        Ok(())
    }

    pub fn upsert_sso_user(
        &self,
        subject: &str,
        username: &str,
        role: Role,
        auto_provision: bool,
    ) -> AnyResult<Option<AuthPrincipal>> {
        let existing: Option<(i64, String, String, bool)> = self
            .conn
            .query_row(
                "SELECT user_id, username, role, enabled FROM users WHERE sso_subject=?1",
                [subject],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let record = match existing {
            Some(value) => value,
            None if auto_provision => {
                let stored_username = format!("sso:{username}");
                self.conn.execute(
                    "INSERT INTO users(username, sso_subject, role, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![stored_username, subject, role.as_str(), Utc::now().to_rfc3339()],
                )?;
                (
                    self.conn.last_insert_rowid(),
                    format!("sso:{username}"),
                    role.as_str().into(),
                    true,
                )
            }
            None => return Ok(None),
        };
        if !record.3 {
            return Ok(None);
        }
        Ok(Some(AuthPrincipal {
            user_id: record.0,
            username: record.1,
            role: Role::parse(&record.2).ok_or_else(|| anyhow!("rôle invalide en base"))?,
            source: AuthSource::Sso,
            token: String::new(),
            expires_at: None,
        }))
    }

    pub fn list_users(&self) -> AnyResult<Vec<UserRecord>> {
        let mut statement = self.conn.prepare(
            "SELECT user_id, username, role, enabled, sso_subject IS NOT NULL
             FROM users ORDER BY username",
        )?;
        let records = statement
            .query_map([], |row| {
                Ok(UserRecord {
                    user_id: row.get(0)?,
                    username: row.get(1)?,
                    role: row.get(2)?,
                    enabled: row.get(3)?,
                    sso: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(records)
    }
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub sso: bool,
}

fn map_rom_location(row: &rusqlite::Row<'_>) -> Result<RomLocation> {
    let availability = match row.get::<_, String>(8)?.as_str() {
        "available" => RomAvailability::Available,
        "missing" => RomAvailability::Missing,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(RomLocation {
        id: row.get(0)?,
        game_id: row.get(1)?,
        system_id: row.get(2)?,
        path: row.get(3)?,
        extension: row.get(4)?,
        size_bytes: row.get(5)?,
        modified_at: row.get(6)?,
        sha256: row.get(7)?,
        availability,
        last_seen_at: row.get(9)?,
    })
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
        launch_availability: LaunchAvailability {
            available: row.get(7)?,
            location_count: row.get::<_, i64>(8)? as usize,
            preferred_path: row.get(9)?,
        },
    })
}
