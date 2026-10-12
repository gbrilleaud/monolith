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
const FLUTTER_ASSET: &str = "monolith-flutter-windows-x86_64.zip";
const FLUTTER_CHECKSUM: &str = "monolith-flutter-windows-x86_64.zip.sha256";
const MAX_ARCHIVE_BYTES: usize = 150 * 1024 * 1024;
const MAX_EXE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    archive_url: String,
    checksum_url: String,
}

#[derive(Debug, Clone, Copy)]
enum Distribution {
    Egui,
    Flutter,
}

impl Distribution {
    fn assets(self) -> (&'static str, &'static str) {
        match self {
            Self::Egui => (ASSET, CHECKSUM),
            Self::Flutter => (FLUTTER_ASSET, FLUTTER_CHECKSUM),
        }
    }
}

fn version(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        bail!("version non prise en charge");
    }
    Ok([parts[0].parse()?, parts[1].parse()?, parts[2].parse()?])
}

pub fn parse_release(value: &Value, current: &str) -> Result<Option<AvailableUpdate>> {
    parse_release_for(value, current, Distribution::Egui)
}

pub fn parse_flutter_release(value: &Value, current: &str) -> Result<Option<AvailableUpdate>> {
    parse_release_for(value, current, Distribution::Flutter)
}

fn parse_release_for(
    value: &Value,
    current: &str,
    distribution: Distribution,
) -> Result<Option<AvailableUpdate>> {
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
    let (archive, checksum) = distribution.assets();
    Ok(match (asset_url(archive), asset_url(checksum)) {
        (Some(archive_url), Some(checksum_url)) => Some(AvailableUpdate {
            version: release_version.to_owned(),
            archive_url,
            checksum_url,
        }),
        _ => None,
    })
}

fn verify_archive_checksum(archive: &[u8], checksum: &str, asset: &str) -> Result<()> {
    if archive.len() > MAX_ARCHIVE_BYTES {
        bail!("archive trop volumineuse");
    }
    let expected_line = checksum.trim_end_matches(['\r', '\n']);
    let (hash, name) = expected_line
        .split_once("  ")
        .context("checksum invalide")?;
    if name != asset || hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("checksum invalide");
    }
    if format!("{:x}", Sha256::digest(archive)) != hash.to_ascii_lowercase() {
        bail!("SHA-256 de l’archive incorrect");
    }
    Ok(())
}

pub fn unpack_verified_client(archive: &[u8], checksum: &str, destination: &Path) -> Result<()> {
    verify_archive_checksum(archive, checksum, ASSET)?;
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

/// Décompresse uniquement le bundle Flutter officiel, sans traversée de chemins ni liens.
pub fn unpack_flutter_bundle(archive: &[u8], checksum: &str, destination: &Path) -> Result<()> {
    use std::path::Component;
    verify_archive_checksum(archive, checksum, FLUTTER_ASSET)?;
    let mut zip =
        zip::ZipArchive::new(Cursor::new(archive)).context("archive Flutter illisible")?;
    if zip.len() > 500 {
        bail!("trop de fichiers dans le bundle Flutter");
    }
    let prefix = "monolith-flutter-windows-x86_64/";
    let mut total = 0_u64;
    let mut names = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let name = file.name();
        let relative = name
            .strip_prefix(prefix)
            .context("racine d’archive invalide")?;
        if relative.is_empty() {
            continue;
        }
        if name.contains('\\') || name.contains(':') {
            bail!("séparateur ou lecteur Windows interdit dans le bundle Flutter");
        }
        let relative = Path::new(relative);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            bail!("chemin dangereux dans le bundle Flutter");
        }
        let key = relative.to_string_lossy().to_ascii_lowercase();
        let allowed = if file.is_dir() {
            key == "data" || key == "data/flutter_assets" || key.starts_with("data/flutter_assets/")
        } else {
            (!key.contains('/')
                && (key.ends_with(".dll")
                    || matches!(
                        key.as_str(),
                        "monolith.exe" | "monolith-flutter-updater.exe"
                    )))
                || matches!(key.as_str(), "data/app.so" | "data/icudtl.dat")
                || key.starts_with("data/flutter_assets/")
        };
        if !allowed {
            bail!("fichier extérieur au bundle Flutter : {key}");
        }
        if !names.insert(key) {
            bail!("fichier en double dans le bundle Flutter");
        }
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("lien symbolique interdit dans le bundle Flutter");
        }
        if file.is_dir() {
            continue;
        }
        if file.size() == 0 || file.size() > MAX_EXE_BYTES {
            bail!("fichier Flutter de taille invalide");
        }
        total = total.saturating_add(file.size());
        if total > 500 * 1024 * 1024 {
            bail!("bundle Flutter décompressé trop volumineux");
        }
        let path = destination.join(relative);
        std::fs::create_dir_all(path.parent().context("chemin Flutter invalide")?)?;
        let mut output = std::fs::File::create(&path)?;
        let copied = std::io::copy(&mut file.take(MAX_EXE_BYTES + 1), &mut output)?;
        if copied == 0 || copied > MAX_EXE_BYTES {
            bail!("fichier Flutter trop volumineux");
        }
    }
    for required in [
        "monolith.exe",
        "monolith.dll",
        "flutter_windows.dll",
        "monolith-flutter-updater.exe",
        "data/app.so",
        "data/icudtl.dat",
    ] {
        if !destination.join(required).is_file() {
            bail!("bundle Flutter incomplet : {required}");
        }
    }
    if !destination.join("data/flutter_assets").is_dir() {
        bail!("ressources Flutter absentes");
    }
    Ok(())
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

