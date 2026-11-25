use super::*;

#[test]
fn the_launch_agent_runs_the_exe_at_login_under_the_bundle_label() {
    let plist = launch_agent_plist(Path::new(
        "/Applications/Globlin.app/Contents/MacOS/globlin",
    ));

    assert!(
        plist.contains("<string>dev.globlin.app</string>"),
        "{plist}"
    );
    assert!(
        plist.contains("<string>/Applications/Globlin.app/Contents/MacOS/globlin</string>"),
        "{plist}"
    );
    assert!(
        plist.contains("globlin</string>\n        <string>--background</string>"),
        "{plist}"
    );
    assert!(
        plist.contains("<key>RunAtLoad</key>\n    <true/>"),
        "{plist}"
    );
}

#[test]
fn a_path_with_xml_characters_is_escaped_in_the_launch_agent() {
    let plist = launch_agent_plist(Path::new("/Users/me/Apps & Tools/<x>/globlin"));

    assert!(
        plist.contains("<string>/Users/me/Apps &amp; Tools/&lt;x&gt;/globlin</string>"),
        "{plist}"
    );
}

#[test]
fn a_login_item_that_is_enabled_or_awaiting_approval_counts_as_on() {
    assert!(counts_as_enabled(1));
    assert!(counts_as_enabled(2));
}

#[test]
fn a_login_item_that_is_unregistered_missing_or_unknown_counts_as_off() {
    assert!(!counts_as_enabled(0));
    assert!(!counts_as_enabled(3));
    assert!(!counts_as_enabled(42));
}
