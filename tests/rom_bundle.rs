use monolith::rom_bundle::{build_rom_bundle, BundleKind};

#[test]
fn one_rom_file_is_sent_directly_without_creating_an_archive() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("Rayman 2.iso");
    std::fs::write(&source, b"single-rom").unwrap();

    let bundle = build_rom_bundle(std::slice::from_ref(&source)).unwrap();

    assert_eq!(bundle.kind, BundleKind::SingleFile);
    assert_eq!(bundle.path, source);
    assert_eq!(bundle.entry_count, 1);
    assert!(!bundle.cleanup_after_upload);
}

#[test]
fn multiple_rom_files_are_staged_as_a_zip_with_basename_entries_only() {
    let directory = tempfile::tempdir().unwrap();
    let cue = directory.path().join("Panzer Dragoon.cue");
    let bin = directory.path().join("Panzer Dragoon (Track 01).bin");
    std::fs::write(&cue, b"FILE \"Panzer Dragoon (Track 01).bin\" BINARY").unwrap();
    std::fs::write(&bin, b"disc-data").unwrap();

    let bundle = build_rom_bundle(&[cue, bin]).unwrap();

    assert_eq!(bundle.kind, BundleKind::ZipArchive);
    assert_eq!(bundle.entry_count, 2);
    assert!(bundle.cleanup_after_upload);
    assert_eq!(bundle.display_name, "Panzer Dragoon.bundle.zip");
    assert!(bundle.path.is_file());
    assert_eq!(std::fs::read(&bundle.path).unwrap()[..2], *b"PK");
    bundle.cleanup().unwrap();
    assert!(!bundle.path.exists());
}
