use monolith::client_update::{
    parse_flutter_release, parse_release, unpack_flutter_bundle, unpack_verified_client,
};
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
fn flutter_update_never_accepts_the_egui_release_package() {
    let release = serde_json::json!({
        "tag_name": "v0.2.0", "draft": false, "prerelease": false,
        "assets": [
            {"name": "monolith-windows-x86_64.zip", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-windows-x86_64.zip"},
            {"name": "monolith-windows-x86_64.zip.sha256", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-windows-x86_64.zip.sha256"}
        ]
    });
    assert!(parse_flutter_release(&release, "0.1.0").unwrap().is_none());
    let mut flutter = release;
    flutter["assets"] = serde_json::json!([
        {"name": "monolith-flutter-windows-x86_64.zip", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-flutter-windows-x86_64.zip"},
        {"name": "monolith-flutter-windows-x86_64.zip.sha256", "browser_download_url": "https://github.com/gbrilleaud/monolith/releases/download/v0.2.0/monolith-flutter-windows-x86_64.zip.sha256"}
    ]);
    assert_eq!(
        parse_flutter_release(&flutter, "0.1.0")
            .unwrap()
            .unwrap()
            .version,
        "0.2.0"
    );
    assert!(parse_flutter_release(&flutter, "0.2.0").unwrap().is_none());
}

#[test]
fn flutter_bundle_requires_runtime_and_rejects_traversal_or_tampering() {
    let make = |bad: Option<&str>| {
        let bytes = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(bytes);
        let root = "monolith-flutter-windows-x86_64/";
        for path in [
            "monolith.exe",
            "monolith.dll",
            "flutter_windows.dll",
            "monolith-flutter-updater.exe",
            "data/app.so",
            "data/icudtl.dat",
            "data/flutter_assets/AssetManifest.json",
        ] {
            zip.start_file::<_, ()>(format!("{root}{path}"), zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(b"runtime").unwrap();
        }
        if let Some(path) = bad {
            zip.start_file::<_, ()>(format!("{root}{path}"), zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(b"evil").unwrap();
        }
        zip.finish().unwrap().into_inner()
    };
    let bytes = make(None);
    let checksum = format!(
        "{:x}  monolith-flutter-windows-x86_64.zip",
        Sha256::digest(&bytes)
    );
    let output = tempfile::tempdir().unwrap();
    unpack_flutter_bundle(&bytes, &checksum, output.path()).unwrap();
    assert_eq!(
        std::fs::read(output.path().join("data/app.so")).unwrap(),
        b"runtime"
    );
    assert!(unpack_flutter_bundle(b"corrupt", &checksum, output.path()).is_err());
    let incomplete = tempfile::tempdir().unwrap();
    assert!(unpack_flutter_bundle(
        &bytes,
        &checksum.replace("flutter-windows", "windows"),
        incomplete.path()
    )
    .is_err());
    let malicious = make(Some("../outside.exe"));
    let malicious_checksum = format!(
        "{:x}  monolith-flutter-windows-x86_64.zip",
        Sha256::digest(&malicious)
    );
    assert!(unpack_flutter_bundle(&malicious, &malicious_checksum, incomplete.path()).is_err());
    assert!(!incomplete.path().join("outside.exe").exists());
    for protected in [
        "data/private.sqlite",
        "client.toml",
        "data\\\\roms\\\\game.bin",
    ] {
        let malicious = make(Some(protected));
        let checksum = format!(
            "{:x}  monolith-flutter-windows-x86_64.zip",
            Sha256::digest(&malicious)
        );
        assert!(unpack_flutter_bundle(&malicious, &checksum, incomplete.path()).is_err());
    }
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
