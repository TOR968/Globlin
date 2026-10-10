use super::*;

#[test]
fn a_plain_launch_is_one_the_user_opened() {
    assert_eq!(launch_kind(["globlin.exe"].iter()), Launch::Opened);
    assert_eq!(
        launch_kind(["globlin.exe", "--other"].iter()),
        Launch::Opened
    );
}

#[test]
fn the_background_flag_marks_an_autostart_launch() {
    assert_eq!(
        launch_kind(["globlin.exe", platform::BACKGROUND_FLAG].iter()),
        Launch::Background
    );
}

#[test]
fn only_the_restart_flag_marks_a_replaced_launch() {
    assert_eq!(
        launch_kind(["globlin.exe", selfupdate::RESTART_FLAG].iter()),
        Launch::Replaced
    );
    assert_eq!(
        launch_kind(
            [
                "globlin.exe",
                platform::BACKGROUND_FLAG,
                selfupdate::RESTART_FLAG
            ]
            .iter()
        ),
        Launch::Replaced
    );
}
