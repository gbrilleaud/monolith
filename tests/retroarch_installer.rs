use monolith::retroarch_installer::{
    pcsx2_install_directory, retroarch_install_directory, retroarch_setup_path,
    xenia_install_directory,
};
use std::path::Path;

#[test]
fn retroarch_is_installed_under_the_selected_monolith_root() {
    assert_eq!(
        retroarch_install_directory(Path::new("/opt/MONOLITH")).unwrap(),
        Path::new("/opt/MONOLITH/tools/retroarch")
    );
}

#[test]
fn retroarch_setup_is_staged_outside_the_install_directory() {
    assert_eq!(
        retroarch_setup_path(Path::new("/opt/MONOLITH")).unwrap(),
        Path::new("/opt/MONOLITH/downloads/RetroArch-Win64-setup.exe")
    );
}

#[test]
fn pcsx2_is_installed_under_the_selected_monolith_root() {
    assert_eq!(
        pcsx2_install_directory(Path::new("/opt/MONOLITH")).unwrap(),
        Path::new("/opt/MONOLITH/tools/pcsx2")
    );
}

#[test]
fn xenia_is_extracted_under_the_selected_monolith_root() {
    assert_eq!(
        xenia_install_directory(Path::new("/opt/MONOLITH")).unwrap(),
        Path::new("/opt/MONOLITH/tools/xenia")
    );
}

#[test]
fn retroarch_rejects_a_relative_install_root() {
    assert!(retroarch_install_directory(Path::new("MONOLITH")).is_err());
}