async fn latest(
    client: &reqwest::Client,
    distribution: Distribution,
) -> Result<Option<AvailableUpdate>> {
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
    parse_release_for(&value, env!("CARGO_PKG_VERSION"), distribution)
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
    distribution: Distribution,
}
impl ClientUpdate {
    pub fn new(data_dir: PathBuf) -> Self {
        Self::new_for(data_dir, Distribution::Egui)
    }
    pub fn new_flutter(data_dir: PathBuf) -> Self {
        Self::new_for(data_dir, Distribution::Flutter)
    }
    fn new_for(data_dir: PathBuf, distribution: Distribution) -> Self {
        let update = Self {
            state: UpdateState::Unavailable,
            receiver: None,
            data_dir,
            distribution,
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
        let distribution = self.distribution;
        self.spawn(move || {
            let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let client = http_client().map_err(|e| e.to_string())?;
                latest(&client, distribution)
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
        let distribution = self.distribution;
        let (archive_name, _) = distribution.assets();
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
                let checksum = std::str::from_utf8(&checksum).map_err(|e| e.to_string())?;
                let path = match distribution {
                    Distribution::Egui => {
                        let path = dir.join(format!("monolith-{}.exe", release.version));
                        unpack_verified_client(&archive, checksum, &path)
                            .map_err(|e| e.to_string())?;
                        path
                    }
                    Distribution::Flutter => {
                        let path = dir.join(format!("monolith-flutter-{}.zip", release.version));
                        verify_archive_checksum(&archive, checksum, archive_name)
                            .map_err(|e| e.to_string())?;
                        let staged = tempfile::tempdir_in(&dir).map_err(|e| e.to_string())?;
                        unpack_flutter_bundle(&archive, checksum, staged.path())
                            .map_err(|e| e.to_string())?;
                        let partial = path.with_extension("zip.partial");
                        std::fs::write(&partial, &archive).map_err(|e| e.to_string())?;
                        std::fs::rename(&partial, &path).map_err(|e| e.to_string())?;
                        path
                    }
                };
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
        match self.distribution {
            Distribution::Egui => install_and_restart(staged, &self.data_dir),
            Distribution::Flutter => launch_flutter_update(staged, &self.data_dir),
        }
    }
}

#[cfg(target_os = "windows")]
fn launch_flutter_update(archive: &Path, data_dir: &Path) -> Result<()> {
    use std::process::Command;
    let bytes = std::fs::read(archive).context("archive de mise à jour absente")?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let checksum = format!("{digest}  {FLUTTER_ASSET}");
    let updates = data_dir.join("updates");
    std::fs::create_dir_all(&updates)?;
    let stage = tempfile::tempdir_in(&updates)?;
    unpack_flutter_bundle(&bytes, &checksum, stage.path())?;
    let helper = updates.join(format!("monolith-flutter-updater-{}.exe", &digest[..12]));
    std::fs::copy(stage.path().join("monolith-flutter-updater.exe"), &helper)?;
    let target = std::env::current_exe()?;
    Command::new(&helper)
        .arg("--apply-flutter-update")
        .arg(archive)
        .arg(&target)
        .arg(&digest)
        .arg(std::env::current_dir()?)
        .arg(data_dir)
        .spawn()
        .context("démarrage de la mise à jour Flutter")?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_flutter_update(_archive: &Path, _data_dir: &Path) -> Result<()> {
    bail!("mise à jour Flutter intégrée disponible uniquement sur Windows")
}

/// Exécuté par le processus auxiliaire, après fermeture du client Flutter.
pub fn apply_flutter_update(
    archive: &Path,
    target: &Path,
    expected_sha: &str,
    working_dir: &Path,
    data_dir: &Path,
) -> Result<()> {
    if expected_sha.len() != 64 || !expected_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("empreinte de mise à jour invalide");
    }
    let bytes = std::fs::read(archive)?;
    let checksum = format!("{}  {FLUTTER_ASSET}", expected_sha.to_ascii_lowercase());
    let stage = tempfile::tempdir_in(data_dir.join("updates"))?;
    unpack_flutter_bundle(&bytes, &checksum, stage.path())?;
    let install_dir = target
        .parent()
        .context("répertoire d’installation introuvable")?;
    if target.file_name().and_then(|name| name.to_str()) != Some("monolith.exe") {
        bail!("exécutable Flutter inattendu");
    }
    #[cfg(target_os = "windows")]
    {
        for _ in 0..120 {
            if std::fs::OpenOptions::new().write(true).open(target).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    let changed = install_flutter_files(stage.path(), install_dir)?;
    if let Err(error) = std::process::Command::new(target)
        .current_dir(working_dir)
        .spawn()
    {
        restore_flutter_files(install_dir, &changed)
            .context("redémarrage échoué et restauration impossible")?;
        return Err(error).context("redémarrage échoué ; ancien client restauré");
    }
    std::fs::remove_dir_all(install_dir.join(".monolith-flutter-backup"))?;
    Ok(())
}

/// Ne touche qu'aux fichiers présents dans le nouveau bundle (jamais aux ROM, BIOS, ou SQLite).
fn install_flutter_files(stage: &Path, install_dir: &Path) -> Result<Vec<PathBuf>> {
    let backup = install_dir.join(".monolith-flutter-backup");
    if backup.exists() {
        bail!("sauvegarde de mise à jour préexistante : intervention requise");
    }
    std::fs::create_dir(&backup)?;
    let mut files = Vec::new();
    let mut stack = vec![stage.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                stack.push(entry.path());
            } else if entry.file_type()?.is_file() {
                files.push(entry.path().strip_prefix(stage)?.to_path_buf());
            } else {
                bail!("fichier de mise à jour non ordinaire");
            }
        }
    }
    files.sort();
    let mut changed = Vec::new();
    let result = (|| -> Result<()> {
        for relative in &files {
            let source = stage.join(relative);
            let target = install_dir.join(relative);
            let old = backup.join(relative);
            if target.exists() {
                std::fs::create_dir_all(old.parent().context("sauvegarde invalide")?)?;
                std::fs::copy(&target, &old)?;
            }
            changed.push(relative.clone());
            std::fs::create_dir_all(target.parent().context("destination invalide")?)?;
            std::fs::copy(source, target)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        restore_flutter_files(install_dir, &changed)?;
        return Err(error).context("installation Flutter échouée ; ancien client restauré");
    }
    Ok(changed)
}

fn restore_flutter_files(install_dir: &Path, changed: &[PathBuf]) -> Result<()> {
    let backup = install_dir.join(".monolith-flutter-backup");
    for relative in changed.iter().rev() {
        let target = install_dir.join(relative);
        let old = backup.join(relative);
        if old.exists() {
            std::fs::copy(old, target).context("restauration du client échouée")?;
        } else if target.exists() {
            std::fs::remove_file(target).context("suppression du fichier partiel échouée")?;
        }
    }
    std::fs::remove_dir_all(backup)?;
    Ok(())
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

#[cfg(test)]
mod flutter_update_tests {
    use super::*;

    #[test]
    fn replaces_only_bundle_files_and_can_restore_existing_installation() {
        let root = tempfile::tempdir().unwrap();
        let stage = root.path().join("stage");
        let install = root.path().join("install");
        std::fs::create_dir_all(stage.join("data/flutter_assets")).unwrap();
        std::fs::create_dir_all(install.join("data/flutter_assets")).unwrap();
        std::fs::write(stage.join("monolith.exe"), b"new").unwrap();
        std::fs::write(stage.join("data/flutter_assets/game.txt"), b"new").unwrap();
        std::fs::write(install.join("monolith.exe"), b"old").unwrap();
        std::fs::write(install.join("data/private.sqlite"), b"private").unwrap();
        let changed = install_flutter_files(&stage, &install).unwrap();
        assert_eq!(std::fs::read(install.join("monolith.exe")).unwrap(), b"new");
        assert_eq!(
            std::fs::read(install.join("data/private.sqlite")).unwrap(),
            b"private"
        );
        restore_flutter_files(&install, &changed).unwrap();
        assert_eq!(std::fs::read(install.join("monolith.exe")).unwrap(), b"old");
        assert!(!install.join("data/flutter_assets/game.txt").exists());
        assert_eq!(
            std::fs::read(install.join("data/private.sqlite")).unwrap(),
            b"private"
        );
    }
}
