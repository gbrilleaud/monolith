use anyhow::{bail, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

const REPO: &str = "gbrilleaud/monolith";
const ASSET: &str = "monolith-windows-x86_64.zip";
const CHECKSUM: &str = "monolith-windows-x86_64.zip.sha256";
const MAX_ARCHIVE_BYTES: usize = 150 * 1024 * 1024;
const MAX_EXE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    archive_url: String,
    checksum_url: String,
}

fn version(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        bail!("version non prise en charge");
    }
    Ok([parts[0].parse()?, parts[1].parse()?, parts[2].parse()?])
}

pub fn parse_release(value: &Value, current: &str) -> Result<Option<AvailableUpdate>> {
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        return Ok(None);
    }
    let Some(tag) = value["tag_name"].as_str() else {
        return Ok(None);
    };
    let Some(release_version) = tag.strip_prefix('v') else {
        return Ok(None);
    };
    let Ok(latest) = version(release_version) else {
        return Ok(None);
    };
    if latest <= version(current)? {
        return Ok(None);
    }
    let assets = value["assets"]
        .as_array()
        .context("liste des artefacts absente")?;
    let asset_url = |name: &str| -> Option<String> {
        let url =
            assets.iter().find(|asset| asset["name"] == name)?["browser_download_url"].as_str()?;
        let expected = format!("https://github.com/{REPO}/releases/download/{tag}/{name}");
        (url == expected).then(|| url.to_owned())
    };
    Ok(match (asset_url(ASSET), asset_url(CHECKSUM)) {
        (Some(archive_url), Some(checksum_url)) => Some(AvailableUpdate {
            version: release_version.to_owned(),
            archive_url,
            checksum_url,
        }),
        _ => None,
    })
}

pub fn unpack_verified_client(archive: &[u8], checksum: &str, destination: &Path) -> Result<()> {
    if archive.len() > MAX_ARCHIVE_BYTES {
        bail!("archive trop volumineuse");
    }
    let expected_line = checksum.trim_end_matches(['\r', '\n']);
    let (hash, name) = expected_line
        .split_once("  ")
        .context("checksum invalide")?;
    if name != ASSET || hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("checksum invalide");
    }
    if format!("{:x}", Sha256::digest(archive)) != hash.to_ascii_lowercase() {
        bail!("SHA-256 de l’archive incorrect");
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).context("archive illisible")?;
    let file = zip
        .by_name("monolith-windows-x86_64/monolith.exe")
        .context("binaire Monolith absent")?;
    if file.size() == 0 || file.size() > MAX_EXE_BYTES {
        bail!("taille du binaire invalide");
    }
    let partial = destination.with_extension("exe.partial");
    let result = (|| -> Result<()> {
        let mut output = std::fs::File::create(&partial)?;
        let count = std::io::copy(&mut file.take(MAX_EXE_BYTES + 1), &mut output)?;
        if count == 0 || count > MAX_EXE_BYTES {
            bail!("binaire décompressé trop volumineux");
        }
        output.flush()?;
        output.sync_all()?;
        std::fs::rename(&partial, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(partial);
    }
    result
}

async fn download_limited(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > max as u64)
    {
        bail!("téléchargement trop volumineux");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > max.saturating_sub(bytes.len()) {
            bail!("téléchargement trop volumineux");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn latest(client: &reqwest::Client) -> Result<Option<AvailableUpdate>> {
    let response = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let value: Value = response.error_for_status()?.json().await?;
    parse_release(&value, env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone)]
pub enum UpdateState {
    Checking,
    Unavailable,
    Available(AvailableUpdate),
    Downloading,
    Ready(PathBuf),
    Error(String),
}

pub struct ClientUpdate {
    state: UpdateState,
    receiver: Option<Receiver<Result<UpdateState, String>>>,
    data_dir: PathBuf,
}

impl ClientUpdate {
    pub fn new(data_dir: PathBuf) -> Self {
        let update = Self {
            state: UpdateState::Unavailable,
            receiver: None,
            data_dir,
        };
        #[cfg(target_os = "windows")]
        {
            let mut update = update;
            update.check();
            update
        }
        #[cfg(not(target_os = "windows"))]
        update
    }
    pub fn state(&self) -> &UpdateState {
        &self.state
    }
    pub fn busy(&self) -> bool {
        matches!(self.state, UpdateState::Checking | UpdateState::Downloading)
    }
    pub fn check(&mut self) {
        if self.busy() {
            return;
        }
        self.state = UpdateState::Checking;
        self.spawn(move || {
            let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let client = http_client().map_err(|e| e.to_string())?;
                latest(&client)
                    .await
                    .map(|value| value.map_or(UpdateState::Unavailable, UpdateState::Available))
                    .map_err(|e| e.to_string())
            })
        });
    }
    pub fn download(&mut self) -> Result<()> {
        if self.busy() {
            bail!("une opération est déjà en cours");
        }
        let UpdateState::Available(release) = self.state.clone() else {
            bail!("aucune mise à jour disponible")
        };
        let data_dir = self.data_dir.clone();
        self.state = UpdateState::Downloading;
        self.spawn(move || {
            let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let client = http_client().map_err(|e| e.to_string())?;
                let checksum = download_limited(&client, &release.checksum_url, 1024)
                    .await
                    .map_err(|e| e.to_string())?;
                let archive = download_limited(&client, &release.archive_url, MAX_ARCHIVE_BYTES)
                    .await
                    .map_err(|e| e.to_string())?;
                let dir = data_dir.join("updates");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(format!("monolith-{}.exe", release.version));
                unpack_verified_client(
                    &archive,
                    std::str::from_utf8(&checksum).map_err(|e| e.to_string())?,
                    &path,
                )
                .map_err(|e| e.to_string())?;
                Ok(UpdateState::Ready(path))
            })
        });
        Ok(())
    }
    fn spawn(&mut self, task: impl FnOnce() -> Result<UpdateState, String> + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(task());
        });
        self.receiver = Some(rx);
    }
    pub fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(state)) => {
                self.state = state;
                self.receiver = None;
            }
            Ok(Err(error)) => {
                self.state = UpdateState::Error(error);
                self.receiver = None;
            }
            Err(TryRecvError::Disconnected) => {
                self.state = UpdateState::Error("worker interrompu".into());
                self.receiver = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
    pub fn install(&self) -> Result<()> {
        let UpdateState::Ready(staged) = &self.state else {
            bail!("mise à jour non téléchargée")
        };
        install_and_restart(staged, &self.data_dir)
    }
}

fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("Monolith-Client-Updater")
        .timeout(std::time::Duration::from_secs(300))
        .build()?)
}

#[cfg(target_os = "windows")]
fn install_and_restart(staged: &Path, data_dir: &Path) -> Result<()> {
    use std::process::Command;
    let target = std::env::current_exe()?;
    let helper = data_dir.join("updates").join("monolith-updater.exe");
    std::fs::copy(staged, &helper).context("copie du programme d’installation")?;
    let sha = format!("{:x}", Sha256::digest(std::fs::read(staged)?));
    Command::new(&helper)
        .arg("--apply-update")
        .arg(staged)
        .arg(&target)
        .arg(sha)
        .arg(std::env::current_dir()?)
        .spawn()
        .context("démarrage de l’installation")?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_and_restart(_staged: &Path, _data_dir: &Path) -> Result<()> {
    bail!("mise à jour intégrée disponible uniquement sur Windows")
}

/// Exécuté par une copie temporaire du nouveau binaire, après la fermeture du client original.
#[cfg(target_os = "windows")]
pub fn apply_update(
    staged: &Path,
    target: &Path,
    expected_sha: &str,
    working_dir: &Path,
) -> Result<()> {
    use std::{process::Command, time::Duration};
    if expected_sha.len() != 64
        || !expected_sha.bytes().all(|b| b.is_ascii_hexdigit())
        || format!("{:x}", Sha256::digest(std::fs::read(staged)?))
            != expected_sha.to_ascii_lowercase()
    {
        bail!("binaire téléchargé altéré");
    }
    let backup = target.with_extension("exe.monolith-backup");
    if backup.exists() {
        bail!("sauvegarde précédente présente : intervention requise");
    }
    let mut moved = false;
    for _ in 0..120 {
        match std::fs::rename(target, &backup) {
            Ok(()) => {
                moved = true;
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(500)),
        }
    }
    if !moved {
        bail!("application toujours ouverte ou dossier non accessible en écriture");
    }
    if let Err(error) = std::fs::copy(staged, target) {
        std::fs::rename(&backup, target).context("restauration de l’ancien client échouée")?;
        return Err(error).context("installation du nouveau client échouée");
    }
    match Command::new(target).current_dir(working_dir).spawn() {
        Ok(_) => {
            let _ = std::fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            std::fs::remove_file(target)?;
            std::fs::rename(backup, target)?;
            Err(error).context("redémarrage échoué ; ancienne version restaurée")
        }
    }
}
