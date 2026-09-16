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
        Self::from_config(config)
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
        Ok(Self { profiles })
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
        Ok(LaunchCommand {
            executable: profile.executable.clone(),
            arguments: profile
                .arguments
                .iter()
                .map(|argument| argument.replace("{rom}", &rom))
                .collect(),
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
