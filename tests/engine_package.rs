use monolith::engine_package::{install_engine_package, EnginePackageManifest, InstallLimits};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, ZipWriter};

fn write_zip(path: &Path, entries: &[(&str, &[u8], Option<u32>)]) -> String {
    let output = fs::File::create(path).unwrap();
    let mut archive = ZipWriter::new(output);
    for (name, contents, mode) in entries {
        let mut options = SimpleFileOptions::default();
        if let Some(mode) = mode {
            options = options.unix_permissions(*mode);
        }
        archive.start_file(*name, options).unwrap();
        archive.write_all(contents).unwrap();
    }
    archive.finish().unwrap();
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn manifest(sha256: String) -> EnginePackageManifest {
    EnginePackageManifest {
        component_id: "retroarch-genesis".into(),
        source_url: "https://downloads.example.test/retroarch-1.2.3.zip".into(),
        sha256,
        executable: "bin/retroarch".into(),
        core_files: vec!["cores/genesis.so".into()],
    }
}

fn write_symlink_zip(path: &Path) -> String {
    let output = fs::File::create(path).unwrap();
    let mut archive = ZipWriter::new(output);
    archive
        .add_symlink("bin/retroarch", "../outside", SimpleFileOptions::default())
        .unwrap();
    archive
        .start_file("cores/genesis.so", SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"core").unwrap();
    archive.finish().unwrap();
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

#[test]
fn installs_a_verified_local_archive_and_keeps_user_data_separate() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("engine.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"executable", Some(0o755)),
            ("cores/genesis.so", b"core", Some(0o644)),
        ],
    );
    let install_root = sandbox.path().join("engines");
    let user_data = sandbox.path().join("user-data");
    fs::create_dir_all(&user_data).unwrap();
    fs::write(user_data.join("save.srm"), b"save").unwrap();

    let installed = install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &user_data,
        InstallLimits::default(),
    )
    .unwrap();

    assert_eq!(installed.directory, install_root.join("retroarch-genesis"));
    assert_eq!(
        fs::read(installed.directory.join("bin/retroarch")).unwrap(),
        b"executable"
    );
    assert_eq!(
        fs::read(installed.directory.join("cores/genesis.so")).unwrap(),
        b"core"
    );
    assert_eq!(fs::read(user_data.join("save.srm")).unwrap(), b"save");
}

#[test]
fn refuses_to_replace_a_portable_engine_containing_personal_saves() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("engine.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"new", None),
            ("cores/genesis.so", b"new-core", None),
        ],
    );
    let install_root = sandbox.path().join("engines");
    let previous = install_root.join("retroarch-genesis");
    fs::create_dir_all(previous.join("saves")).unwrap();
    fs::write(previous.join("saves/my-game.srm"), b"personal save").unwrap();

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &sandbox.path().join("user-data"),
        InstallLimits::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("personnelles"), "{error}");
    assert_eq!(
        fs::read(previous.join("saves/my-game.srm")).unwrap(),
        b"personal save"
    );
}

#[test]
fn rejects_parent_traversal_without_writing_outside_staging() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("traversal.zip");
    let hash = write_zip(
        &archive,
        &[
            ("../escaped", b"attack", None),
            ("bin/retroarch", b"executable", None),
            ("cores/genesis.so", b"core", None),
        ],
    );
    let install_root = sandbox.path().join("engines");
    let user_data = sandbox.path().join("user-data");

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &user_data,
        InstallLimits::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("chemin ZIP"));
    assert!(!install_root.join("escaped").exists());
    assert!(!install_root.join("retroarch-genesis").exists());
}

#[test]
fn rejects_duplicate_paths_case_insensitively() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("duplicate.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"first", None),
            ("BIN/RETROARCH", b"second", None),
            ("cores/genesis.so", b"core", None),
        ],
    );

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &sandbox.path().join("engines"),
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("dupliqué"));
}

#[test]
fn rejects_symbolic_links() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("symlink.zip");
    let hash = write_symlink_zip(&archive);

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &sandbox.path().join("engines"),
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("lien symbolique"));
}

