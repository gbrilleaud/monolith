use monolith::{
    emulator_launcher::{EmulatorConfig, EmulatorLauncher, EmulatorProfile},
    local_rom::command_for_local_rom,
    models::{RomAvailability, RomLocation},
};

fn location(path: String) -> RomLocation {
    RomLocation {
        id: Some(1),
        game_id: None,
        system_id: 77,
        path,
        extension: "iso".into(),
        size_bytes: 11,
        modified_at: None,
        sha256: None,
        availability: RomAvailability::Available,
        last_seen_at: "2026-09-18T00:00:00Z".into(),
    }
}

#[test]
fn unknown_available_local_rom_builds_a_launch_command_without_authentication() {
    let directory = tempfile::tempdir().unwrap();
    let rom = directory.path().join("Rayman 2.iso");
    std::fs::write(&rom, b"offline-rom").unwrap();
    let launcher = EmulatorLauncher::from_config(EmulatorConfig {
        profiles: vec![EmulatorProfile {
            system_id: 77,
            executable: "emulator".into(),
            arguments: vec!["--rom".into(), "{rom}".into()],
        }],
    })
    .unwrap();

    let command = command_for_local_rom(&launcher, &location(rom.display().to_string())).unwrap();

    assert_eq!(command.executable, "emulator");
    assert_eq!(
        command.arguments,
        vec!["--rom", rom.to_string_lossy().as_ref()]
    );
}
