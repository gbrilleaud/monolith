use monolith::backend_config::LibraryConfig;

#[test]
fn library_upload_root_must_be_absolute() {
    assert!(LibraryConfig {
        roots: vec![],
        upload_root: "/srv/monolith/roms/00_inbox".into(),
    }
    .validate()
    .is_ok());

    assert!(LibraryConfig {
        roots: vec![],
        upload_root: "roms/00_inbox".into(),
    }
    .validate()
    .is_err());
}
