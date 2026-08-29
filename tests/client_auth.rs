use monolith::{
    auth::Role,
    backend::{router, BackendState},
    backend_config::BackendConfig,
    client_auth::{AuthState, ClientAuth},
    db::Database,
    session_store::{ClientSession, SessionStore},
};
use std::time::Duration;
use tempfile::tempdir;

fn session() -> ClientSession {
    ClientSession {
        backend_url: "http://127.0.0.1:8787".into(),
        access_token: "restored-token".into(),
        user_id: 7,
        username: "guillaume".into(),
        role: Role::Standard,
        expires_at: 2_000,
    }
}

#[test]
fn unexpired_session_is_restored_without_network_access() {
    let directory = tempdir().unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    store.save(&session()).unwrap();

    let auth = ClientAuth::new(
        "http://127.0.0.1:8787",
        store,
        directory.path().join("cache.json"),
        1_000,
    )
    .unwrap();

    assert_eq!(auth.state(), &AuthState::Authenticated(session()));
}

#[test]
fn logout_clears_memory_and_persisted_session() {
    let directory = tempdir().unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    store.save(&session()).unwrap();
    let mut auth = ClientAuth::new(
        "http://127.0.0.1:8787",
        store.clone(),
        directory.path().join("cache.json"),
        1_000,
    )
    .unwrap();

    auth.logout().unwrap();

    assert_eq!(auth.state(), &AuthState::SignedOut);
    assert!(!store.path().exists());
}

#[tokio::test]
async fn policy_probe_runs_in_background() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let config = BackendConfig {
        database_path: database_path.display().to_string(),
        ..BackendConfig::default()
    };
    let state = BackendState::new(config, directory.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let mut auth = ClientAuth::new(
        format!("http://{address}"),
        SessionStore::new(directory.path().join("session.json")),
        directory.path().join("cache.json"),
        chrono::Utc::now().timestamp(),
    )
    .unwrap();

    auth.probe_policy().unwrap();
    for _ in 0..100 {
        auth.poll();
        if auth.auth_mode().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(auth.auth_mode(), Some(monolith::auth::AuthMode::Local));
}

#[tokio::test]
async fn local_login_runs_in_background_and_persists_no_password() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("alice", "safe-password", Role::Standard)
        .unwrap();
    let config = BackendConfig {
        database_path: database_path.display().to_string(),
        ..BackendConfig::default()
    };
    let state = BackendState::new(config, directory.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });

    let store = SessionStore::new(directory.path().join("session.json"));
    let mut auth = ClientAuth::new(
        format!("http://{address}"),
        store.clone(),
        directory.path().join("cache.json"),
        chrono::Utc::now().timestamp(),
    )
    .unwrap();

    auth.begin_local_login("alice", "safe-password").unwrap();
    assert_eq!(auth.state(), &AuthState::Authenticating);
    for _ in 0..100 {
        auth.poll();
        if matches!(auth.state(), AuthState::Authenticated(_)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let AuthState::Authenticated(session) = auth.state() else {
        panic!("authentification non terminée: {:?}", auth.state());
    };
    assert_eq!(session.username, "alice");
    let persisted = std::fs::read_to_string(store.path()).unwrap();
    assert!(!persisted.contains("safe-password"));
    assert!(directory.path().join("cache.json").exists());
}

#[tokio::test]
async fn bearer_login_resolves_identity_and_persists_session() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("backend.db");
    let database = Database::open(&database_path).unwrap();
    database
        .create_local_user("bearer-user", "safe-password", Role::ReadOnly)
        .unwrap();
    let config = BackendConfig {
        database_path: database_path.display().to_string(),
        ..BackendConfig::default()
    };
    let state = BackendState::new(config, directory.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let backend_url = format!("http://{address}");
    let token = monolith::backend_client::BackendClient::new(&backend_url)
        .unwrap()
        .login_local("bearer-user", "safe-password")
        .await
        .unwrap()
        .access_token;

    let store = SessionStore::new(directory.path().join("session.json"));
    let mut auth = ClientAuth::new(
        &backend_url,
        store,
        directory.path().join("cache.json"),
        chrono::Utc::now().timestamp(),
    )
    .unwrap();
    auth.begin_bearer_login(&token).unwrap();
    for _ in 0..100 {
        auth.poll();
        if matches!(auth.state(), AuthState::Authenticated(_)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let AuthState::Authenticated(session) = auth.state() else {
        panic!("authentification Bearer non terminée: {:?}", auth.state());
    };
    assert_eq!(session.username, "bearer-user");
    assert_eq!(session.role, Role::ReadOnly);
}
