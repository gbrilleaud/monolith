use monolith::backend_config::BackendConfig;

fn load_config(raw: &str) -> anyhow::Result<BackendConfig> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("backend.toml");
    std::fs::write(&path, raw)?;
    BackendConfig::load(&path)
}

#[test]
fn configuration_loads_multiple_library_roots() {
    let config = load_config(
        r#"
            database_path = "data/backend.db"

            [[library.roots]]
            system_id = 1
            path = "/mnt/roms/megadrive"
            extensions = ["zip", ".MD"]

            [[library.roots]]
            system_id = 2
            path = "/mnt/roms/dreamcast"
            extensions = ["chd", "gdi"]
        "#,
    )
    .unwrap();

    assert_eq!(config.library.roots.len(), 2);
    assert_eq!(config.library.roots[0].system_id, 1);
    assert_eq!(config.library.roots[0].extensions, vec!["zip", "md"]);
    assert_eq!(config.library.roots[1].path, "/mnt/roms/dreamcast");
}

#[test]
fn configuration_rejects_a_relative_library_root() {
    let error = load_config(
        r#"
            [[library.roots]]
            system_id = 1
            path = "roms/megadrive"
            extensions = ["zip"]
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("chemin de scan doit être absolu"));
}

#[test]
fn configuration_rejects_a_library_root_without_extensions() {
    let error = load_config(
        r#"
            [[library.roots]]
            system_id = 1
            path = "/mnt/roms/megadrive"
            extensions = []
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("au moins une extension est requise"));
}

#[test]
fn configuration_rejects_a_library_root_with_an_invalid_system_id() {
    let error = load_config(
        r#"
            [[library.roots]]
            system_id = 0
            path = "/mnt/roms/megadrive"
            extensions = ["zip"]
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("system_id doit être positif"));
}
