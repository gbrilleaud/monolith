use monolith::{
    db::Database,
    models::{GameMetadata, LaunchAvailability},
};
use std::{fs, process::Command};

fn write_config(directory: &std::path::Path, library_root: &std::path::Path) -> std::path::PathBuf {
    let path = directory.join("backend.toml");
    fs::write(
        &path,
        format!(
            r#"
                database_path = "backend.db"

                [[library.roots]]
                system_id = 42
                path = "{}"
                extensions = ["iso"]
            "#,
            library_root.display()
        ),
    )
    .unwrap();
    path
}

fn admin(config: &std::path::Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_monolith-admin"))
        .arg("--config")
        .arg(config)
        .args(arguments)
        .output()
        .unwrap()
}

fn insert_game(config: &std::path::Path, game_id: i64, system_id: i64) {
    let database = Database::open(&config.parent().unwrap().join("backend.db")).unwrap();
    database
        .upsert_game(&GameMetadata {
            game_id,
            system_id,
            system_name: "PlayStation 2".into(),
            title: "Tekken 5".into(),
            description: String::new(),
            cover_art: None,
            language: "fr".into(),
            launch_availability: LaunchAvailability::default(),
        })
        .unwrap();
}

#[test]
fn library_scan_persists_observations_and_status_reports_them() {
    let directory = tempfile::tempdir().unwrap();
    let roms = directory.path().join("roms");
    fs::create_dir(&roms).unwrap();
    fs::write(roms.join("Tekken 5.ISO"), b"disc-image").unwrap();
    fs::write(roms.join("notes.txt"), b"not a rom").unwrap();
    let config = write_config(directory.path(), &roms);

    let scan = admin(&config, &["library", "scan"]);
    assert!(
        scan.status.success(),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );
    let scan_stdout = String::from_utf8(scan.stdout).unwrap();
    assert!(scan_stdout.contains("visités=2"));
    assert!(scan_stdout.contains("acceptés=1"));
    assert!(scan_stdout.contains("ignorés=1"));
    assert!(scan_stdout.contains("absents=0"));

    let status = admin(&config, &["library", "status"]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_stdout = String::from_utf8(status.stdout).unwrap();
    assert!(status_stdout.contains("SYSTEM_ID\tDISPONIBLES\tABSENTS"));
    assert!(status_stdout.contains("42\t1\t0"));
}

#[test]
fn library_status_can_filter_a_single_system() {
    let directory = tempfile::tempdir().unwrap();
    let roms = directory.path().join("roms");
    fs::create_dir(&roms).unwrap();
    fs::write(roms.join("Tekken 5.iso"), b"disc-image").unwrap();
    let config = write_config(directory.path(), &roms);

    assert!(admin(&config, &["library", "scan"]).status.success());
    let status = admin(&config, &["library", "status", "--system-id", "42"]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(String::from_utf8(status.stdout)
        .unwrap()
        .contains("42\t1\t0"));
}

#[test]
fn library_link_and_unlink_update_the_unlinked_listing() {
    let directory = tempfile::tempdir().unwrap();
    let roms = directory.path().join("roms");
    fs::create_dir(&roms).unwrap();
    let rom_path = roms.join("Tekken 5.iso");
    fs::write(&rom_path, b"disc-image").unwrap();
    let config = write_config(directory.path(), &roms);
    assert!(admin(&config, &["library", "scan"]).status.success());
    insert_game(&config, 7, 42);

    let unlinked = admin(&config, &["library", "unlinked"]);
    assert!(unlinked.status.success());
    assert!(String::from_utf8(unlinked.stdout)
        .unwrap()
        .contains(&rom_path.display().to_string()));

    let link = admin(
        &config,
        &["library", "link", &rom_path.display().to_string(), "7"],
    );
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    assert!(String::from_utf8(link.stdout)
        .unwrap()
        .contains("associée au jeu 7"));
    let database = Database::open(&config.parent().unwrap().join("backend.db")).unwrap();
    assert!(
        database
            .base_game(7)
            .unwrap()
            .unwrap()
            .launch_availability
            .available
    );

    let unlinked = admin(&config, &["library", "unlinked"]);
    assert!(unlinked.status.success());
    assert!(!String::from_utf8(unlinked.stdout)
        .unwrap()
        .contains(&rom_path.display().to_string()));

    let unlink = admin(
        &config,
        &["library", "unlink", &rom_path.display().to_string()],
    );
    assert!(unlink.status.success());
    assert!(String::from_utf8(unlink.stdout)
        .unwrap()
        .contains("Association supprimée"));
    assert!(
        !database
            .base_game(7)
            .unwrap()
            .unwrap()
            .launch_availability
            .available
    );
}

#[test]
fn library_link_rejects_a_game_from_another_system() {
    let directory = tempfile::tempdir().unwrap();
    let roms = directory.path().join("roms");
    fs::create_dir(&roms).unwrap();
    let rom_path = roms.join("Tekken 5.iso");
    fs::write(&rom_path, b"disc-image").unwrap();
    let config = write_config(directory.path(), &roms);
    assert!(admin(&config, &["library", "scan"]).status.success());
    insert_game(&config, 9, 99);

    let link = admin(
        &config,
        &["library", "link", &rom_path.display().to_string(), "9"],
    );
    assert!(!link.status.success());
    assert!(String::from_utf8(link.stderr)
        .unwrap()
        .contains("systèmes différents"));
}
