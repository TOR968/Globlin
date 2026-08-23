use super::*;

#[test]
fn only_the_restart_flag_marks_a_replaced_launch() {
    assert!(was_replaced(
        ["globlin.exe", selfupdate::RESTART_FLAG].iter()
    ));
    assert!(!was_replaced(["globlin.exe"].iter()));
    assert!(!was_replaced(["globlin.exe", "--other"].iter()));
}
