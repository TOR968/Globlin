use super::*;

#[test]
fn the_global_dir_is_the_line_that_is_not_yarn_chatter() {
    let stdout =
        "yarn global v1.22.22\nC:\\Users\\x\\AppData\\Local\\Yarn\\Data\\global\nDone in 0.2s.\n";

    assert_eq!(
        parse_global_dir(stdout),
        Some(PathBuf::from(
            "C:\\Users\\x\\AppData\\Local\\Yarn\\Data\\global"
        ))
    );
}

#[test]
fn a_bare_path_is_taken_as_written() {
    assert_eq!(
        parse_global_dir("/home/user/.config/yarn/global\n"),
        Some(PathBuf::from("/home/user/.config/yarn/global"))
    );
}

#[test]
fn a_warning_line_does_not_become_the_path() {
    let stdout = "warning package.json: No license field\n/home/user/.config/yarn/global\n";

    assert_eq!(
        parse_global_dir(stdout),
        Some(PathBuf::from("/home/user/.config/yarn/global"))
    );
}

#[test]
fn no_output_resolves_to_no_directory() {
    assert_eq!(parse_global_dir(""), None);
    assert_eq!(parse_global_dir("yarn global v1.22.22\n"), None);
}

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_yarn_update_command_adds_the_latest_globally() {
    let yarn = Yarn {
        command: PathBuf::from("yarn"),
    };

    assert_eq!(
        arguments(&yarn.update_command("eslint").unwrap()),
        vec!["global", "add", "eslint@latest"]
    );
}

#[test]
fn the_yarn_uninstall_command_removes_the_package_globally() {
    let yarn = Yarn {
        command: PathBuf::from("yarn"),
    };

    assert_eq!(
        arguments(&yarn.uninstall_command("eslint").unwrap()),
        vec!["global", "remove", "eslint"]
    );
}
