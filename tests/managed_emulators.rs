use monolith::emulator_launcher::{resolve_install_root, EmulatorLauncher};
use std::fs;

#[test]
fn managed_launcher_accepts_default_relative_data_directory() {
    let sandbox = tempfile::tempdir().unwrap();
    let install = sandbox.path().join("bundle");
    let launcher = EmulatorLauncher::load_managed(
        &sandbox.path().join("absent.toml"),
        &install,
        std::path::Path::new("data"),
    );
    assert!(launcher.is_ok(), "{launcher:?}");
}

#[test]
fn moved_bundle_prefers_the_executable_directory_over_missing_stale_install_root() {
    let temp = tempfile::tempdir().unwrap();
    let moved = temp.path().join("Moved Games é");
    fs::create_dir_all(moved.join("tools/retroarch")).unwrap();
    let data = temp.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let config = monolith::client_config::ClientConfig::new("http://127.0.0.1:8787")
        .unwrap()
        .with_install_root(&temp.path().join("missing old location"))
        .unwrap();
    config.save(&data.join("client.toml")).unwrap();
    assert_eq!(
        resolve_install_root(&data, &moved.join("monolith.exe")).unwrap(),
        moved
    );
}

#[test]
fn catalog_name_selects_sega_core_without_using_a_numeric_sqlite_id() {
    let temp = tempfile::tempdir().unwrap();
    let install = temp.path().join("bundle");
    let core = if cfg!(windows) {
        "genesis_plus_gx_libretro.dll"
    } else {
        "genesis_plus_gx_libretro.so"
    };
    let executable = if cfg!(windows) {
        "retroarch.exe"
    } else {
        "retroarch"
    };
    let engine = install.join("tools/retroarch");
    fs::create_dir_all(engine.join("cores")).unwrap();
    fs::write(engine.join(executable), b"portable executable").unwrap();
    fs::write(engine.join("cores").join(core), b"core").unwrap();
    let launcher = EmulatorLauncher::load_managed(
        &temp.path().join("none.toml"),
        &install,
        &temp.path().join("data"),
    )
    .unwrap();
    let rom = temp.path().join("game.md");
    fs::write(&rom, b"fixture").unwrap();
    for system in ["Mega Drive / Genesis", "Master System", "Game Gear"] {
        let command = launcher.command_for_named(97, Some(system), &rom).unwrap();
        assert_eq!(
            command.arguments[1],
            engine.join("cores").join(core).to_string_lossy()
        );
    }
    assert!(launcher
        .command_for_named(11, Some("PlayStation 2"), &rom)
        .is_err());
}

#[test]
fn existing_profile_overrides_managed_engine_without_rewriting_settings() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("user data");
    fs::create_dir_all(&data).unwrap();
    let config = data.join("emulators.toml");
    let text =
        "[[profile]]\nsystem_id = 11\nexecutable = 'custom-retroarch'\narguments = ['{rom}']\n";
    fs::write(&config, text).unwrap();
    let rom = temp.path().join("test.zip");
    fs::write(&rom, b"homebrew").unwrap();
    let launcher = EmulatorLauncher::load_managed(&config, temp.path(), &data).unwrap();
    let command = launcher.command_for(11, &rom).unwrap();
    assert_eq!(command.executable, "custom-retroarch");
    assert_eq!(fs::read_to_string(&config).unwrap(), text);
}