#[test]
fn rejects_zip_bombs_using_declared_uncompressed_limits() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("bomb.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"far-too-large", None),
            ("cores/genesis.so", b"core", None),
        ],
    );
    let limits = InstallLimits {
        max_file_bytes: 8,
        ..InstallLimits::default()
    };

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &sandbox.path().join("engines"),
        &sandbox.path().join("data"),
        limits,
    )
    .unwrap_err();

    assert!(error.to_string().contains("limite"));
    assert!(!sandbox.path().join("engines/retroarch-genesis").exists());
}

#[test]
fn refuses_an_unsafe_publish_and_preserves_the_previous_installation() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("engine.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"new", None),
            ("cores/genesis.so", b"new-core", None),
        ],
    );
    let install_root = sandbox.path().join("engines");
    let previous = install_root.join("retroarch-genesis");
    fs::create_dir_all(&previous).unwrap();
    fs::write(previous.join("version"), b"old").unwrap();
    fs::create_dir_all(install_root.join(".retroarch-genesis.rollback")).unwrap();

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("sauvegarde"));
    assert_eq!(fs::read(previous.join("version")).unwrap(), b"old");
    assert!(!previous.join("bin/retroarch").exists());
}

#[test]
fn checksum_or_required_file_failure_preserves_previous_installation() {
    let sandbox = TempDir::new().unwrap();
    let install_root = sandbox.path().join("engines");
    let previous = install_root.join("retroarch-genesis");
    fs::create_dir_all(&previous).unwrap();
    fs::write(previous.join("version"), b"old").unwrap();
    let archive = sandbox.path().join("incomplete.zip");
    let hash = write_zip(&archive, &[("bin/retroarch", b"new", None)]);

    let mut wrong_hash = manifest("0".repeat(64));
    let checksum_error = install_engine_package(
        &wrong_hash,
        &archive,
        &install_root,
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap_err();
    assert!(checksum_error.to_string().contains("SHA-256"));

    wrong_hash.sha256 = hash;
    let missing_error = install_engine_package(
        &wrong_hash,
        &archive,
        &install_root,
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap_err();
    assert!(missing_error.to_string().contains("absent"));
    assert_eq!(fs::read(previous.join("version")).unwrap(), b"old");
}

#[test]
fn rejects_absolute_and_windows_style_entry_paths() {
    for dangerous in ["/tmp/retroarch", "C:/retroarch.exe", "bin\\retroarch.exe"] {
        let sandbox = TempDir::new().unwrap();
        let archive = sandbox.path().join("absolute.zip");
        let hash = write_zip(
            &archive,
            &[
                (dangerous, b"attack", None),
                ("bin/retroarch", b"executable", None),
                ("cores/genesis.so", b"core", None),
            ],
        );
        let error = install_engine_package(
            &manifest(hash),
            &archive,
            &sandbox.path().join("engines"),
            &sandbox.path().join("data"),
            InstallLimits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("chemin ZIP"), "{dangerous}");
    }
}

#[test]
fn atomically_replaces_a_previous_installation_and_removes_the_backup() {
    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("engine.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"new", None),
            ("cores/genesis.so", b"new-core", None),
        ],
    );
    let install_root = sandbox.path().join("engines");
    let previous = install_root.join("retroarch-genesis");
    fs::create_dir_all(&previous).unwrap();
    fs::write(previous.join("version"), b"old").unwrap();

    install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &sandbox.path().join("data"),
        InstallLimits::default(),
    )
    .unwrap();

    assert_eq!(fs::read(previous.join("bin/retroarch")).unwrap(), b"new");
    assert!(!previous.join("version").exists());
    assert!(!install_root.join(".retroarch-genesis.rollback").exists());
}

#[cfg(unix)]
#[test]
fn rejects_install_root_aliasing_user_data_through_a_symlink() {
    use std::os::unix::fs::symlink;

    let sandbox = TempDir::new().unwrap();
    let archive = sandbox.path().join("engine.zip");
    let hash = write_zip(
        &archive,
        &[
            ("bin/retroarch", b"new", None),
            ("cores/genesis.so", b"core", None),
        ],
    );
    let install_root = sandbox.path().join("engines");
    fs::create_dir(&install_root).unwrap();
    let user_data_alias = sandbox.path().join("user-data-alias");
    symlink(&install_root, &user_data_alias).unwrap();

    let error = install_engine_package(
        &manifest(hash),
        &archive,
        &install_root,
        &user_data_alias,
        InstallLimits::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("séparés"));
    assert!(!install_root.join("retroarch-genesis").exists());
}
