#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::{bail, Result};
#[cfg(target_os = "windows")]
use std::{fs, process::Command};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

pub const RETROARCH_WINDOWS_X64_SETUP_URL: &str =
    "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch-Win64-setup.exe";
pub const PCSX2_WINDOWS_X64_SETUP_URL: &str =
    "https://github.com/PCSX2/pcsx2/releases/download/v2.8.0/PCSX2-v2.8.0-windows-x64-installer.exe";
pub const XENIA_CANARY_WINDOWS_X64_ARCHIVE_URL: &str =
    "https://github.com/xenia-canary/xenia-canary/releases/latest/download/xenia_canary_windows.7z";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RetroArchInstallState {
    #[default]
    Idle,
    Downloading,
    InstallerStarted(PathBuf),
    Error(String),
}

pub struct RetroArchInstaller {
    receiver: Option<Receiver<std::result::Result<PathBuf, String>>>,
    state: RetroArchInstallState,
}

impl Default for RetroArchInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl RetroArchInstaller {
    pub fn new() -> Self {
        Self {
            receiver: None,
            state: RetroArchInstallState::Idle,
        }
    }

    pub fn state(&self) -> &RetroArchInstallState {
        &self.state
    }

    pub fn start(&mut self, install_root: impl AsRef<Path>) -> Result<()> {
        self.start_download(
            install_root.as_ref(),
            retroarch_install_directory,
            RETROARCH_WINDOWS_X64_SETUP_URL,
            "RetroArch-Win64-setup.exe",
        )
    }

    pub fn start_pcsx2(&mut self, install_root: impl AsRef<Path>) -> Result<()> {
        self.start_download(
            install_root.as_ref(),
            pcsx2_install_directory,
            PCSX2_WINDOWS_X64_SETUP_URL,
            "PCSX2-v2.8.0-windows-x64-installer.exe",
        )
    }

    pub fn start_xenia(&mut self, install_root: impl AsRef<Path>) -> Result<()> {
        if self.receiver.is_some() {
            bail!("téléchargement d’émulateur déjà en cours");
        }
        let install_directory = xenia_install_directory(install_root.as_ref())?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = download_and_extract_xenia(&install_directory)
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = RetroArchInstallState::Downloading;
        Ok(())
    }

    fn start_download(
        &mut self,
        install_root: &Path,
        directory: fn(&Path) -> Result<PathBuf>,
        url: &'static str,
        setup_name: &'static str,
    ) -> Result<()> {
        if self.receiver.is_some() {
            bail!("téléchargement d’émulateur déjà en cours");
        }
        let install_directory = directory(install_root)?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = download_and_start(&install_directory, url, setup_name)
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = RetroArchInstallState::Downloading;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<&RetroArchInstallState> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(Ok(directory)) => {
                self.receiver = None;
                self.state = RetroArchInstallState::InstallerStarted(directory);
                Some(&self.state)
            }
            Ok(Err(error)) => {
                self.receiver = None;
                self.state = RetroArchInstallState::Error(error);
                Some(&self.state)
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.state =
                    RetroArchInstallState::Error("installation RetroArch interrompue".into());
                Some(&self.state)
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}

pub fn retroarch_install_directory(install_root: &Path) -> Result<PathBuf> {
    if !install_root.is_absolute() {
        bail!("le répertoire d’installation Monolith doit être absolu");
    }
    Ok(install_root.join("tools").join("retroarch"))
}

pub fn pcsx2_install_directory(install_root: &Path) -> Result<PathBuf> {
    if !install_root.is_absolute() {
        bail!("le répertoire d’installation Monolith doit être absolu");
    }
    Ok(install_root.join("tools").join("pcsx2"))
}

pub fn xenia_install_directory(install_root: &Path) -> Result<PathBuf> {
    if !install_root.is_absolute() {
        bail!("le répertoire d’installation Monolith doit être absolu");
    }
    Ok(install_root.join("tools").join("xenia"))
}

pub fn retroarch_setup_path(install_root: &Path) -> Result<PathBuf> {
    if !install_root.is_absolute() {
        bail!("le répertoire d’installation Monolith doit être absolu");
    }
    Ok(install_root
        .join("downloads")
        .join("RetroArch-Win64-setup.exe"))
}

fn download_and_extract_xenia(install_directory: &Path) -> Result<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = install_directory;
        bail!("l’extraction automatique de Xenia est disponible uniquement sous Windows");
    }

    #[cfg(target_os = "windows")]
    {
        let install_root = install_directory
            .parent()
            .and_then(Path::parent)
            .context("répertoire racine Monolith introuvable")?;
        let archive_path = install_root
            .join("downloads")
            .join("xenia_canary_windows.7z");
        let parent = archive_path
            .parent()
            .context("dossier de téléchargement Xenia introuvable")?;
        fs::create_dir_all(parent)?;
        let temporary = archive_path.with_extension("7z.partial");
        let runtime = tokio::runtime::Runtime::new()?;
        let bytes = runtime.block_on(async {
            reqwest::get(XENIA_CANARY_WINDOWS_X64_ARCHIVE_URL)
                .await?
                .error_for_status()?
                .bytes()
                .await
        })?;
        fs::write(&temporary, &bytes)?;
        fs::rename(&temporary, &archive_path)?;
        fs::create_dir_all(install_directory)?;
        let status = Command::new("tar.exe")
            .args([
                "-xf",
                &archive_path.to_string_lossy(),
                "-C",
                &install_directory.to_string_lossy(),
            ])
            .status()
            .context("lancement de tar.exe pour Xenia")?;
        if !status.success() || !install_directory.join("xenia_canary.exe").is_file() {
            bail!("extraction Xenia échouée : Windows 11 récent avec prise en charge .7z requis");
        }
        Ok(install_directory.to_path_buf())
    }
}

fn download_and_start(install_directory: &Path, url: &str, setup_name: &str) -> Result<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (install_directory, url, setup_name);
        bail!("l’installation automatique est disponible uniquement sous Windows");
    }

    #[cfg(target_os = "windows")]
    {
        let install_root = install_directory
            .parent()
            .and_then(Path::parent)
            .context("répertoire racine Monolith introuvable")?;
        let setup_path = install_root.join("downloads").join(setup_name);
        let setup_parent = setup_path
            .parent()
            .context("dossier de téléchargement RetroArch introuvable")?;
        fs::create_dir_all(setup_parent)
            .with_context(|| format!("création de {}", setup_parent.display()))?;
        let temporary = setup_path.with_extension("exe.partial");
        let runtime = tokio::runtime::Runtime::new()?;
        let bytes = runtime
            .block_on(async { reqwest::get(url).await?.error_for_status()?.bytes().await })?;
        fs::write(&temporary, &bytes)
            .with_context(|| format!("écriture de {}", temporary.display()))?;
        fs::rename(&temporary, &setup_path)
            .with_context(|| format!("publication de {}", setup_path.display()))?;
        Command::new(&setup_path)
            .arg(format!("/DIR={}", install_directory.display()))
            .spawn()
            .with_context(|| "lancement de l’installeur RetroArch")?;
        Ok(install_directory.to_path_buf())
    }
}
