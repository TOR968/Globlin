use super::*;

#[test]
fn the_autostart_entry_starts_the_exe_in_the_background() {
    let entry = autostart_desktop_entry(Path::new("/usr/bin/globlin"));

    assert!(entry.starts_with("[Desktop Entry]\n"), "{entry}");
    assert!(
        entry.contains("\nExec=\"/usr/bin/globlin\" --background\n"),
        "{entry}"
    );
    assert!(
        entry.contains("\nX-GNOME-Autostart-enabled=true\n"),
        "{entry}"
    );
}

#[test]
fn shell_and_quote_characters_in_the_exe_path_are_escaped() {
    assert_eq!(
        desktop_quote("/home/me/My \"Apps\"/$x`y`/globlin"),
        "\"/home/me/My \\\"Apps\\\"/\\$x\\`y\\`/globlin\""
    );
}
