use chrono::Utc;
use monolith::{
    auth::Role,
    backend::{router, BackendState},
    backend_config::BackendConfig,
    client_config::ClientConfig,
    db::Database,
    flutter_api::{
        backend_url, load_library, login_local, logout, save_backend_url, session_identity,
    },
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

#[test]
fn flutter_session_identity_hides_the_bearer_and_logout_keeps_catalog() {
    let directory = tempfile::tempdir().unwrap();
    let db = Database::open(&directory.path().join("monolith.db")).unwrap();
    db.upsert_game(&sample_game()).unwrap();
    let root = directory.path().display().to_string();
    let store = SessionStore::new(directory.path().join("session.json"));
    store
        .save(&ClientSession {
            backend_url: "http://127.0.0.1:8787".into(),
            access_token: "secret-bearer".into(),
            user_id: 42,
            username: "guillaume".into(),
            role: Role::Standard,
            expires_at: Utc::now().timestamp() + 3600,
        })
        .unwrap();

    let user = session_identity(root.clone()).unwrap().unwrap();
    assert_eq!(user.user_id, 42);
    assert_eq!(user.username, "guillaume");
    assert_eq!(user.role, "standard");
    assert!(!format!("{user:?}").contains("secret-bearer"));
    logout(root.clone()).unwrap();
    assert!(!store.path().exists());
    assert!(session_identity(root.clone()).unwrap().is_none());
    assert_eq!(load_library(root, "Ray*".into()).unwrap().len(), 1);
}

#[tokio::test]
async fn flutter_local_login_refreshes_cache_and_persists_session() {
    let server = tempfile::tempdir().unwrap();
    let backend_db = server.path().join("backend.db");
    let database = Database::open(&backend_db).unwrap();
    let user_id = database
        .create_local_user("guillaume", "safe-password", Role::Standard)
        .unwrap();
    database.upsert_game(&sample_game()).unwrap();
    let config = BackendConfig {
        database_path: backend_db.display().to_string(),
        ..BackendConfig::default()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = BackendState::new(config, server.path()).unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let client = tempfile::tempdir().unwrap();
    ClientConfig::new(format!("http://{address}"))
        .unwrap()
        .save(&client.path().join("client.toml"))
        .unwrap();
    let directory = client.path().display().to_string();

    let identity = tokio::task::spawn_blocking({
        let directory = directory.clone();
        move || login_local(directory, "guillaume".into(), "safe-password".into())
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(identity.user_id, user_id);
    assert_eq!(identity.username, "guillaume");
    assert!(client.path().join("session.json").exists());
    assert!(client.path().join("monolith_cache_data.json").exists());
    assert_eq!(
        session_identity(directory.clone())
            .unwrap()
            .unwrap()
            .user_id,
        user_id
    );
    assert_eq!(
        load_library(directory, "Ray*".into()).unwrap()[0].games[0].title,
        "Rayman 2"
    );
}

#[tokio::test]
async fn flutter_download_requires_session_before_network_or_disk_write() {
    let client = tempfile::tempdir().unwrap();
    let error = tokio::task::spawn_blocking({
        let root = client.path().display().to_string();
        move || monolith::flutter_api::download_library_rom(root, 101).unwrap_err()
    })
    .await
    .unwrap();
    assert!(error.to_string().contains("connexion"));
    assert!(!client.path().join("roms").exists());
}

#[tokio::test]
async fn flutter_download_verifies_rom_and_registers_local_inventory() {
    let server = tempfile::tempdir().unwrap();
    let backend_db = server.path().join("backend.db");
    let database = Database::open(&backend_db).unwrap();
    database
        .create_local_user("guillaume", "safe-password", Role::Standard)
        .unwrap();
    database.upsert_game(&sample_game()).unwrap();
    let source = server.path().join("rayman.gdi");
    std::fs::write(&source, b"rom fixture").unwrap();
    database
        .record_downloaded_rom(101, &source, 11, "source hash")
        .unwrap();
    let config = BackendConfig {
        database_path: backend_db.display().to_string(),
        ..BackendConfig::default()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = BackendState::new(config, server.path()).unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let client = tempfile::tempdir().unwrap();
    ClientConfig::new(format!("http://{address}"))
        .unwrap()
        .save(&client.path().join("client.toml"))
        .unwrap();
    let root = client.path().display().to_string();
    tokio::task::spawn_blocking({
        let root = root.clone();
        move || login_local(root, "guillaume".into(), "safe-password".into()).unwrap()
    })
    .await
    .unwrap();
    let mut updated = sample_game();
    updated.description = "Description actualisée du backend".into();
    database.upsert_game(&updated).unwrap();
    let saved = tokio::task::spawn_blocking(move || {
        monolith::flutter_api::download_library_rom(root, 101).unwrap()
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read(&saved).unwrap(), b"rom fixture");
    let local = Database::open(&client.path().join("monolith.db")).unwrap();
    assert_eq!(
        local.base_game(101).unwrap().unwrap().description,
        updated.description
    );
    assert_eq!(
        local
            .available_rom_location_for_game(101)
            .unwrap()
            .unwrap()
            .path,
        saved
    );
}

#[test]
fn flutter_emulator_settings_validate_before_writing_and_preserve_on_failure() {
    use monolith::flutter_api::{emulator_config_text, save_emulator_config};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().display().to_string();
    assert!(emulator_config_text(root.clone())
        .unwrap()
        .contains("system_id = 11"));
    assert!(save_emulator_config(
        root.clone(),
        "[[profile]]\nsystem_id = 11\nexecutable = 'retroarch'\narguments = []".into()
    )
    .is_err());
    assert!(!directory.path().join("emulators.toml").exists());
    save_emulator_config(
        root.clone(),
        "[[profile]]\nsystem_id = 11\nexecutable = 'retroarch'\narguments = ['{rom}']".into(),
    )
    .unwrap();
    assert!(emulator_config_text(root)
        .unwrap()
        .contains("system_id = 11"));
}

#[test]
fn flutter_does_not_restore_session_from_another_backend() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().display().to_string();
    let db = Database::open(&directory.path().join("monolith.db")).unwrap();
    db.upsert_game(&sample_game()).unwrap();
    db.save_override(&UserOverride {
        user_id: 42,
        game_id: 101,
        description: Some("Privé".into()),
        cover_art: None,
    })
    .unwrap();
    ClientConfig::new("http://192.168.1.39:8788")
        .unwrap()
        .save(&directory.path().join("client.toml"))
        .unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    store
        .save(&ClientSession {
            backend_url: "http://127.0.0.1:8787".into(),
            access_token: "ancien jeton".into(),
            user_id: 42,
            username: "guillaume".into(),
            role: Role::Standard,
            expires_at: Utc::now().timestamp() + 3600,
        })
        .unwrap();

    assert!(session_identity(root.clone()).unwrap().is_none());
    assert!(!store.path().exists());
    assert_eq!(
        load_library(root, "Ray*".into()).unwrap()[0].games[0].description,
        "Description générale"
    );
}

#[test]
fn flutter_backend_setting_normalizes_and_invalidates_only_on_change() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().display().to_string();
    let store = SessionStore::new(directory.path().join("session.json"));
    store
        .save(&ClientSession {
            backend_url: "http://127.0.0.1:8787".into(),
            access_token: "secret".into(),
            user_id: 42,
            username: "guillaume".into(),
            role: Role::Standard,
            expires_at: Utc::now().timestamp() + 3600,
        })
        .unwrap();
    assert_eq!(backend_url(root.clone()).unwrap(), "http://127.0.0.1:8787");
    assert!(save_backend_url(root.clone(), "invalid".into()).is_err());
    assert!(store.path().exists());
    save_backend_url(root.clone(), "http://127.0.0.1:8787/".into()).unwrap();
    assert!(store.path().exists());
    save_backend_url(root.clone(), "http://192.168.1.39:8788/".into()).unwrap();
    assert!(!store.path().exists());
    assert_eq!(backend_url(root).unwrap(), "http://192.168.1.39:8788");
}
