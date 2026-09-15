use monolith::{
    db::Database,
    inventory::scan_root,
    models::{RomAvailability, RomLocation, ScanObservation, ScanRoot},
};
use std::fs;

#[test]
fn rom_location_serializes_availability_as_snake_case() {
    let location = RomLocation {
        id: Some(7),
        game_id: None,
        system_id: 42,
        path: "/library/ps2/Tekken 5.iso".into(),
        extension: "iso".into(),
        size_bytes: 4_718_592_000,
        modified_at: Some(1_725_000_000),
        sha256: None,
        availability: RomAvailability::Available,
        last_seen_at: "2026-09-02T23:30:00Z".into(),
    };

    let json = serde_json::to_value(&location).unwrap();
    assert_eq!(json["availability"], "available");
    assert_eq!(
        serde_json::from_value::<RomLocation>(json).unwrap(),
        location
    );
}

#[test]
fn scan_root_requires_an_absolute_path_and_extensions() {
    assert!(ScanRoot {
        system_id: 42,
        path: "/library/ps2".into(),
        extensions: vec!["iso".into(), "chd".into()],
    }
    .validate()
    .is_ok());

    assert!(ScanRoot {
        system_id: 42,
        path: "roms/ps2".into(),
        extensions: vec!["iso".into()],
    }
    .validate()
    .is_err());

    assert!(ScanRoot {
        system_id: 42,
        path: "/library/ps2".into(),
        extensions: vec![],
    }
    .validate()
    .is_err());
}

#[test]
fn scanner_accepts_configured_extensions_without_touching_other_files() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("Tekken 5.ISO"), b"disc-image").unwrap();
    fs::write(directory.path().join("notes.txt"), b"not a rom").unwrap();
    let nested = directory.path().join("bonus");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("Ridge Racer.chd"), b"chd-image").unwrap();

    let report = scan_root(&ScanRoot {
        system_id: 42,
        path: directory.path().display().to_string(),
        extensions: vec!["iso".into(), "chd".into()],
    })
    .unwrap();

    assert_eq!(report.visited, 3);
    assert_eq!(report.ignored, 1);
    assert_eq!(report.issues.len(), 0);
    assert_eq!(report.observations.len(), 2);
    let paths = report
        .observations
        .iter()
        .map(|observation| observation.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn scanner_reports_an_inaccessible_root_without_panicking() {
    let directory = tempfile::tempdir().unwrap();
    let missing_root = directory.path().join("missing");

    let report = scan_root(&ScanRoot {
        system_id: 42,
        path: missing_root.display().to_string(),
        extensions: vec!["iso".into()],
    })
    .unwrap();

    assert_eq!(report.visited, 0);
    assert_eq!(report.accepted, 0);
    assert_eq!(report.ignored, 0);
    assert_eq!(report.observations.len(), 0);
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].path, missing_root.display().to_string());
}

fn scan_root_for(system_id: i64, path: &str) -> ScanRoot {
    ScanRoot {
        system_id,
        path: path.into(),
        extensions: vec!["iso".into(), "chd".into()],
    }
}

fn observation(system_id: i64, path: &str, size_bytes: u64, modified_at: i64) -> ScanObservation {
    ScanObservation {
        system_id,
        path: path.into(),
        extension: path.rsplit('.').next().unwrap().into(),
        size_bytes,
        modified_at: Some(modified_at),
    }
}

#[test]
fn database_persists_a_scan_idempotently() {
    let database = Database::open_in_memory().unwrap();
    let root = scan_root_for(42, "/library/ps2");
    let observations = vec![observation(42, "/library/ps2/Tekken 5.iso", 100, 10)];

    assert_eq!(
        database.sync_rom_inventory(&root, &observations).unwrap(),
        0
    );
    assert_eq!(
        database.sync_rom_inventory(&root, &observations).unwrap(),
        0
    );

    assert_eq!(database.rom_locations().unwrap().len(), 1);
    assert_eq!(
        database.rom_locations().unwrap()[0].availability,
        RomAvailability::Available
    );
}

#[test]
fn database_updates_a_seen_rom_location() {
    let database = Database::open_in_memory().unwrap();
    let root = scan_root_for(42, "/library/ps2");

    database
        .sync_rom_inventory(
            &root,
            &[observation(42, "/library/ps2/Tekken 5.iso", 100, 10)],
        )
        .unwrap();
    database
        .sync_rom_inventory(
            &root,
            &[observation(42, "/library/ps2/Tekken 5.iso", 200, 20)],
        )
        .unwrap();

    let location = database.rom_locations().unwrap().pop().unwrap();
    assert_eq!(location.size_bytes, 200);
    assert_eq!(location.modified_at, Some(20));
    assert_eq!(location.availability, RomAvailability::Available);
}

#[test]
fn database_marks_unseen_locations_missing_only_within_the_scanned_root() {
    let database = Database::open_in_memory().unwrap();
    let ps2_root = scan_root_for(42, "/library/ps2");
    let psp_root = scan_root_for(43, "/library/psp");

    database
        .sync_rom_inventory(
            &ps2_root,
            &[
                observation(42, "/library/ps2/Tekken 5.iso", 100, 10),
                observation(42, "/library/ps2/Ridge Racer.chd", 200, 10),
            ],
        )
        .unwrap();
    database
        .sync_rom_inventory(
            &psp_root,
            &[observation(43, "/library/psp/Wipeout.iso", 300, 10)],
        )
        .unwrap();

    assert_eq!(
        database
            .sync_rom_inventory(
                &ps2_root,
                &[observation(42, "/library/ps2/Tekken 5.iso", 100, 10)],
            )
            .unwrap(),
        1
    );

    let locations = database.rom_locations().unwrap();
    assert_eq!(locations.len(), 3);
    assert_eq!(
        locations
            .iter()
            .find(|location| location.path == "/library/ps2/Ridge Racer.chd")
            .unwrap()
            .availability,
        RomAvailability::Missing
    );
    assert_eq!(
        locations
            .iter()
            .find(|location| location.path == "/library/psp/Wipeout.iso")
            .unwrap()
            .availability,
        RomAvailability::Available
    );
}
