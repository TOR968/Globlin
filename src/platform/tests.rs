use super::*;

#[test]
fn an_appimage_launch_names_the_appimage_file_as_the_program() {
    assert_eq!(
        appimage_from(Some(OsString::from(
            "/home/me/Apps/globlin-x86_64.AppImage"
        ))),
        Some(PathBuf::from("/home/me/Apps/globlin-x86_64.AppImage"))
    );
}

#[test]
fn a_launch_outside_an_appimage_has_no_appimage_file() {
    assert_eq!(appimage_from(None), None);
    assert_eq!(appimage_from(Some(OsString::new())), None);
}

#[test]
fn outside_linux_the_program_is_always_the_running_executable() {
    if cfg!(not(target_os = "linux")) {
        assert_eq!(appimage(), None);
        assert_eq!(program_path().unwrap(), std::env::current_exe().unwrap());
    }
}
