use monolith::emulator_launcher::{EmulatorConfig, EmulatorLauncher, EmulatorProfile};
use std::fs;

#[test]
fn mega_drive_profile_builds_a_retroarch_command_without_shell() {
    let launcher = EmulatorLauncher::from_config(EmulatorConfig {
        profiles: vec![EmulatorProfile {
            system_id: 11,
            executable: "C:\\RetroArch\\retroarch.exe".into(),
            arguments: vec![
                "-L".into(),
                "C:\\RetroArch\\cores\\genesis_plus_gx_libretro.dll".into(),
                "{rom}".into(),
            ],
        }],
    })
    .unwrap();

    let directory = tempfile::tempdir().unwrap();
    let rom_path = directory.path().join("Global Gladiators.zip");
    fs::write(&rom_path, b"test-rom").unwrap();
    let command = launcher.command_for(11, &rom_path).unwrap();

    assert_eq!(command.executable, "C:\\RetroArch\\retroarch.exe");
    assert_eq!(
        command.arguments,
        vec![
            "-L",
            "C:\\RetroArch\\cores\\genesis_plus_gx_libretro.dll",
            rom_path.to_string_lossy().as_ref(),
        ]
    );
}

#[test]
fn launcher_rejects_profile_without_rom_placeholder() {
    let error = EmulatorLauncher::from_config(EmulatorConfig {
        profiles: vec![EmulatorProfile {
            system_id: 11,
            executable: "retroarch.exe".into(),
            arguments: vec!["--menu".into()],
        }],
    })
    .unwrap_err();

    assert!(error.to_string().contains("{rom}"));
}
