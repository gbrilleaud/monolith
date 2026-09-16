use monolith::{
    models::{GameMetadata, LaunchAvailability, RomAvailability, RomLocation},
    ui::{association_candidates, linked_locations_for_game},
};

fn game(game_id: i64, system_id: i64, title: &str) -> GameMetadata {
    GameMetadata {
        game_id,
        system_id,
        system_name: "Console".into(),
        title: title.into(),
        description: String::new(),
        cover_art: None,
        language: "fr".into(),
        launch_availability: LaunchAvailability::default(),
    }
}

fn location(path: &str, game_id: Option<i64>) -> RomLocation {
    RomLocation {
        id: None,
        game_id,
        system_id: 42,
        path: path.into(),
        extension: "iso".into(),
        size_bytes: 1,
        modified_at: None,
        sha256: None,
        availability: RomAvailability::Available,
        last_seen_at: String::new(),
    }
}

#[test]
fn linked_locations_for_game_returns_only_selected_games_paths() {
    let linked = linked_locations_for_game(
        vec![
            location("/roms/tekken.iso", Some(7)),
            location("/roms/rayman.iso", Some(8)),
            location("/roms/unlinked.iso", None),
        ],
        7,
    );

    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].path, "/roms/tekken.iso");
}

#[test]
fn association_candidates_keep_only_same_system_and_matching_title() {
    let candidates = association_candidates(
        vec![
            game(1, 42, "Tekken 5"),
            game(2, 42, "Ridge Racer V"),
            game(3, 7, "Tekken"),
        ],
        42,
        "tek",
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].game_id, 1);
}

#[test]
fn association_candidates_are_case_insensitive_and_empty_search_lists_system_games() {
    let candidates = association_candidates(
        vec![
            game(1, 42, "Rayman 2"),
            game(2, 42, "Tekken 5"),
            game(3, 7, "Rayman"),
        ],
        42,
        "",
    );

    assert_eq!(
        candidates
            .into_iter()
            .map(|game| game.game_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}
