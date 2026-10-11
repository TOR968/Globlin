use super::*;
use crate::config::Sources;

fn only(kinds: &[SourceKind]) -> Config {
    let mut config = Config {
        sources: Sources {
            npm: false,
            bun: false,
            pnpm: false,
            yarn: false,
            pipx: false,
            uv: false,
            scoop: false,
            cargo: false,
            go: false,
            dotnet: false,
            psgallery: false,
            gem: false,
            brew: false,
            winget: false,
            choco: false,
        },
        ..Default::default()
    };
    for kind in kinds {
        config.set_source_enabled(*kind, true);
    }
    config
}

#[test]
fn disabling_every_source_is_an_error_rather_than_an_empty_result() {
    let error = enabled(&only(&[]))
        .err()
        .expect("disabling every source should fail")
        .to_string();
    assert!(error.contains("no package sources are enabled"), "{error}");
}

#[test]
fn npm_alone_is_enough_to_run() {
    let sources = enabled(&only(&[SourceKind::Npm])).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].kind(), SourceKind::Npm);
}

#[test]
fn a_missing_optional_source_does_not_break_the_working_one() {
    let sources = enabled(&only(&[SourceKind::Npm, SourceKind::Bun])).unwrap();
    assert!(sources
        .iter()
        .any(|source| source.kind() == SourceKind::Npm));
}

#[test]
fn a_hidden_command_targets_the_program_it_was_given() {
    let command = hidden_command(Path::new("npm.cmd"));
    assert_eq!(command.get_program(), "npm.cmd");
}

#[test]
fn a_program_that_is_not_on_path_is_not_found() {
    assert_eq!(find_on_path("globlin-absent-4b1e"), None);
}

#[test]
fn every_source_kind_round_trips_through_its_label() {
    for kind in KINDS {
        assert_eq!(SourceKind::from_label(kind.label()), Some(kind));
    }
}

#[test]
fn only_the_enabled_sources_are_built() {
    let config = only(&[SourceKind::Npm]);

    assert!(config.source_enabled(SourceKind::Npm));
    assert!(!config.source_enabled(SourceKind::Winget));
}

#[test]
fn a_node_modules_root_without_a_manifest_is_empty_not_a_panic() {
    let root = std::env::temp_dir().join("globlin-test-absent-root");

    assert!(node_modules_listing(&root, SourceKind::Yarn).is_empty());
}

fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("globlin-resolver-{label}"));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(windows)]
#[test]
fn a_bare_name_finds_its_cmd_shim_on_windows() {
    let dir = scratch_dir("cmd-shim");
    std::fs::write(dir.join("fake-tool.cmd"), "@exit 0").unwrap();

    assert_eq!(
        find_in(dir.as_os_str(), "fake-tool"),
        Some(dir.join("fake-tool.cmd"))
    );
}

#[cfg(windows)]
#[test]
fn an_exe_wins_over_a_cmd_shim_in_the_same_directory() {
    let dir = scratch_dir("exe-first");
    std::fs::write(dir.join("fake-tool.cmd"), "@exit 0").unwrap();
    std::fs::write(dir.join("fake-tool.exe"), "").unwrap();

    assert_eq!(
        find_in(dir.as_os_str(), "fake-tool"),
        Some(dir.join("fake-tool.exe"))
    );
}

#[cfg(windows)]
#[test]
fn an_extensionless_file_is_not_an_executable_on_windows() {
    let dir = scratch_dir("extensionless");
    std::fs::write(dir.join("fake-tool"), "#!/bin/sh").unwrap();

    assert_eq!(find_in(dir.as_os_str(), "fake-tool"), None);
}

#[cfg(not(windows))]
#[test]
fn a_file_without_an_execute_bit_is_not_an_executable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch_dir("no-exec");
    let tool = dir.join("fake-tool");
    std::fs::write(
        &tool,
        "#!/bin/sh
",
    )
    .unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(find_in(dir.as_os_str(), "fake-tool"), None);

    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(find_in(dir.as_os_str(), "fake-tool"), Some(tool));
}

