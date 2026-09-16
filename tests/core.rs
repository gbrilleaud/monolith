use monolith::{
    cache::{load_cache, write_cache_atomic},
    db::Database,
    models::{
        GameMetadata, LaunchAvailability, ScanObservation, ScanRoot, UserOverride,
    },
    navigation::{AppView, Navigator},
    sync::SyncEngine,
};

fn sample_game() -> GameMetadata {
    GameMetadata {
        game_id: 7,
        system_id: 2,
        system_name: "Dreamcast".into(),
        title: "Rayman 2".into(),
        description: "Description récoltée".into(),
        cover_art: Some("harvest.jpg".into()),
        language: "fr".into(),
        launch_availability: LaunchAvailability::default(),
    }
}

#[test]
fn user_override_takes_precedence_without_destroying_harvest_data() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 42,
        game_id: 7,
        description: Some("Ma description".into()),
        cover_art: Some("perso.png".into()),
    })
    .unwrap();

    let resolved = db.resolved_game(42, 7).unwrap().unwrap();
    assert_eq!(resolved.description, "Ma description");
    assert_eq!(resolved.cover_art.as_deref(), Some("perso.png"));

    let harvested = db.base_game(7).unwrap().unwrap();
    assert_eq!(harvested.description, "Description récoltée");
    assert_eq!(harvested.cover_art.as_deref(), Some("harvest.jpg"));
}

#[test]
fn empty_override_fields_fall_back_to_harvest_metadata() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 42,
        game_id: 7,
        description: Some("Description locale".into()),
        cover_art: None,
    })
    .unwrap();

    let resolved = db.resolved_game(42, 7).unwrap().unwrap();
    assert_eq!(resolved.description, "Description locale");
    assert_eq!(resolved.cover_art.as_deref(), Some("harvest.jpg"));
}

#[test]
fn cache_round_trip_preserves_resolved_catalog() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&sample_game()).unwrap();
    let snapshot = db.build_cache(42).unwrap();
    let path = std::env::temp_dir().join(format!("monolith-{}.json", uuid::Uuid::new_v4()));

    write_cache_atomic(&path, &snapshot).unwrap();
    let loaded = load_cache(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(loaded, snapshot);
}

#[test]
fn cache_exposes_launch_availability_from_linked_rom_locations() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&sample_game()).unwrap();
    let root = ScanRoot {
        system_id: 2,
        path: "/library/dreamcast".into(),
        extensions: vec!["chd".into()],
    };
    let path = "/library/dreamcast/Rayman 2.chd";
    db.sync_rom_inventory(
        &root,
        &[ScanObservation {
            system_id: 2,
            path: path.into(),
            extension: "chd".into(),
            size_bytes: 123,
            modified_at: Some(10),
        }],
    )
    .unwrap();
    db.assign_rom_location_to_game(path, 7).unwrap();

    let snapshot = db.build_cache(42).unwrap();
    assert_eq!(
        snapshot.games[0].launch_availability,
        LaunchAvailability {
            available: true,
            location_count: 1,
            preferred_path: Some(path.into()),
        }
    );

    db.sync_rom_inventory(&root, &[]).unwrap();
    let snapshot = db.build_cache(42).unwrap();
    assert_eq!(
        snapshot.games[0].launch_availability,
        LaunchAvailability {
            available: false,
            location_count: 0,
            preferred_path: None,
        }
    );
}

#[test]
fn heart_navigation_moves_sequentially_and_can_go_back() {
    let mut nav = Navigator::default();
    assert_eq!(nav.current(), &AppView::Home);

    nav.open_systems();
    assert_eq!(nav.current(), &AppView::Systems);
    nav.open_catalog(2);
    assert_eq!(nav.current(), &AppView::Catalog { system_id: 2 });
    nav.open_details(7);
    assert_eq!(nav.current(), &AppView::Details { game_id: 7 });

    nav.back();
    assert_eq!(nav.current(), &AppView::Catalog { system_id: 2 });
}

#[test]
fn brain_refreshes_the_offline_cache_for_the_current_user() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 9,
        game_id: 7,
        description: Some("Synchronisée".into()),
        cover_art: None,
    })
    .unwrap();
    let path = std::env::temp_dir().join(format!("monolith-sync-{}.json", uuid::Uuid::new_v4()));

    let engine = SyncEngine::new(&db, 9, &path);
    let report = engine.refresh_local_cache().unwrap();
    let cached = load_cache(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(report.games_written, 1);
    assert_eq!(cached.user_id, 9);
    assert_eq!(cached.games[0].description, "Synchronisée");
}
