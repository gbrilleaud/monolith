use monolith::{
    auth::Role,
    backend::{router, BackendState},
    backend_client::BackendClient,
    backend_config::BackendConfig,
    db::Database,
    models::{GameMetadata, UserOverride},
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