#[test]
fn the_first_path_directory_that_has_the_tool_wins() {
    let first = scratch_dir("first");
    let second = scratch_dir("second");
    let file_name = format!("fake-tool{}", std::env::consts::EXE_SUFFIX);
    for dir in [&first, &second] {
        let tool = dir.join(&file_name);
        std::fs::write(&tool, "").unwrap();
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let path = std::env::join_paths([&first, &second]).unwrap();

    assert_eq!(find_in(&path, "fake-tool"), Some(first.join(&file_name)));
}

#[test]
fn an_empty_listing_names_the_command_and_carries_its_stderr() {
    let output = std::process::Output {
        status: std::process::ExitStatus::default(),
        stdout: Vec::new(),
        stderr: b"npm error code EBUSY\n".to_vec(),
    };

    let message = silent_listing("npm ls -g", &output);

    assert!(
        message.starts_with("npm ls -g listed nothing and "),
        "{message}"
    );
    assert!(message.ends_with(": npm error code EBUSY"), "{message}");
}

fn finished(code_zero: bool) -> std::process::Output {
    #[cfg(not(windows))]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    #[cfg(windows)]
    let status = std::process::ExitStatus::from_raw(u32::from(!code_zero));
    #[cfg(not(windows))]
    let status = std::process::ExitStatus::from_raw(i32::from(!code_zero) << 8);
    std::process::Output {
        status,
        stdout: Vec::new(),
        stderr: b"source not reachable".to_vec(),
    }
}

#[test]
fn a_failed_command_that_listed_nothing_is_an_error_not_an_empty_source() {
    let error = listed(Vec::<String>::new(), "pnpm ls -g", &finished(false))
        .unwrap_err()
        .to_string();

    assert!(
        error.starts_with("pnpm ls -g listed nothing and "),
        "{error}"
    );
    assert!(error.ends_with(": source not reachable"), "{error}");
}

#[test]
fn a_successful_command_that_listed_nothing_is_an_empty_source() {
    assert!(listed(Vec::<String>::new(), "pnpm ls -g", &finished(true))
        .unwrap()
        .is_empty());
}

#[test]
fn rows_from_a_command_that_exited_badly_are_still_kept() {
    assert_eq!(
        listed(vec!["a".to_string()], "winget list", &finished(false)).unwrap(),
        ["a"]
    );
}

#[test]
fn a_source_that_is_not_built_is_reported_by_name() {
    let sources: Vec<Box<dyn PackageSource>> = Vec::new();

    let error = source_for(&sources, "prettier", SourceKind::Pnpm)
        .err()
        .unwrap();

    assert_eq!(error, "prettier: the pnpm source is not available\n");
}

#[test]
fn a_read_only_source_runs_nothing() {
    let error = run_change(None, "Git.Git", SourceKind::Winget).unwrap_err();

    assert_eq!(error, "Git.Git: the winget source is read-only\n");
}

#[test]
fn a_manifest_spec_that_names_a_path_a_link_a_git_url_or_an_alias_is_another_origin() {
    for spec in [
        "file:../tool",
        "link:tool",
        "github:me/tool",
        "me/tool",
        "git+https://example.com/tool.git",
        "https://example.com/tool.tgz",
        "npm:other@1.0.0",
    ] {
        assert!(names_another_origin(spec), "{spec}");
    }
    for spec in ["^1.2.3", "1.2.3", "latest", "*", ">=1 <2", "1.x"] {
        assert!(!names_another_origin(spec), "{spec}");
    }
}

struct Present(SourceKind);

impl PackageSource for Present {
    fn kind(&self) -> SourceKind {
        self.0
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        Ok(Vec::new())
    }

    fn update_command(&self, _name: &str) -> Option<Command> {
        None
    }

    fn uninstall_command(&self, _name: &str) -> Option<Command> {
        None
    }
}

fn present(kind: SourceKind) -> Box<dyn PackageSource> {
    Box::new(Present(kind))
}

fn broken(message: &str) -> Result<Box<dyn PackageSource>> {
    Err(message.into())
}

#[test]
fn a_machine_without_any_enabled_package_manager_is_an_empty_check_not_a_failure() {
    let sources = assemble([Err(not_on_path("npm")), Err(not_on_path("cargo"))]).unwrap();

    assert!(sources.is_empty());
}

#[test]
fn an_installed_package_manager_that_fails_to_start_is_still_a_failure() {
    let error = assemble([Err(not_on_path("npm")), broken("\"npm_cmd\" was refused")])
        .err()
        .expect("a broken source with nothing else working should fail")
        .to_string();

    assert!(error.contains("npm_cmd"), "{error}");
    assert!(!error.contains("not found on PATH"), "{error}");
}

#[test]
fn a_working_source_outweighs_both_missing_and_broken_ones() {
    let sources = assemble([
        Err(not_on_path("bun")),
        broken("pnpm could not be started"),
        Ok(present(SourceKind::Npm)),
    ])
    .unwrap();

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].kind(), SourceKind::Npm);
}

#[test]
fn a_missing_program_is_told_apart_from_a_broken_one() {
    assert!(not_on_path("go").is::<NotInstalled>());
    assert_eq!(not_on_path("go").to_string(), "go was not found on PATH");
    assert!(!crate::Error::from("go crashed").is::<NotInstalled>());
}
