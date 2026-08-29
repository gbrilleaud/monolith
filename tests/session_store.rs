use monolith::{
    auth::Role,
    session_store::{ClientSession, SessionStore},
};
use tempfile::tempdir;

fn session(expires_at: i64) -> ClientSession {
    ClientSession {
        backend_url: "http://127.0.0.1:8787".into(),
        access_token: "opaque-secret".into(),
        user_id: 42,
        username: "guillaume".into(),
        role: Role::Standard,
        expires_at,
    }
}

#[test]
fn saved_session_round_trips_before_expiration() {
    let directory = tempdir().unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    let expected = session(2_000);

    store.save(&expected).unwrap();

    assert_eq!(store.load_valid_at(1_999).unwrap(), Some(expected));
}

#[test]
fn expired_session_is_rejected_and_deleted() {
    let directory = tempdir().unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    store.save(&session(2_000)).unwrap();

    assert_eq!(store.load_valid_at(2_000).unwrap(), None);
    assert!(!store.path().exists());
}

#[test]
fn invalid_session_is_rejected_and_deleted() {
    let directory = tempdir().unwrap();
    let store = SessionStore::new(directory.path().join("session.json"));
    std::fs::write(store.path(), b"not-json").unwrap();

    assert_eq!(store.load_valid_at(1_000).unwrap(), None);
    assert!(!store.path().exists());
}
