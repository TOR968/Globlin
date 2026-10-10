use super::*;

#[test]
#[ignore = "writes to the HKCU Run key: cargo test -- --ignored --exact platform::windows::tests::autostart_round_trips_through_the_run_key"]
fn autostart_round_trips_through_the_run_key() {
    let was_enabled = autostart_enabled();

    set_autostart(true).unwrap();
    assert!(autostart_enabled());

    let value: String = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .unwrap()
        .get_value(RUN_VALUE)
        .unwrap();
    assert!(value.contains("globlin"), "unexpected value: {value}");

    set_autostart(false).unwrap();
    assert!(!autostart_enabled());

    set_autostart(was_enabled).unwrap();
    assert_eq!(autostart_enabled(), was_enabled);
}

#[test]
#[ignore = "shows a real toast: cargo test -- --ignored --exact platform::windows::tests::raises_a_real_notification"]
fn raises_a_real_notification() {
    notify(
        "Globlin — test notification",
        "prettier  3.9.6 → 3.10.0\nvercel  58.9.1 → 59.0.0",
    )
    .unwrap();

    let artwork = super::super::data_dir().join("app.ico");
    assert!(
        artwork.is_file(),
        "the toast artwork should have been written"
    );

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!(
            r"Software\Classes\AppUserModelId\{APP_USER_MODEL_ID}"
        ))
        .unwrap();
    assert_eq!(
        key.get_value::<String, _>("DisplayName").unwrap(),
        DISPLAY_NAME
    );
    assert_eq!(
        key.get_value::<String, _>("IconUri").unwrap(),
        artwork.display().to_string()
    );
}

#[test]
fn disabling_autostart_twice_is_not_an_error() {
    if autostart_enabled() {
        return;
    }
    assert!(set_autostart(false).is_ok());
}

#[test]
fn the_data_dir_exists_after_it_is_requested() {
    assert!(super::super::data_dir().is_dir());
}

#[test]
fn the_run_value_starts_globlin_in_the_background() {
    assert_eq!(
        run_value(Path::new(r"C:\Tools\globlin.exe")),
        r#""C:\Tools\globlin.exe" --background"#
    );
}

#[test]
fn registered_path_entries_the_launcher_dropped_are_appended_after_the_current_ones() {
    assert_eq!(
        append_missing_paths(
            r"C:\Windows\system32;C:\Tools",
            r"C:\Windows\System32\;C:\nvm4w\nodejs;C:\Users\me\AppData\Roaming\npm"
        ),
        r"C:\Windows\system32;C:\Tools;C:\nvm4w\nodejs;C:\Users\me\AppData\Roaming\npm"
    );
}

#[test]
fn an_empty_current_path_is_rebuilt_from_the_registry() {
    assert_eq!(append_missing_paths("", r"C:\a;;C:\b;"), r"C:\a;C:\b");
}

#[test]
fn registry_paths_expand_the_variables_they_name() {
    let lookup = |name: &str| match name {
        "USERPROFILE" => Some(r"C:\Users\me".to_string()),
        "SystemRoot" => Some(r"C:\Windows".to_string()),
        _ => None,
    };

    assert_eq!(
        expand_variables(r"%USERPROFILE%\scoop\shims;%SystemRoot%\system32", lookup),
        r"C:\Users\me\scoop\shims;C:\Windows\system32"
    );
}

#[test]
fn an_unknown_or_unclosed_variable_is_left_as_written() {
    let lookup = |_: &str| None;

    assert_eq!(expand_variables(r"%NOPE%\bin", lookup), r"%NOPE%\bin");
    assert_eq!(expand_variables(r"50%\bin", lookup), r"50%\bin");
}
