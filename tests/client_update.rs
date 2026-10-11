use monolith::client_update::{parse_release, unpack_verified_client};
use sha2::{Digest, Sha256};
use std::io::Write;

#[test]
fn only_newer_stable_releases_with_exact_windows_assets_are_offered() {
    let release = serde_json::json!({
        "tag_name": "v0.2.0", "draft": false, "prerelease": false,
        "assets": [
            {"name": "monolith-windows-x86_64.zip", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-windows-x86_64.zip"},
            {"name": "monolith-windows-x86_64.zip.sha256", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-windows-x86_64.zip.sha256"}
        ]
    });
    let update = parse_release(&release, "0.1.0").unwrap().unwrap();
    assert_eq!(update.version, "0.2.0");
    assert!(parse_release(&release, "0.2.0").unwrap().is_none());
    assert!(parse_release(&release, "0.3.0").unwrap().is_none());
    let mut draft = release.clone();
    draft["prerelease"] = true.into();
    assert!(parse_release(&draft, "0.1.0").unwrap().is_none());
    let mut missing_asset = release.clone();
    missing_asset["assets"].as_array_mut().unwrap().pop();
    assert!(parse_release(&missing_asset, "0.1.0").unwrap().is_none());
}

#[test]
fn verified_archive_extracts_only_the_client_binary_and_rejects_tampering() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
    zip.start_file::<_, ()>(
        "monolith-windows-x86_64/monolith.exe",
        zip::write::FileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"new-client").unwrap();
    zip.finish().unwrap();
    let bytes = std::fs::read(file.path()).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let checksum = format!("{digest}  monolith-windows-x86_64.zip");
    let output = tempfile::tempdir().unwrap();
    let destination = output.path().join("staged.exe");
    unpack_verified_client(&bytes, &checksum, &destination).unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"new-client");
    assert!(unpack_verified_client(b"tampered", &checksum, &destination).is_err());
    assert!(unpack_verified_client(&bytes, &format!("{digest}  evil.zip"), &destination).is_err());
}
