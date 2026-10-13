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
}

impl EmulatorLauncher {
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
        })
    }

    pub fn command_for(&self, system_id: i64, rom_path: &Path) -> Result<LaunchCommand> {
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

    pub fn launch(&self, system_id: i64, rom_path: &Path) -> Result<LaunchCommand> {
        let command = self.command_for(system_id, rom_path)?;
        Command::new(&command.executable)
            .args(&command.arguments)
            .spawn()
            .with_context(|| format!("lancement de {}", command.executable))?;
        Ok(command)
    }
}

pub fn default_config_path(data_directory: &Path) -> PathBuf {
    data_directory.join("emulators.toml")
}
