use monolith::cover::cover_uri;

#[test]
fn remote_cover_url_is_kept_for_the_http_loader() {
    assert_eq!(
        cover_uri("https://media.example/cover.webp").unwrap(),
        Some("https://media.example/cover.webp".into())
    );
}

#[test]
fn local_cover_path_becomes_an_absolute_file_uri() {
    let directory = tempfile::tempdir().unwrap();
    let cover = directory.path().join("jaquette française.png");
    std::fs::write(&cover, b"image").unwrap();

    assert_eq!(
        cover_uri(cover.to_str().unwrap()).unwrap(),
        Some(format!("file://{}", cover.display()))
    );
}

#[test]
fn windows_cover_path_becomes_a_valid_file_uri() {
    assert_eq!(
        cover_uri(r"C:\MONOLITH\data\covers\11002.png").unwrap(),
        Some("file:///C:/MONOLITH/data/covers/11002.png".into())
    );
}

#[test]
fn empty_cover_reference_has_no_image_source() {
    assert_eq!(cover_uri("   ").unwrap(), None);
}
