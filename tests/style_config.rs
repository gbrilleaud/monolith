use monolith::style_config::{StylePreset, StyleStore};

fn custom_json(background_image: Option<&str>) -> String {
    let mut custom: StylePreset = StyleStore::load(tempfile::tempdir().unwrap().path())
        .unwrap()
        .resolved;
    custom.id = "custom".into();
    custom.name = "Mon style".into();
    custom.background_image = background_image.map(str::to_owned);
    serde_json::to_string(&custom).unwrap()
}

#[test]
fn first_load_uses_the_seven_bundled_presets_and_dark_by_default() {
    let root = tempfile::tempdir().unwrap();

    let snapshot = StyleStore::load(root.path()).unwrap();

    let ids = snapshot
        .catalog
        .presets
        .iter()
        .map(|preset| preset.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "light",
            "dark",
            "windows_xp",
            "pink",
            "back_to_future",
            "zelda",
            "futuristic",
        ]
    );
    assert_eq!(snapshot.selected, "dark");
    assert_eq!(snapshot.resolved.id, "dark");
    assert!(snapshot.cartridge_animation);
}

#[test]
fn corrupt_cache_is_ignored_without_losing_local_preferences() {
    let root = tempfile::tempdir().unwrap();
    StyleStore::set_animation(root.path(), false).unwrap();
    std::fs::write(root.path().join("style-catalog.json"), b"not json").unwrap();

    let snapshot = StyleStore::load(root.path()).unwrap();

    assert_eq!(snapshot.catalog.presets.len(), 7);
    assert_eq!(snapshot.selected, "dark");
    assert!(!snapshot.cartridge_animation);
}

#[test]
fn custom_style_is_selected_persisted_and_appended_once() {
    let root = tempfile::tempdir().unwrap();

    let saved =
        StyleStore::save_custom_json(root.path(), &custom_json(Some("stars.webp"))).unwrap();
    let loaded = StyleStore::load(root.path()).unwrap();

    assert_eq!(saved.selected, "custom");
    assert_eq!(loaded.resolved.name, "Mon style");
    assert_eq!(
        loaded.resolved.background_image.as_deref(),
        Some("stars.webp")
    );
    assert_eq!(loaded.catalog.presets.len(), 8);
    assert_eq!(
        loaded
            .catalog
            .presets
            .iter()
            .filter(|preset| preset.id == "custom")
            .count(),
        1
    );
}

#[test]
fn custom_style_rejects_traversal_external_urls_and_unsafe_extensions() {
    let root = tempfile::tempdir().unwrap();
    for image in [
        "../secret.png",
        "https://example.test/a.png",
        "script.svg",
        "sub/image.jpg",
        r"C:\\private.png",
        r"..\\private.png",
        "bad.png:stream",
    ] {
        assert!(
            StyleStore::save_custom_json(root.path(), &custom_json(Some(image))).is_err(),
            "{image}"
        );
    }
    assert!(!root.path().join("style.json").exists());
}

#[test]
fn selection_and_animation_survive_reload() {
    let root = tempfile::tempdir().unwrap();
    StyleStore::select(root.path(), "zelda").unwrap();
    StyleStore::set_animation(root.path(), false).unwrap();

    let loaded = StyleStore::load(root.path()).unwrap();
    assert_eq!(loaded.selected, "zelda");
    assert_eq!(loaded.resolved.id, "zelda");
    assert!(!loaded.cartridge_animation);
    assert!(StyleStore::select(root.path(), "missing").is_err());
}

#[test]
fn damaged_local_style_can_be_explicitly_reset() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("style.json"), b"{broken").unwrap();
    assert!(StyleStore::load(root.path()).is_err());
    let recovered = StyleStore::reset(root.path()).unwrap();
    assert_eq!(recovered.selected, "dark");
    assert!(recovered.cartridge_animation);
    assert_eq!(StyleStore::load(root.path()).unwrap().selected, "dark");
}

#[test]
fn server_may_add_a_preset_after_the_seven_bundled_styles() {
    let root = tempfile::tempdir().unwrap();
    let mut catalog = monolith::style_config::bundled_style_catalog();
    let mut extra = catalog.presets[0].clone();
    extra.id = "arcade".into();
    extra.name = "Arcade du serveur".into();
    catalog.presets.push(extra);
    std::fs::write(
        root.path().join("style-catalog.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();

    let snapshot = StyleStore::load(root.path()).unwrap();
    assert_eq!(snapshot.catalog.presets.len(), 8);
    let selected = StyleStore::select(root.path(), "arcade").unwrap();
    assert_eq!(selected.resolved.name, "Arcade du serveur");
}

async fn json_server(body: String) -> String {
    let app = axum::Router::new().route(
        "/api/v1/styles",
        axum::routing::get(move || {
            let body = body.clone();
            async move {
                (
                    [(&axum::http::header::CONTENT_TYPE, "application/json")],
                    body,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}")
}

#[tokio::test]
async fn refresh_replaces_only_a_valid_cache_and_preserves_customization() {
    let root = tempfile::tempdir().unwrap();
    StyleStore::save_custom_json(root.path(), &custom_json(None)).unwrap();
    let mut remote = monolith::style_config::bundled_style_catalog();
    remote.presets[0].name = "Remote light".into();
    let valid_url = json_server(serde_json::to_string(&remote).unwrap()).await;

    let root_path = root.path().to_path_buf();
    let refreshed =
        tokio::task::spawn_blocking(move || StyleStore::refresh(&root_path, &valid_url))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(refreshed.catalog.presets[0].name, "Remote light");
    assert_eq!(refreshed.selected, "custom");

    remote.presets[0].density = 99.0;
    let invalid_url = json_server(serde_json::to_string(&remote).unwrap()).await;
    let root_path = root.path().to_path_buf();
    assert!(
        tokio::task::spawn_blocking(move || StyleStore::refresh(&root_path, &invalid_url))
            .await
            .unwrap()
            .is_err()
    );

    let preserved = StyleStore::load(root.path()).unwrap();
    assert_eq!(preserved.catalog.presets[0].name, "Remote light");
    assert_eq!(preserved.selected, "custom");
    assert_eq!(preserved.resolved.name, "Mon style");
}
