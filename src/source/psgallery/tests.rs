use super::*;

const LISTING: &str = "Pester 5.6.1
PSReadLine 2.3.6
Az.Accounts 3.0.4
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn each_row_is_a_module_and_its_version() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["Pester", "PSReadLine", "Az.Accounts"]);
    assert_eq!(installed[0].version, "5.6.1");
    assert_eq!(installed[0].source, SourceKind::PsGallery);
}

#[test]
fn a_warning_line_from_powershell_is_not_a_module() {
    assert!(parse_listing("WARNING: Unable to find module repositories.\n").is_empty());
    assert!(parse_listing("").is_empty());
}

#[test]
fn a_module_name_is_quoted_so_it_can_never_become_script() {
    assert_eq!(quoted("Pester"), "'Pester'");
    assert_eq!(
        quoted("x'; Remove-Item C:\\ -Recurse; '"),
        "'x''; Remove-Item C:\\ -Recurse; '''"
    );
}

#[test]
fn the_update_command_updates_the_module_in_the_current_user_scope() {
    let gallery = PsGallery {
        command: PathBuf::from("pwsh"),
    };

    assert_eq!(
        arguments(&gallery.update_command("Pester").unwrap()),
        vec![
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Update-Module -Name 'Pester' -Scope CurrentUser -Force"
        ]
    );
}

#[test]
fn the_uninstall_command_removes_the_module() {
    let gallery = PsGallery {
        command: PathBuf::from("pwsh"),
    };

    assert_eq!(
        arguments(&gallery.uninstall_command("Pester").unwrap()),
        vec![
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Uninstall-Module -Name 'Pester'"
        ]
    );
}

#[test]
#[ignore = "runs the real PowerShell: cargo test -- --ignored --exact source::psgallery::tests::the_real_module_listing_still_parses"]
fn the_real_module_listing_still_parses() {
    let installed = PsGallery::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
