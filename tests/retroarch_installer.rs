use monolith::retroarch_installer::retroarch_install_directory;
use std::path::Path;

#[test]
fn retroarch_is_installed_under_the_selected_monolith_root() {
    assert_eq!(
        retroarch_install_directory(Path::new("/opt/MONOLITH")).unwrap(),
        Path::new("/opt/MONOLITH/tools/retroarch")
    );
}

#[test]
fn retroarch_rejects_a_relative_install_root() {
    assert!(retroarch_install_directory(Path::new("MONOLITH")).is_err());
}