#[cfg(unix)]
#[test]
fn managed_launch_runs_in_engine_directory_with_spaced_unicode_paths() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    let temp = tempfile::tempdir().unwrap();
    let install = temp.path().join("jeux déplacés");
    let engine = install.join("tools/retroarch");
    fs::create_dir_all(engine.join("cores")).unwrap();
    fs::write(engine.join("cores/genesis_plus_gx_libretro.so"), b"core").unwrap();
    let executable = engine.join("retroarch");
    fs::write(&executable, b"#!/bin/sh\npwd >> ../launch-cwd.txt\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let data = temp.path().join("données personnelles");
    let launcher =
        EmulatorLauncher::load_managed(&temp.path().join("absent.toml"), &install, &data).unwrap();
    let rom = temp.path().join("ROMs avec espaces/Étoile de la mort.md");
    fs::create_dir_all(rom.parent().unwrap()).unwrap();
    fs::write(&rom, b"redistributable fake ROM fixture").unwrap();
    launcher.launch(11, &rom).unwrap();
    let receipt = install.join("tools/launch-cwd.txt");
    for _ in 0..100 {
        if receipt.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&receipt).unwrap().trim(),
        engine.display().to_string()
    );
    let user_config = data.join("retroarch/retroarch.cfg");
    let managed_paths = data.join("retroarch/monolith_paths.cfg");
    assert!(user_config.is_file());
    let paths = fs::read_to_string(managed_paths).unwrap();
    for directory in ["saves", "states", "firmware"] {
        assert!(paths.contains(&data.join("retroarch").join(directory).display().to_string()));
    }
    assert!(!engine.join("retroarch.cfg").exists());
    fs::write(&user_config, "video_driver = \"gl\"\n").unwrap();
    launcher.launch(11, &rom).unwrap();
    assert_eq!(
        fs::read_to_string(user_config).unwrap(),
        "video_driver = \"gl\"\n"
    );
    for _ in 0..100 {
        if fs::read_to_string(&receipt).unwrap().lines().count() == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(receipt).unwrap().lines().count(), 2);
}

#[test]
fn genesis_launch_requires_managed_retroarch_core_without_toml() {
    let temp = tempfile::tempdir().unwrap();
    let install = temp.path().join("Monolith déplacé é");
    let data = temp.path().join("Données joueur");
    let engine = install.join("tools/retroarch");
    fs::create_dir_all(engine.join("cores")).unwrap();
    fs::create_dir_all(&data).unwrap();
    let exe = if cfg!(windows) {
        "retroarch.exe"
    } else {
        "retroarch"
    };
    let core = if cfg!(windows) {
        "genesis_plus_gx_libretro.dll"
    } else {
        "genesis_plus_gx_libretro.so"
    };
    fs::write(engine.join(exe), b"engine").unwrap();
    let rom = temp.path().join("Ma ROM é.zip");
    fs::write(&rom, b"homebrew").unwrap();
    let launcher =
        EmulatorLauncher::load_managed(&data.join("emulators.toml"), &install, &data).unwrap();
    let missing = launcher.command_for(11, &rom).unwrap_err().to_string();
    assert!(missing.contains("genesis_plus_gx"), "{missing}");
    fs::write(engine.join("cores").join(core), b"core").unwrap();
    let command = launcher.command_for(11, &rom).unwrap();
    assert_eq!(command.executable, engine.join(exe).to_string_lossy());
    assert_eq!(
        command.arguments,
        vec![
            "-L",
            engine.join("cores").join(core).to_str().unwrap(),
            "--config",
            data.join("retroarch")
                .join("retroarch.cfg")
                .to_str()
                .unwrap(),
            "--appendconfig",
            data.join("retroarch")
                .join("monolith_paths.cfg")
                .to_str()
                .unwrap(),
            rom.to_str().unwrap()
        ]
    );
}

#[test]
fn managed_ps2_requires_private_bios_and_uses_separate_data_directory() {
    let temp = tempfile::tempdir().unwrap();
    let install = temp.path().join("Bundle é");
    let data = temp.path().join("Données PS2");
    let engine = install.join("tools/pcsx2");
    fs::create_dir_all(&engine).unwrap();
    let executable = if cfg!(windows) {
        "pcsx2-qt.exe"
    } else {
        "pcsx2-qt"
    };
    fs::write(engine.join(executable), b"fixture").unwrap();
    let rom = temp.path().join("Jeu é.iso");
    fs::write(&rom, b"fixture").unwrap();
    let launcher =
        EmulatorLauncher::load_managed(&data.join("none.toml"), &install, &data).unwrap();
    let missing = launcher
        .command_for_named(97, Some("PlayStation 2"), &rom)
        .unwrap_err()
        .to_string();
    assert!(missing.contains("BIOS"), "{missing}");
    fs::create_dir_all(data.join("pcsx2/bios")).unwrap();
    fs::write(
        data.join("pcsx2/bios/own-bios.bin"),
        b"private test fixture",
    )
    .unwrap();
    let command = launcher
        .command_for_named(97, Some("PlayStation 2"), &rom)
        .unwrap();
    assert_eq!(
        command.executable,
        engine.join(executable).to_string_lossy()
    );
    assert_eq!(
        command.arguments,
        vec![
            "-datapath",
            data.join("pcsx2").to_str().unwrap(),
            "-batch",
            "--",
            rom.to_str().unwrap()
        ]
    );
}
