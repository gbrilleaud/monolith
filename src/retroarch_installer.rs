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
        if self.receiver.is_some() {
            bail!("installation RetroArch déjà en cours");
        }
        let install_directory = retroarch_install_directory(install_root.as_ref())?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result =
                download_and_start(&install_directory).map_err(|error| format!("{error:#}"));
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

fn download_and_start(install_directory: &Path) -> Result<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = install_directory;
        bail!("l’installation automatique de RetroArch est disponible uniquement sous Windows");
    }

    #[cfg(target_os = "windows")]
    {
        fs::create_dir_all(install_directory)
            .with_context(|| format!("création de {}", install_directory.display()))?;
        let setup_path = install_directory.join("RetroArch-Win64-setup.exe");
        let temporary = setup_path.with_extension("exe.partial");
        let runtime = tokio::runtime::Runtime::new()?;
        let bytes = runtime.block_on(async {
            reqwest::get(RETROARCH_WINDOWS_X64_SETUP_URL)
                .await?
                .error_for_status()?
                .bytes()
                .await
        })?;
        fs::write(&temporary, &bytes)
            .with_context(|| format!("écriture de {}", temporary.display()))?;
        fs::rename(&temporary, &setup_path)
            .with_context(|| format!("publication de {}", setup_path.display()))?;
        Command::new(&setup_path)
            .arg("/VERYSILENT")
            .arg("/SUPPRESSMSGBOXES")
            .arg(format!("/DIR={}", install_directory.display()))
            .spawn()
            .with_context(|| "lancement de l’installeur RetroArch")?;
        Ok(install_directory.to_path_buf())
    }
}
