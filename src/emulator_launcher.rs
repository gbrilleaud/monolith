use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EmulatorConfig {
    #[serde(default, rename = "profile")]
    pub profiles: Vec<EmulatorProfile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmulatorProfile {
    pub system_id: i64,
    pub executable: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub executable: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EmulatorLauncher {
    profiles: HashMap<i64, EmulatorProfile>,
    controller_root: Option<PathBuf>,
    managed_root: Option<PathBuf>,
    managed_data_root: Option<PathBuf>,
}

impl EmulatorLauncher {
    /// Advanced TOML profiles override managed defaults.
    pub fn load_managed(path: &Path, install_root: &Path, data_root: &Path) -> Result<Self> {
        if !install_root.is_absolute() {
            bail!("le répertoire des moteurs doit être absolu");
        }
        let data_root = if data_root.is_absolute() {
            data_root.to_path_buf()
        } else {
            std::env::current_dir()?.join(data_root)
        };
        let mut launcher = Self::load(path)?;
        launcher.managed_root = Some(install_root.to_path_buf());
        launcher.managed_data_root = Some(data_root.clone());
        launcher.controller_root = Some(data_root);
        Ok(launcher)
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Self::from_config(EmulatorConfig::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("lecture de {}", path.display()))?;
        let config: EmulatorConfig = toml::from_str(&text)
            .with_context(|| format!("configuration émulateurs invalide : {}", path.display()))?;
        let mut launcher = Self::from_config(config)?;
        launcher.controller_root = path.parent().map(Path::to_path_buf);
        Ok(launcher)
    }

    pub fn from_config(config: EmulatorConfig) -> Result<Self> {
        let mut profiles = HashMap::new();
        for profile in config.profiles {
            if profile.system_id <= 0 {
                bail!("system_id émulateur doit être positif");
            }
            if profile.executable.trim().is_empty() {
                bail!(
                    "exécutable émulateur absent pour le système {}",
                    profile.system_id
                );
            }
            if !profile.arguments.iter().any(|argument| argument == "{rom}") {
                bail!(
                    "le profil émulateur du système {} doit contenir l'argument {{rom}}",
                    profile.system_id
                );
            }
            if profiles.insert(profile.system_id, profile).is_some() {
                bail!("profil émulateur dupliqué pour le système");
            }
        }
        Ok(Self {
            profiles,
            controller_root: None,
            managed_root: None,
            managed_data_root: None,
        })
    }

    pub fn command_for(&self, system_id: i64, rom_path: &Path) -> Result<LaunchCommand> {
        self.command_for_named(system_id, None, rom_path)
    }

    /// Catalog names take priority over unstable SQLite identifiers. Explicit
    /// advanced profiles remain user-controlled overrides.
    pub fn command_for_named(
        &self,
        system_id: i64,
        system_name: Option<&str>,
        rom_path: &Path,
    ) -> Result<LaunchCommand> {
        if !self.profiles.contains_key(&system_id) {
            if let Some(root) = &self.managed_root {
                let identity = crate::system_identity::detect_system_identity(
                    system_name,
                    None,
                    Some(system_id),
                );
                return self.managed_command_for(root, identity, rom_path);
            }
        }
        let profile = self
            .profiles
            .get(&system_id)
            .with_context(|| format!("aucun émulateur configuré pour le système {system_id}"))?;
        if !rom_path.is_file() {
            bail!("ROM locale introuvable : {}", rom_path.display());
        }
        let rom = rom_path.to_string_lossy();
        let mut arguments: Vec<String> = profile
            .arguments
            .iter()
            .map(|argument| argument.replace("{rom}", &rom))
            .collect();
        // RetroArch accepts a per-console overlay without changing the user's
        // global config. Never pass this option to PCSX2 or Xenia.
        if Path::new(&profile.executable)
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("retroarch"))
        {
            if let Some(root) = &self.controller_root {
                let configured = crate::controller_profiles::ControllerConfig::load(root)?;
                let overlay = crate::controller_profiles::retroarch_path(root, system_id);
                if configured.profiles.iter().any(|p| p.system_id == system_id) && overlay.is_file()
                {
                    arguments.splice(
                        0..0,
                        [
                            "--appendconfig".into(),
                            overlay.to_string_lossy().into_owned(),
                        ],
                    );
                }
            }
        }
        Ok(LaunchCommand {
            executable: profile.executable.clone(),
            arguments,
        })
    }

    fn managed_command_for(
        &self,
        root: &Path,
        identity: crate::system_identity::SystemIdentity,
        rom_path: &Path,
    ) -> Result<LaunchCommand> {
        use crate::system_identity::SystemIdentity;
        if identity == SystemIdentity::PlayStation2 {
            if !rom_path.is_file() {
                bail!("ROM locale introuvable : {}", rom_path.display());
            }
            let executable = root.join("tools/pcsx2").join(if cfg!(windows) {
                "pcsx2-qt.exe"
            } else {
                "pcsx2-qt"
            });
            if !executable.is_file() {
                bail!(
                    "composant manquant : {} ; installer PCSX2",
                    executable.display()
                );
            }
            let data = self
                .managed_data_root
                .as_ref()
                .context("données PCSX2 introuvables")?
                .join("pcsx2");
            let bios = data.join("bios");
            let has_bios = bios.read_dir().ok().into_iter().flatten().any(|entry| {
                entry.ok().is_some_and(|file| {
                    file.path().is_file()
                        && file
                            .path()
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"))
                })
            });
            if !has_bios {
                bail!(
                    "BIOS PS2 personnel manquant : importer un fichier .bin dans {}",
                    bios.display()
                );
            }
            return Ok(LaunchCommand {
                executable: executable.to_string_lossy().into_owned(),
                arguments: vec![
                    "-datapath".into(),
                    data.to_string_lossy().into_owned(),
                    "-batch".into(),
                    "--".into(),
                    rom_path.to_string_lossy().into_owned(),
                ],
            });
        }
        if !matches!(
            identity,
            SystemIdentity::MegaDrive | SystemIdentity::MasterSystem | SystemIdentity::GameGear
        ) {
            bail!("aucune association automatique pour le système {} ; installer le moteur compatible ou définir un profil avancé", identity.as_key());
        }
        if !rom_path.is_file() {
            bail!("ROM locale introuvable : {}", rom_path.display());
        }
        let directory = root.join("tools/retroarch");
        let executable = directory.join(if cfg!(windows) {
            "retroarch.exe"
        } else {
            "retroarch"
        });
        let core = directory.join("cores").join(if cfg!(windows) {
            "genesis_plus_gx_libretro.dll"
        } else {
            "genesis_plus_gx_libretro.so"
        });
        for component in [&executable, &core] {
            if !component.is_file() {
                bail!("composant manquant : {} ; réparer ou installer RetroArch et son cœur Genesis Plus GX", component.display());
            }
        }
        let data = self
            .managed_data_root
            .as_ref()
            .context("données RetroArch introuvables")?;
        let settings = data.join("retroarch");
        let arguments = vec![
            "-L".into(),
            core.to_string_lossy().into_owned(),
            "--config".into(),
            settings
                .join("retroarch.cfg")
                .to_string_lossy()
                .into_owned(),
            "--appendconfig".into(),
            settings
                .join("monolith_paths.cfg")
                .to_string_lossy()
                .into_owned(),
            rom_path.to_string_lossy().into_owned(),
        ];
        Ok(LaunchCommand {
            executable: executable.to_string_lossy().into_owned(),
            arguments,
        })
    }

    fn prepare_managed_retroarch(&self, system_id: i64) -> Result<()> {
        let data = self
            .managed_data_root
            .as_ref()
            .context("données RetroArch introuvables")?;
        let settings = data.join("retroarch");
        std::fs::create_dir_all(&settings)?;
        let base = settings.join("retroarch.cfg");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&base)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(b"# Reglages personnels RetroArch ; conserves lors des mises a jour.\nconfig_save_on_exit = \"true\"\n")?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| format!("création de {}", base.display()))
            }
        }
        let mut overlay =
            String::from("# Chemins Monolith régénérés au lancement ; ne pas éditer.\n");
        if let Some(root) = &self.controller_root {
            let configured = crate::controller_profiles::ControllerConfig::load(root)?;
            let user_overlay = crate::controller_profiles::retroarch_path(root, system_id);
            if configured.profiles.iter().any(|p| p.system_id == system_id)
                && user_overlay.is_file()
            {
                overlay.push_str(&std::fs::read_to_string(&user_overlay)?);
                overlay.push('\n');
            }
        }
        for (key, subdir) in [
            ("savefile_directory", "saves"),
            ("savestate_directory", "states"),
            ("system_directory", "firmware"),
            ("input_remapping_directory", "remaps"),
        ] {
            let directory = settings.join(subdir);
            std::fs::create_dir_all(&directory)?;
            let path = directory.to_string_lossy();
            if path.contains(['\n', '\r', '"']) {
                bail!(
                    "chemin RetroArch non représentable : {}",
                    directory.display()
                );
            }
            #[cfg(windows)]
            let path = path.replace('\\', "/");
            overlay.push_str(&format!("{key} = \"{path}\"\n"));
        }
        let generated = settings.join("monolith_paths.cfg");
        if std::fs::read_to_string(&generated).ok().as_deref() != Some(overlay.as_str()) {
            std::fs::write(&generated, overlay)
                .with_context(|| format!("écriture de {}", generated.display()))?;
        }
        Ok(())
    }

    pub fn launch(&self, system_id: i64, rom_path: &Path) -> Result<LaunchCommand> {
        self.launch_named(system_id, None, rom_path)
    }

    pub fn launch_named(
        &self,
        system_id: i64,
        system_name: Option<&str>,
        rom_path: &Path,
    ) -> Result<LaunchCommand> {
        let command = self.command_for_named(system_id, system_name, rom_path)?;
        if !self.profiles.contains_key(&system_id) && self.managed_root.is_some() {
            let identity =
                crate::system_identity::detect_system_identity(system_name, None, Some(system_id));
            if matches!(
                identity,
                crate::system_identity::SystemIdentity::MegaDrive
                    | crate::system_identity::SystemIdentity::MasterSystem
                    | crate::system_identity::SystemIdentity::GameGear
            ) {
                self.prepare_managed_retroarch(system_id)?;
            }
        }
        let mut child = Command::new(&command.executable);
        child.args(&command.arguments);
        if let Some(directory) = Path::new(&command.executable)
            .parent()
            .filter(|p| p.is_dir())
        {
            child.current_dir(directory);
        }
        child
            .spawn()
            .with_context(|| format!("lancement de {}", command.executable))?;
        Ok(command)
    }
}

/// Locate replaceable engines relative to the running bundle by default.
/// A stale persisted path never prevents a moved portable bundle from working.
pub fn resolve_install_root(data_directory: &Path, executable: &Path) -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("MONOLITH_INSTALL_ROOT") {
        let path = PathBuf::from(explicit);
        if !path.is_absolute() {
            bail!("MONOLITH_INSTALL_ROOT doit être absolu");
        }
        return Ok(path);
    }
    let config =
        crate::client_config::ClientConfig::load_or_default(&data_directory.join("client.toml"))?;
    if let Some(root) = config.install_root {
        let path = PathBuf::from(root);
        if path.exists() {
            return Ok(path);
        }
    }
    let parent = executable
        .parent()
        .context("emplacement Monolith introuvable")?;
    if !parent.is_absolute() {
        bail!("emplacement Monolith doit être absolu");
    }
    Ok(parent.to_path_buf())
}

pub fn default_config_path(data_directory: &Path) -> PathBuf {
    data_directory.join("emulators.toml")
}
