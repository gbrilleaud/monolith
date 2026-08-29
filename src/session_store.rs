use crate::auth::Role;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientSession {
    pub backend_url: String,
    pub access_token: String,
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self, session: &ClientSession) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("création de {}", parent.display()))?;
        }
        let temporary = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(session)?;
        write_private(&temporary, &json)?;
        fs::rename(&temporary, &self.path)
            .with_context(|| format!("publication de {}", self.path.display()))?;
        Ok(())
    }

    pub fn load_valid_at(&self, now: i64) -> Result<Option<ClientSession>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes =
            fs::read(&self.path).with_context(|| format!("lecture de {}", self.path.display()))?;
        let session: ClientSession = match serde_json::from_slice(&bytes) {
            Ok(session) => session,
            Err(_) => {
                self.clear()?;
                return Ok(None);
            }
        };
        if session.expires_at <= now {
            self.clear()?;
            return Ok(None);
        }
        Ok(Some(session))
    }

    pub fn clear(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                Err(error).with_context(|| format!("suppression de {}", self.path.display()))
            }
        }
    }
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes)?;
    Ok(())
}
