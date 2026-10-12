use chrono::Utc;
use monolith::{
    auth::Role,
    db::Database,
    flutter_api::load_library,
    models::{GameMetadata, LaunchAvailability, UserOverride},
    session_store::{ClientSession, SessionStore},
};

fn sample_game() -> GameMetadata {
    GameMetadata {
        game_id: 101,
        system_id: 7,
        system_name: "Dreamcast".into(),
        title: "Rayman 2".into(),
        description: "Description générale".into(),
        cover_art: None,
        language: "fr".into(),
        launch_availability: LaunchAvailability::default(),
    }
}

#[test]
fn flutter_library_uses_the_restored_session_profile() {
    let directory = tempfile::tempdir().unwrap();
    let db = Database::open(&directory.path().join("monolith.db")).unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 42,
        game_id: 101,
        description: Some("Description privée du profil 42".into()),
        cover_art: None,
    })
    .unwrap();
    SessionStore::new(directory.path().join("session.json"))
        .save(&ClientSession {
            backend_url: "http://127.0.0.1:8787".into(),
            access_token: "test-token".into(),
            user_id: 42,
            username: "guillaume".into(),
            role: Role::Standard,
            expires_at: Utc::now().timestamp() + 3600,
        })
        .unwrap();

    let sections = load_library(directory.path().display().to_string(), "Ray*".into()).unwrap();
    assert_eq!(
        sections[0].games[0].description,
        "Description privée du profil 42"
    );
}

#[test]
fn flutter_library_does_not_use_expired_profile_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let db = Database::open(&directory.path().join("monolith.db")).unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 42,
        game_id: 101,
        description: Some("Description privée du profil 42".into()),
        cover_art: None,
    })
    .unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    store
        .save(&ClientSession {
            backend_url: "http://127.0.0.1:8787".into(),
            access_token: "test-token".into(),
            user_id: 42,
            username: "guillaume".into(),
            role: Role::Standard,
            expires_at: Utc::now().timestamp() - 1,
        })
        .unwrap();

    let sections = load_library(directory.path().display().to_string(), "Ray*".into()).unwrap();
    assert_eq!(sections[0].games[0].description, "Description générale");
    assert!(!store.path().exists());
}
