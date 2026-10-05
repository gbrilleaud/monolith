use monolith::client_config::ClientConfig;
use tempfile::tempdir;

#[test]
fn config_persists_a_normalized_backend_url_without_trailing_slash() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("client.toml");

    let config = ClientConfig::new(" http://192.168.1.39:8788/ ").unwrap();
    config.save(&path).unwrap();

    assert_eq!(
        ClientConfig::load_or_default(&path).unwrap().backend_url,
        "http://192.168.1.39:8788"
    );
}

#[test]
fn config_persists_an_absolute_install_root() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("client.toml");
    let install_root = directory.path().join("MONOLITH");
    let config = ClientConfig::new("http://127.0.0.1:8787")
        .unwrap()
        .with_install_root(&install_root)
        .unwrap();

    config.save(&path).unwrap();

    assert_eq!(
        ClientConfig::load_or_default(&path).unwrap().install_root,
        Some(install_root.display().to_string())
    );
}

#[test]
fn config_rejects_a_backend_url_without_http_scheme() {
    assert!(ClientConfig::new("192.168.1.39:8788").is_err());
}
