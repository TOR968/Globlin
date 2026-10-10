use super::{is_setup_path, is_system_package_path, is_winget_path, Manager};
use std::path::Path;

#[cfg(windows)]
#[test]
fn a_user_scope_winget_package_directory_is_managed() {
    assert!(is_winget_path(Path::new(
        r"C:\Users\me\AppData\Local\Microsoft\WinGet\Packages\TOR968.Globlin_Microsoft.Winget.Source_8wekyb3d8bbwe\globlin.exe"
    )));
}

#[cfg(windows)]
#[test]
fn a_machine_scope_winget_package_directory_is_managed() {
    assert!(is_winget_path(Path::new(
        r"C:\Program Files\WinGet\Packages\TOR968.Globlin_Microsoft.Winget.Source\globlin.exe"
    )));
}

#[cfg(windows)]
#[test]
fn the_match_ignores_case_because_windows_paths_do() {
    assert!(is_winget_path(Path::new(
        r"C:\Users\me\AppData\Local\Microsoft\winget\packages\TOR968.Globlin\globlin.exe"
    )));
}

#[cfg(windows)]
#[test]
fn a_portable_copy_anywhere_else_is_not_managed() {
    assert!(!is_winget_path(Path::new(r"D:\tools\globlin.exe")));
    assert!(!is_winget_path(Path::new(
        r"C:\Users\me\Downloads\globlin.exe"
    )));
}

#[cfg(windows)]
#[test]
fn the_winget_links_directory_is_not_a_package_directory() {
    assert!(!is_winget_path(Path::new(
        r"C:\Users\me\AppData\Local\Microsoft\WinGet\Links\globlin.exe"
    )));
}

#[cfg(windows)]
#[test]
fn a_folder_merely_named_packages_is_not_enough() {
    assert!(!is_winget_path(Path::new(r"D:\packages\globlin.exe")));
}

#[cfg(windows)]
#[test]
fn the_two_components_must_be_adjacent() {
    assert!(!is_winget_path(Path::new(
        r"C:\winget\other\packages\globlin.exe"
    )));
}

#[cfg(windows)]
const SETUP_LOCATION: &str = r"C:\Users\me\AppData\Local\Programs\Globlin\";

#[cfg(windows)]
#[test]
fn an_exe_in_the_recorded_install_location_is_managed_by_setup() {
    assert!(is_setup_path(
        Path::new(r"C:\Users\me\AppData\Local\Programs\Globlin\globlin.exe"),
        Path::new(SETUP_LOCATION)
    ));
}

#[cfg(windows)]
#[test]
fn the_setup_location_matches_regardless_of_case() {
    assert!(is_setup_path(
        Path::new(r"c:\users\ME\appdata\local\programs\globlin\globlin.exe"),
        Path::new(SETUP_LOCATION)
    ));
}

#[cfg(windows)]
#[test]
fn the_trailing_separator_inno_records_does_not_matter() {
    assert!(is_setup_path(
        Path::new(r"C:\Users\me\AppData\Local\Programs\Globlin\globlin.exe"),
        Path::new(r"C:\Users\me\AppData\Local\Programs\Globlin")
    ));
}

#[cfg(windows)]
#[test]
fn a_portable_copy_elsewhere_is_not_managed_by_setup() {
    assert!(!is_setup_path(
        Path::new(r"D:\tools\globlin.exe"),
        Path::new(SETUP_LOCATION)
    ));
}

#[cfg(windows)]
#[test]
fn a_copy_in_a_subdirectory_of_the_install_location_is_not_the_installed_one() {
    assert!(!is_setup_path(
        Path::new(r"C:\Users\me\AppData\Local\Programs\Globlin\old\globlin.exe"),
        Path::new(SETUP_LOCATION)
    ));
}

#[test]
fn a_binary_under_usr_belongs_to_the_system_package_manager() {
    assert!(is_system_package_path(Path::new("/usr/bin/globlin")));
    assert!(is_system_package_path(Path::new("/usr/local/bin/globlin")));
}

#[test]
fn a_binary_in_the_home_directory_is_not_a_system_package() {
    assert!(!is_system_package_path(Path::new("/home/me/bin/globlin")));
    assert!(!is_system_package_path(Path::new("/usr2/bin/globlin")));
}

#[test]
fn each_manager_says_how_to_update_globlin_instead() {
    assert_eq!(Manager::Winget.label(), "winget");
    assert!(Manager::Winget.advice().contains("winget upgrade"));
    assert!(Manager::SystemPackage.advice().contains("package manager"));
}
