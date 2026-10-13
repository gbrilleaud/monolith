use monolith::controller_profiles::{ControllerConfig, ControllerProfile};
use std::collections::BTreeMap;

#[test]
fn flutter_controller_api_saves_and_resets_one_console() {
    use monolith::flutter_api;
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().to_string_lossy().into_owned();
    let text = "[[profile]]\nsystem_id = 11\ndevice_id = 'xinput:0'\n[profile.buttons]\na = '0'\n";
    flutter_api::save_controller_config(directory.clone(), text.to_string()).unwrap();
    assert!(flutter_api::controller_config_text(directory.clone())
        .unwrap()
        .contains("system_id = 11"));
    flutter_api::reset_controller_config(directory.clone(), 11, "xinput:0".into()).unwrap();
    assert!(!flutter_api::controller_config_text(directory)
        .unwrap()
        .contains("system_id = 11"));
}

#[test]
fn retroarch_launch_uses_only_the_selected_console_overlay() {
    use monolith::emulator_launcher::EmulatorLauncher;
    let root = tempfile::tempdir().unwrap();
    let rom = root.path().join("game.rom");
    std::fs::write(&rom, b"rom").unwrap();
    let config = ControllerConfig {
        profiles: vec![ControllerProfile {
            system_id: 11,
            device_id: "xinput:0".into(),
            buttons: BTreeMap::from([("a".into(), "0".into())]),
        }],
    };
    config.save(root.path()).unwrap();
    std::fs::write(root.path().join("emulators.toml"), "[[profile]]\nsystem_id=11\nexecutable='retroarch.exe'\narguments=['{rom}']\n[[profile]]\nsystem_id=2\nexecutable='pcsx2.exe'\narguments=['{rom}']\n").unwrap();
    let launcher = EmulatorLauncher::load(&root.path().join("emulators.toml")).unwrap();
    let retroarch = launcher.command_for(11, &rom).unwrap();
    assert!(retroarch
        .arguments
        .windows(2)
        .any(|pair| pair[0] == "--appendconfig" && pair[1].contains("system-11.cfg")));
    let pcsx2 = launcher.command_for(2, &rom).unwrap();
    assert!(!pcsx2.arguments.iter().any(|arg| arg == "--appendconfig"));
}

#[test]
fn controller_profiles_are_separate_by_console_and_device() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ControllerConfig::default();
    config.profiles.push(ControllerProfile {
        system_id: 11,
        device_id: "xinput:0".into(),
        buttons: BTreeMap::from([("a".into(), "0".into())]),
    });
    config.profiles.push(ControllerProfile {
        system_id: 2,
        device_id: "xinput:0".into(),
        buttons: BTreeMap::from([("a".into(), "1".into())]),
    });
    config.save(root.path()).unwrap();
    let read = ControllerConfig::load(root.path()).unwrap();
    assert_eq!(read.profiles.len(), 2);
    assert_eq!(read.profiles[0].buttons["a"], "0");
    assert!(root
        .path()
        .join("controllers/retroarch/system-11.cfg")
        .is_file());
    assert!(root
        .path()
        .join("controllers/retroarch/system-2.cfg")
        .is_file());
}

#[test]
fn reset_restores_autoconfig_without_erasing_emulator_settings() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("retroarch.cfg"), "personal = true").unwrap();
    let config = ControllerConfig {
        profiles: vec![ControllerProfile {
            system_id: 11,
            device_id: "xinput:0".into(),
            buttons: BTreeMap::from([("a".into(), "0".into())]),
        }],
    };
    config.save(root.path()).unwrap();
    ControllerConfig::reset(root.path(), 11, "xinput:0").unwrap();
    assert!(ControllerConfig::load(root.path())
        .unwrap()
        .profiles
        .is_empty());
    assert!(!root
        .path()
        .join("controllers/retroarch/system-11.cfg")
        .exists());
    assert_eq!(
        std::fs::read_to_string(root.path().join("retroarch.cfg")).unwrap(),
        "personal = true"
    );
}

#[test]
fn rejects_invalid_mapping_without_touching_previous_export() {
    let root = tempfile::tempdir().unwrap();
    let good = ControllerConfig {
        profiles: vec![ControllerProfile {
            system_id: 11,
            device_id: "xinput:0".into(),
            buttons: BTreeMap::from([("a".into(), "0".into())]),
        }],
    };
    good.save(root.path()).unwrap();
    let target = root.path().join("controllers/retroarch/system-11.cfg");
    let before = std::fs::read(&target).unwrap();
    let bad = ControllerConfig {
        profiles: vec![ControllerProfile {
            system_id: 11,
            device_id: "../../escape".into(),
            buttons: BTreeMap::from([("a".into(), "\"\ninclude evil".into())]),
        }],
    };
    assert!(bad.save(root.path()).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), before);
}
