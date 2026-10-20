use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePackageManifest {
    pub component_id: String,
    pub source_url: String,
    pub sha256: String,
    pub executable: PathBuf,
    pub core_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallLimits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
}

impl Default for InstallLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 512 * 1024 * 1024,
            max_entries: 4_096,
            max_file_bytes: 256 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledEnginePackage {
    pub component_id: String,
    pub source_url: String,
    pub sha256: String,
    pub directory: PathBuf,
}

pub fn install_engine_package(
    manifest: &EnginePackageManifest,
    archive_path: &Path,
    install_root: &Path,
    user_data_root: &Path,
    limits: InstallLimits,
) -> Result<InstalledEnginePackage> {
    validate_manifest(manifest)?;
    let normalized_install = normalized_root(install_root)?;
    let normalized_data = normalized_root(user_data_root)?;
    if normalized_install.starts_with(&normalized_data)
        || normalized_data.starts_with(&normalized_install)
    {
        bail!("les moteurs et les données utilisateur doivent être séparés");
    }
    let archive_size = fs::metadata(archive_path)
        .with_context(|| format!("inspection de l’archive {}", archive_path.display()))?
        .len();
    if archive_size > limits.max_archive_bytes {
        bail!("archive au-delà de la limite autorisée");
    }
    let archive_bytes = fs::read(archive_path)
        .with_context(|| format!("lecture de l’archive {}", archive_path.display()))?;
    let actual = format!("{:x}", Sha256::digest(&archive_bytes));
    if actual != manifest.sha256.to_ascii_lowercase() {
        bail!("SHA-256 de l’archive incorrect");
    }

    fs::create_dir_all(install_root)?;
    let stage = tempfile::Builder::new()
        .prefix(".engine-stage-")
        .tempdir_in(install_root)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive_bytes))?;
    if archive.is_empty() || archive.len() > limits.max_entries {
        bail!("nombre d’entrées ZIP au-delà de la limite autorisée");
    }
    let mut seen = HashSet::new();
    let mut total_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("lien symbolique interdit dans l’archive : {}", entry.name());
        }
        let relative = safe_relative_path(entry.name())?;
        if !seen.insert(relative.to_string_lossy().to_lowercase()) {
            bail!("chemin ZIP dupliqué : {}", entry.name());
        }
        if entry.size() > limits.max_file_bytes {
            bail!(
                "entrée ZIP au-delà de la limite autorisée : {}",
                entry.name()
            );
        }
        total_bytes = total_bytes
            .checked_add(entry.size())
            .context("taille ZIP débordante")?;
        if total_bytes > limits.max_total_bytes {
            bail!("contenu ZIP au-delà de la limite totale autorisée");
        }
        let target = stage.path().join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        fs::create_dir_all(target.parent().context("chemin ZIP invalide")?)?;
        let mut output = File::create(&target)?;
        let copied = std::io::copy(&mut entry.take(limits.max_file_bytes + 1), &mut output)?;
        if copied > limits.max_file_bytes {
            bail!("entrée ZIP au-delà de la limite autorisée");
        }
        output.flush()?;
    }
    for required in std::iter::once(&manifest.executable).chain(&manifest.core_files) {
        if !stage.path().join(required).is_file() {
            bail!("composant attendu absent : {}", required.display());
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = stage.path().join(&manifest.executable);
        let mut permissions = fs::metadata(&executable)?.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(executable, permissions)?;
    }

    let destination = install_root.join(&manifest.component_id);
    // Portable emulator installations may keep saves inside their own tree.
    // Until a migration is implemented, refusing an update is safer than
    // silently discarding those files when the old tree is removed.
    for personal in [
        "saves",
        "states",
        "savestates",
        "memcards",
        "bios",
        "content",
        "portable.ini",
        "portable.txt",
    ] {
        if destination.join(personal).exists() {
            bail!(
                "données personnelles dans le moteur existant : {} ; migrer avant la réparation",
                destination.join(personal).display()
            );
        }
    }
    let backup = install_root.join(format!(".{}.rollback", manifest.component_id));
    if backup.exists() {
        bail!("sauvegarde de moteur préexistante : intervention requise");
    }
    let staged = stage.keep();
    let had_previous = destination.exists();
    if had_previous {
        fs::rename(&destination, &backup).context("sauvegarde de l’ancien moteur")?;
    }
    if let Err(publish_error) = fs::rename(&staged, &destination) {
        if had_previous {
            fs::rename(&backup, &destination)
                .context("publication échouée et restauration impossible")?;
        }
        let _ = fs::remove_dir_all(&staged);
        return Err(publish_error)
            .context("publication du moteur échouée ; ancien moteur restauré");
    }
    if had_previous {
        fs::remove_dir_all(&backup).context("suppression de la sauvegarde du moteur")?;
    }
    Ok(InstalledEnginePackage {
        component_id: manifest.component_id.clone(),
        source_url: manifest.source_url.clone(),
        sha256: actual,
        directory: destination,
    })
}

fn normalized_root(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        bail!(
            "racine de moteur ou de données invalide : {}",
            path.display()
        );
    }
    let mut existing = path;
    let mut missing = Vec::new();
    while !existing.exists() {
        missing.push(
            existing
                .file_name()
                .context("racine invalide")?
                .to_os_string(),
        );
        existing = existing.parent().context("racine invalide")?;
    }
    let mut normalized = existing.canonicalize()?;
    for part in missing.into_iter().rev() {
        normalized.push(part);
    }
    Ok(normalized)
}

fn validate_manifest(manifest: &EnginePackageManifest) -> Result<()> {
    if manifest.component_id.is_empty()
        || manifest.component_id == "."
        || manifest.component_id == ".."
        || manifest.component_id.contains(['/', '\\', ':'])
    {
        bail!("identifiant de composant invalide");
    }
    if !manifest.source_url.starts_with("https://") {
        bail!("URL source épinglée HTTPS requise");
    }
    if manifest.sha256.len() != 64 || !manifest.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("SHA-256 attendu invalide");
    }
    let mut required = HashSet::new();
    for path in std::iter::once(&manifest.executable).chain(&manifest.core_files) {
        let raw = path.to_str().context("chemin attendu non UTF-8")?;
        let safe = safe_relative_path(raw)?;
        if safe != *path || !required.insert(raw.to_lowercase()) {
            bail!("chemin de composant attendu invalide ou dupliqué");
        }
    }
    Ok(())
}

fn safe_relative_path(raw: &str) -> Result<PathBuf> {
    if raw.is_empty() || raw.starts_with('/') || raw.starts_with('\\') || raw.contains('\\') {
        bail!("chemin ZIP dangereux : {raw:?}");
    }
    let trimmed = raw.strip_suffix('/').unwrap_or(raw);
    let mut path = PathBuf::new();
    for (index, component) in trimmed.split('/').enumerate() {
        if component.is_empty()
            || component == "."
            || component == ".."
            || (index == 0 && component.contains(':'))
        {
            bail!("chemin ZIP dangereux : {raw:?}");
        }
        path.push(component);
    }
    Ok(path)
}
