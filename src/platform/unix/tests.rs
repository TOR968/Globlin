use super::*;

#[test]
fn the_login_path_comes_first_and_nothing_is_listed_twice() {
    assert_eq!(
        merge_paths(
            "/opt/homebrew/bin:/usr/bin:/Users/me/.cargo/bin",
            "/usr/bin:/bin:/usr/sbin"
        ),
        "/opt/homebrew/bin:/usr/bin:/Users/me/.cargo/bin:/bin:/usr/sbin"
    );
}

#[test]
fn empty_path_entries_are_dropped() {
    assert_eq!(merge_paths("/a::/b:", ":/c"), "/a:/b:/c");
}

#[test]
fn the_path_is_read_between_the_markers_despite_shell_noise() {
    let output = format!(
        "Last login: today\nwelcome to zsh\n\n{PATH_MARKER}/opt/homebrew/bin:/usr/bin{PATH_MARKER}\n"
    );

    assert_eq!(
        extract_path(&output).as_deref(),
        Some("/opt/homebrew/bin:/usr/bin")
    );
}

#[test]
fn output_without_both_markers_yields_no_path() {
    assert_eq!(extract_path("/usr/bin:/bin"), None);
    assert_eq!(extract_path(&format!("{PATH_MARKER}/usr/bin")), None);
    assert_eq!(extract_path(&format!("{PATH_MARKER}{PATH_MARKER}")), None);
}
