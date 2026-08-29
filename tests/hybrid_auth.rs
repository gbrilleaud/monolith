use monolith::{
    auth::{AuthMode, AuthPrincipal, AuthSource, Role},
    backend_config::{AuthConfig, BackendConfig, OidcConfig},
    db::Database,
};

#[test]
fn configuration_round_trip_preserves_hybrid_auth() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backend.toml");
    let config = BackendConfig {
        auth: AuthConfig {
            mode: AuthMode::Hybrid,
            oidc: OidcConfig {
                issuer: Some("https://sso.example.test/realms/monolith".into()),
                audience: Some("monolith".into()),
                jwks_url: Some("https://sso.example.test/jwks".into()),
                auto_provision: false,
            },
            ..AuthConfig::default()
        },
        ..BackendConfig::default()
    };
    config.save(&path).unwrap();

    let loaded = BackendConfig::load(&path).unwrap();
    assert_eq!(loaded.auth.mode, AuthMode::Hybrid);
    assert_eq!(loaded.auth.oidc.audience.as_deref(), Some("monolith"));
    loaded.validate().unwrap();
}

#[test]
fn sso_mode_rejects_an_incomplete_oidc_configuration() {
    let config = BackendConfig {
        auth: AuthConfig {
            mode: AuthMode::Sso,
            ..AuthConfig::default()
        },
        ..BackendConfig::default()
    };
    let error = config.validate().unwrap_err().to_string();
    assert!(error.contains("issuer"));
}

#[test]
fn local_password_login_creates_a_verifiable_opaque_session() {
    let db = Database::open_in_memory().unwrap();
    let user_id = db
        .create_local_user("alice", "correct horse battery staple", Role::Standard)
        .unwrap();

    let principal = db
        .authenticate_local("alice", "correct horse battery staple", 3600)
        .unwrap()
        .unwrap();
    assert_eq!(principal.user_id, user_id);
    assert_eq!(principal.source, AuthSource::Local);
    assert_eq!(principal.role, Role::Standard);

    let resolved = db.resolve_local_session(&principal.token).unwrap().unwrap();
    assert_eq!(resolved.user_id, user_id);
    assert!(db
        .authenticate_local("alice", "incorrect", 3600)
        .unwrap()
        .is_none());
}

#[test]
fn disabled_local_user_cannot_authenticate() {
    let db = Database::open_in_memory().unwrap();
    let user_id = db
        .create_local_user("bob", "secret-123", Role::Admin)
        .unwrap();
    db.set_user_enabled(user_id, false).unwrap();
    assert!(db
        .authenticate_local("bob", "secret-123", 60)
        .unwrap()
        .is_none());
}

#[test]
fn hybrid_policy_accepts_local_and_sso_principals() {
    assert!(AuthMode::Hybrid.accepts(AuthSource::Local));
    assert!(AuthMode::Hybrid.accepts(AuthSource::Sso));
    assert!(AuthMode::Local.accepts(AuthSource::Local));
    assert!(!AuthMode::Local.accepts(AuthSource::Sso));

    let principal = AuthPrincipal {
        user_id: 42,
        username: "oidc:subject".into(),
        role: Role::ReadOnly,
        source: AuthSource::Sso,
        token: String::new(),
    };
    assert_eq!(principal.role, Role::ReadOnly);
}
