use anyhow::{anyhow, bail, Context, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{copy, BufReader},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleKind {
    SingleFile,
    ZipArchive,
}

#[derive(Debug, Clone)]
pub struct RomBundle {
    pub kind: BundleKind,
    pub path: PathBuf,
    pub display_name: String,
    pub entry_count: usize,
    pub cleanup_after_upload: bool,
}

impl RomBundle {
    pub fn cleanup(&self) -> Result<()> {
        if self.cleanup_after_upload && self.path.exists() {
            std::fs::remove_file(&self.path).with_context(|| {
                format!("suppression archive temporaire {}", self.path.display())
            })?;
        }
        Ok(())
    }
}

pub fn build_rom_bundle(paths: &[PathBuf]) -> Result<RomBundle> {
    if paths.is_empty() {
        bail!("au moins une ROM doit être sélectionnée");
    }
    let entries = validated_entries(paths)?;
    if entries.len() == 1 {
        let path = entries[0].0.clone();
        return Ok(RomBundle {
            kind: BundleKind::SingleFile,
            display_name: entries[0].1.clone(),
            path,
            entry_count: 1,
            cleanup_after_upload: false,
        });
    }
    let stem = Path::new(&entries[0].1)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("nom archive ROM invalide"))?;
    let display_name = format!("{stem}.bundle.zip");
    let path = tempfile::Builder::new()
        .prefix("monolith-rom-")
        .suffix(".zip")
        .tempfile()?
        .into_temp_path()
        .keep()
        .context("conservation archive temporaire")?;
    let result = write_zip(&path, &entries);
    if let Err(error) = result {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(RomBundle {
        kind: BundleKind::ZipArchive,
        path,
        display_name,
        entry_count: entries.len(),
        cleanup_after_upload: true,
    })
}

fn validated_entries(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>> {
    let mut names = BTreeSet::new();
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("ROM sélectionnée inaccessible : {}", path.display()))?;
        if !metadata.is_file() || metadata.len() == 0 {
            bail!("ROM sélectionnée invalide : {}", path.display());
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && !value.contains(['/', '\\', '\r', '\n']))
            .ok_or_else(|| anyhow!("nom de ROM invalide : {}", path.display()))?
            .to_owned();
        if !names.insert(name.to_ascii_lowercase()) {
            bail!("noms de ROM dupliqués dans le bundle : {name}");
        }
        entries.push((path.clone(), name));
    }
    Ok(entries)
}

fn write_zip(path: &Path, entries: &[(PathBuf, String)]) -> Result<()> {
    let output =
        File::create(path).with_context(|| format!("création archive {}", path.display()))?;
    let mut archive = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (path, name) in entries {
        archive.start_file(name, options)?;
        let mut input = BufReader::new(File::open(path)?);
        copy(&mut input, &mut archive)?;
    }
    archive.finish()?;
    Ok(())
}
