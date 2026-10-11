use monolith::{
    db::Database,
    emulator_launcher::{EmulatorConfig, EmulatorLauncher, EmulatorProfile},
    library_service::LibraryService,
    models::{GameMetadata, LaunchAvailability},
};

#[test]
fn library_preserves_linked_games_and_unknown_offline_roms() {
    let folder = tempfile::tempdir().unwrap();
    let known = folder.path().join("Rayman.gdi");
    let unknown = folder.path().join("Mystery.gdi");
    std::fs::write(&known, b"known").unwrap();
    std::fs::write(&unknown, b"unknown").unwrap();
    let db = Database::open_in_memory().unwrap();
    db.upsert_game(&GameMetadata {
        game_id: 101,
        system_id: 7,
        system_name: "Dreamcast".into(),
        title: "Rayman 2".into(),
        description: "Description française".into(),
        cover_art: Some("/covers/rayman.png".into()),
        language: "fr".into(),
        launch_availability: LaunchAvailability::default(),
    })
    .unwrap();
    db.register_local_rom(7, &known).unwrap();
    db.register_local_rom(7, &unknown).unwrap();
    db.link_rom_location_to_game(&known.to_string_lossy(), 101)
        .unwrap();
    let library = LibraryService::new(db);
    let sections = library.sections(1, "*").unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "Dreamcast");
    assert_eq!(sections[0].games.len(), 2);
    let rayman = sections[0]
        .games
        .iter()
        .find(|g| g.game_id == Some(101))
        .unwrap();
    assert_eq!(
        rayman.cover_uri.as_deref(),
        Some("file:///covers/rayman.png")
    );
    assert!(rayman.available);
    let mystery = sections[0]
        .games
        .iter()
        .find(|g| g.game_id.is_none())
        .unwrap();
    assert_eq!(mystery.title, "Mystery");
    assert!(mystery.cover_uri.is_none());
    assert_eq!(library.sections(1, "ray*").unwrap()[0].games.len(), 1);
}

#[test]
fn launch_preview_resolves_location_from_database_and_rejects_missing_rom() {
    let folder = tempfile::tempdir().unwrap();
    let rom = folder.path().join("Game.gdi");
    std::fs::write(&rom, b"rom").unwrap();
    let db = Database::open_in_memory().unwrap();
    db.register_local_rom(7, &rom).unwrap();
    let library = LibraryService::new(db);
    let launcher = EmulatorLauncher::from_config(EmulatorConfig {
        profiles: vec![EmulatorProfile {
            system_id: 7,
            executable: "emulator".into(),
            arguments: vec!["{rom}".into()],
        }],
    })
    .unwrap();
    let command = library
        .command_for_rom(&launcher, &rom.to_string_lossy())
        .unwrap();
    assert_eq!(command.arguments, vec![rom.to_string_lossy().to_string()]);
    assert!(library
        .command_for_rom(&launcher, "/tmp/never-in-catalog.gdi")
        .is_err());
    std::fs::remove_file(&rom).unwrap();
    assert!(library
        .command_for_rom(&launcher, &rom.to_string_lossy())
        .is_err());
}
