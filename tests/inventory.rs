use monolith::{
    inventory::scan_root,
    models::{RomAvailability, RomLocation, ScanRoot},
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
