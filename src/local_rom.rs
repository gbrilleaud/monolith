use crate::{
    emulator_launcher::{EmulatorLauncher, LaunchCommand},
    models::{RomAvailability, RomLocation},
};
use anyhow::{bail, Result};
use std::path::Path;

pub fn command_for_local_rom(
    launcher: &EmulatorLauncher,
    location: &RomLocation,
) -> Result<LaunchCommand> {
    validate_local_rom(location)?;
    launcher.command_for(location.system_id, Path::new(&location.path))
}

pub fn launch_local_rom(
    launcher: &EmulatorLauncher,
    location: &RomLocation,
) -> Result<LaunchCommand> {
    validate_local_rom(location)?;
    launcher.launch(location.system_id, Path::new(&location.path))
}

fn validate_local_rom(location: &RomLocation) -> Result<()> {
    if location.game_id.is_some() {
        bail!("la ROM locale doit être non associée");
    }
    if location.availability != RomAvailability::Available {
        bail!("la ROM locale est indisponible");
    }
    if location.path.trim().is_empty() {
        bail!("chemin ROM local absent");
    }
    Ok(())
}
