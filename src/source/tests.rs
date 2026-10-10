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
