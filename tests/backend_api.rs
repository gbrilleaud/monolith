use monolith::{
    auth::Role,
    backend::{router, BackendState},
    backend_client::BackendClient,
    backend_config::{BackendConfig, LibraryConfig},
    db::Database,
    models::{GameMetadata, ScanObservation, ScanRoot, UserOverride},
};

fn game() -> GameMetadata {
    GameMetadata {
        game_id: 1,
        system_id: 10,
        system_name: "Dreamcast".into(),
        title: "Test Drive".into(),
        description: "Harvest".into(),
        cover_art: None,
        language: "fr".into(),
        launch_availability: Default::default(),
    }
}

async fn test_server(role: Role) -> (BackendClient, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("alice", "safe-password", role)
        .unwrap();
    database.upsert_game(&game()).unwrap();

    let config = BackendConfig {
        database_path: database_path.display().to_string(),
        ..BackendConfig::default()
    };
    let state = BackendState::new(config, directory.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    (
        BackendClient::new(format!("http://{address}")).unwrap(),
        directory,
    )
}

#[tokio::test]
async fn login_exposes_expiration_and_bearer_identity() {
    let (client, _directory) = test_server(Role::Standard).await;
    let login = client.login_local("alice", "safe-password").await.unwrap();

    assert!(login.expires_at > chrono::Utc::now().timestamp());
    let identity = client.identity(&login.access_token).await.unwrap();
    assert_eq!(identity.user_id, login.user_id);
    assert_eq!(identity.username, "alice");
    assert_eq!(identity.role, Role::Standard);
    assert_eq!(identity.expires_at, Some(login.expires_at));
}

#[tokio::test]
async fn connector_logs_in_downloads_catalog_and_uploads_override() {
    let (client, directory) = test_server(Role::Standard).await;
    let login = client.login_local("alice", "safe-password").await.unwrap();
    let cache_path = directory.path().join("client-cache.json");
    assert_eq!(
        client
            .refresh_offline_cache(&login.access_token, &cache_path)
            .await
            .unwrap(),
        1
    );
    assert!(cache_path.exists());
    let initial = client.fetch_catalog(&login.access_token).await.unwrap();
    assert_eq!(initial.games[0].description, "Harvest");

    client
        .save_override(
            &login.access_token,
            &UserOverride {
                user_id: login.user_id,
                game_id: 1,
                description: Some("Personnel".into()),
                cover_art: None,
            },
        )
        .await
        .unwrap();
    let updated = client.fetch_catalog(&login.access_token).await.unwrap();
    assert_eq!(updated.games[0].description, "Personnel");
}

#[tokio::test]
async fn read_only_account_can_download_but_not_upload() {
    let (client, _directory) = test_server(Role::ReadOnly).await;
    let login = client.login_local("alice", "safe-password").await.unwrap();
    assert_eq!(
        client
            .fetch_catalog(&login.access_token)
            .await
            .unwrap()
            .games
            .len(),
        1
    );
    let error = client
        .save_override(
            &login.access_token,
            &UserOverride {
                user_id: login.user_id,
                game_id: 1,
                description: Some("Interdit".into()),
                cover_art: None,
            },
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("surcharge refusée"));
}

#[tokio::test]
async fn read_only_account_cannot_upload_a_rom() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let upload_root = directory.path().join("roms/00_inbox");
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("alice", "safe-password", Role::ReadOnly)
        .unwrap();
    let state = BackendState::new(
        BackendConfig {
            database_path: database_path.display().to_string(),
            library: LibraryConfig {
                roots: vec![ScanRoot {
                    system_id: 10,
                    path: directory.path().join("roms/10").display().to_string(),
                    extensions: vec!["gdi".into()],
                }],
                upload_root: upload_root.display().to_string(),
            },
            ..BackendConfig::default()
        },
        directory.path(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let base_url = format!("http://{address}");
    let client = BackendClient::new(&base_url).unwrap();
    let login = client.login_local("alice", "safe-password").await.unwrap();

    let response = reqwest::Client::new()
        .post(format!("{base_url}/api/v1/library/uploads?system_id=10"))
        .bearer_auth(login.access_token)
        .header("x-monolith-file-name", "forbidden.zip")
        .body("private-rom-content")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(!upload_root.exists());
}

#[tokio::test]
async fn standard_user_cannot_publish_an_inbox_rom() {
    let (client, directory) = test_server(Role::Standard).await;
    let login = client.login_local("alice", "safe-password").await.unwrap();
    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/v1/library/uploads/00000000-0000-0000-0000-000000000000/publish?system_id=10",
            client.base_url()
        ))
        .bearer_auth(login.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(!directory.path().join("roms/00_inbox").exists());
}

#[tokio::test]
async fn standard_user_downloads_only_an_available_linked_rom_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let source_path = directory.path().join("Global Gladiators (Europe).zip");
    let source_bytes = b"private-rom-content";
    std::fs::write(&source_path, source_bytes).unwrap();
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("alice", "safe-password", Role::Standard)
        .unwrap();
    database.upsert_game(&game()).unwrap();
    let root = ScanRoot {
        system_id: 10,
        path: directory.path().display().to_string(),
        extensions: vec!["zip".into()],
    };
    database
        .sync_rom_inventory(
            &root,
            &[ScanObservation {
                system_id: 10,
                path: source_path.display().to_string(),
                extension: "zip".into(),
                size_bytes: source_bytes.len() as u64,
                modified_at: None,
                sha256: None,
            }],
        )
        .unwrap();
    database
        .link_rom_location_to_game(&source_path.display().to_string(), 1)
        .unwrap();

    let state = BackendState::new(
        BackendConfig {
            database_path: database_path.display().to_string(),
            ..BackendConfig::default()
        },
        directory.path(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let client = BackendClient::new(format!("http://{address}")).unwrap();
    let login = client.login_local("alice", "safe-password").await.unwrap();
    let destination = directory.path().join("client-data/roms/10");

    let download = client
        .download_game_rom(&login.access_token, 1, &destination)
        .await
        .unwrap();

    let downloaded_path = destination.join("Global Gladiators (Europe).zip");
    assert_eq!(std::fs::read(&downloaded_path).unwrap(), source_bytes);
    assert_eq!(download.path, downloaded_path);
    assert_eq!(download.size_bytes, source_bytes.len() as u64);
    assert_eq!(download.file_name, "Global Gladiators (Europe).zip");
    assert!(!destination
        .join("Global Gladiators (Europe).zip.partial")
        .exists());
}

#[tokio::test]
async fn standard_user_uploads_to_the_configured_inbox_without_a_partial_file() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let upload_root = directory.path().join("roms/00_inbox");
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("alice", "safe-password", Role::Standard)
        .unwrap();
    database
        .create_local_user("admin", "admin-password", Role::Admin)
        .unwrap();
    let state = BackendState::new(
        BackendConfig {
            database_path: database_path.display().to_string(),
            library: LibraryConfig {
                roots: vec![ScanRoot {
                    system_id: 10,
                    path: directory.path().join("roms/10").display().to_string(),
                    extensions: vec!["gdi".into()],
                }],
                upload_root: upload_root.display().to_string(),
            },
            ..BackendConfig::default()
        },
        directory.path(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let base_url = format!("http://{address}");
    let client = BackendClient::new(&base_url).unwrap();
    let login = client.login_local("alice", "safe-password").await.unwrap();

    let source = directory.path().join("Ikaruga (Europe).gdi");
    std::fs::write(&source, b"disc-image").unwrap();
    let upload = client
        .upload_rom(&login.access_token, 10, &source)
        .await
        .unwrap();

    assert_eq!(upload.file_name, "Ikaruga (Europe).gdi");
    assert_eq!(upload.size_bytes, 10);
    assert_eq!(upload.sha256.len(), 64);
    let duplicate = client
        .upload_rom(&login.access_token, 10, &source)
        .await
        .unwrap_err();
    assert!(duplicate.to_string().contains("import ROM refusé"));
    let upload_directories = std::fs::read_dir(&upload_root)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(upload_directories.len(), 1);
    let upload_directory = upload_directories[0].path();
    assert!(upload_directory.is_dir());
    let files = std::fs::read_dir(&upload_directory)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(files.len(), 1);
    let uploaded = files[0].path();
    assert_eq!(std::fs::read(&uploaded).unwrap(), b"disc-image");
    assert!(!uploaded
        .extension()
        .is_some_and(|extension| extension == "partial"));

    let admin = client.login_local("admin", "admin-password").await.unwrap();
    let published = reqwest::Client::new()
        .post(format!(
            "{base_url}/api/v1/library/uploads/{}/publish?system_id=10",
            upload.upload_id
        ))
        .bearer_auth(admin.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(published.status(), reqwest::StatusCode::CREATED);
    let destination = directory.path().join("roms/10/Ikaruga (Europe).gdi");
    assert_eq!(std::fs::read(&destination).unwrap(), b"disc-image");
    assert!(!upload_directory.exists());
    let locations = Database::open(&database_path)
        .unwrap()
        .unlinked_rom_locations()
        .unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].path, destination.display().to_string());
    assert_eq!(locations[0].sha256, Some(upload.sha256));
}
