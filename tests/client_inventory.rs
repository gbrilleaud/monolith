use monolith::{
    client_inventory::{ClientInventory, InventoryRefreshReport, InventoryRefreshState},
    db::Database,
    models::ScanRoot,
};
use std::{fs, thread, time::Duration};

#[test]
fn inventory_refresh_state_transitions_from_scanning_to_completed() {
    let mut state = InventoryRefreshState::Scanning;
    state.complete(Ok(InventoryRefreshReport {
        visited: 3,
        accepted: 2,
        ignored: 1,
        missing: 1,
        issues: 0,
    }));

    assert_eq!(
        state,
        InventoryRefreshState::Completed(InventoryRefreshReport {
            visited: 3,
            accepted: 2,
            ignored: 1,
            missing: 1,
            issues: 0,
        })
    );
}

#[test]
fn inventory_refresh_state_surfaces_a_background_error() {
    let mut state = InventoryRefreshState::Scanning;
    state.complete(Err("racine inaccessible".into()));

    assert_eq!(
        state,
        InventoryRefreshState::Error("racine inaccessible".into())
    );
}

#[test]
fn background_scan_updates_sqlite_and_local_cache() {
    let directory = tempfile::tempdir().unwrap();
    let root_path = directory.path().join("roms");
    fs::create_dir(&root_path).unwrap();
    fs::write(root_path.join("Rayman 2.ISO"), b"disc-image").unwrap();
    let database_path = directory.path().join("monolith.db");
    let cache_path = directory.path().join("catalog.json");
    let mut inventory = ClientInventory::new(
        &database_path,
        vec![ScanRoot {
            system_id: 2,
            path: root_path.display().to_string(),
            extensions: vec!["iso".into()],
        }],
        1,
        &cache_path,
    );

    inventory.start().unwrap();
    for _ in 0..100 {
        if inventory.poll().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        inventory.state(),
        &InventoryRefreshState::Completed(InventoryRefreshReport {
            visited: 1,
            accepted: 1,
            ignored: 0,
            missing: 0,
            issues: 0,
        })
    );
    assert!(cache_path.exists());
    assert_eq!(
        Database::open(&database_path)
            .unwrap()
            .rom_locations()
            .unwrap()
            .len(),
        1
    );
}
