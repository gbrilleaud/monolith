use crate::{db::Database, inventory::scan_root, models::ScanRoot, sync::SyncEngine};
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InventoryRefreshReport {
    pub visited: usize,
    pub accepted: usize,
    pub ignored: usize,
    pub missing: usize,
    pub issues: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InventoryRefreshState {
    #[default]
    Idle,
    Scanning,
    Completed(InventoryRefreshReport),
    Error(String),
}

impl InventoryRefreshState {
    pub fn complete(&mut self, result: std::result::Result<InventoryRefreshReport, String>) {
        *self = match result {
            Ok(report) => Self::Completed(report),
            Err(error) => Self::Error(error),
        };
    }

    pub fn is_scanning(&self) -> bool {
        matches!(self, Self::Scanning)
    }
}

pub struct ClientInventory {
    database_path: PathBuf,
    roots: Vec<ScanRoot>,
    user_id: i64,
    cache_path: PathBuf,
    receiver: Option<Receiver<std::result::Result<InventoryRefreshReport, String>>>,
    state: InventoryRefreshState,
}

impl ClientInventory {
    pub fn new(
        database_path: impl AsRef<Path>,
        roots: Vec<ScanRoot>,
        user_id: i64,
        cache_path: impl AsRef<Path>,
    ) -> Self {
        Self {
            database_path: database_path.as_ref().to_path_buf(),
            roots,
            user_id,
            cache_path: cache_path.as_ref().to_path_buf(),
            receiver: None,
            state: InventoryRefreshState::Idle,
        }
    }

    pub fn state(&self) -> &InventoryRefreshState {
        &self.state
    }

    pub fn start(&mut self) -> Result<()> {
        if self.receiver.is_some() {
            anyhow::bail!("scan de bibliothèque déjà en cours");
        }
        let database_path = self.database_path.clone();
        let roots = self.roots.clone();
        let cache_path = self.cache_path.clone();
        let user_id = self.user_id;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = refresh_inventory(&database_path, &roots, user_id, &cache_path)
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = InventoryRefreshState::Scanning;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<&InventoryRefreshState> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                self.state.complete(result);
                Some(&self.state)
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.state.complete(Err("tâche de scan interrompue".into()));
                Some(&self.state)
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}

fn refresh_inventory(
    database_path: &Path,
    roots: &[ScanRoot],
    user_id: i64,
    cache_path: &Path,
) -> Result<InventoryRefreshReport> {
    let database = Database::open(database_path)?;
    let mut report = InventoryRefreshReport::default();
    for root in roots {
        let scan = scan_root(root)?;
        report.visited += scan.visited;
        report.accepted += scan.accepted;
        report.ignored += scan.ignored;
        report.issues += scan.issues.len();
        if scan.issues.is_empty() {
            report.missing += database.sync_rom_inventory(root, &scan.observations)?;
        }
    }
    SyncEngine::new(&database, user_id, cache_path).refresh_local_cache()?;
    Ok(report)
}
