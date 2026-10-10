use super::*;

#[cfg(windows)]
fn printing(text: &str) -> Command {
    let mut command = Command::new("cmd");
    command.args(["/C", "echo", text]);
    command
}

#[cfg(not(windows))]
fn printing(text: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", &format!("echo {text}")]);
    command
}

#[cfg(windows)]
fn lingering() -> Command {
    let mut command = Command::new("ping");
    command.args(["-n", "30", "127.0.0.1"]);
    command
}

#[cfg(not(windows))]
fn lingering() -> Command {
    let mut command = Command::new("sleep");
    command.arg("30");
    command
}

#[test]
fn a_command_that_finishes_in_time_hands_back_its_output() {
    let output = output_within(&mut printing("globlin"), Duration::from_secs(30)).unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "globlin");
}

#[test]
fn a_command_that_outlives_its_limit_is_stopped_and_reported() {
    let started = Instant::now();

    let error = output_within(&mut lingering(), Duration::from_millis(300)).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(
        error.to_string().contains("did not finish within"),
        "{error}"
    );
    assert!(started.elapsed() < Duration::from_secs(20));
}
