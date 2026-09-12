use super::*;

const LISTING: &str = "C:\\Users\\x\\go\\bin\\goimports.exe: go1.22.2
\tpath\tgolang.org/x/tools/cmd/goimports
\tmod\tgolang.org/x/tools\tv0.20.0\th1:hz/CVckiOxybQvFw6h7b/q80NTr9IUQb4s1IIzW7KNY=
\tbuild\t-buildmode=exe
C:\\Users\\x\\go\\bin\\gopls.exe: go1.22.2
\tpath\tgolang.org/x/tools/gopls
\tmod\tgolang.org/x/tools/gopls\tv0.15.3\th1:zbdOidFrPTc8Bx0YrN5QKgJ0zCjyGi0L27sKQ/bDG5o=
\tdep\tgithub.com/BurntSushi/toml\tv1.2.1\th1:9F2/+DoOYIOksmaJFPw1tGFy1eDnIJXg+UHjuD8lTak=
C:\\Users\\x\\go\\bin\\test.exe: go1.22.2
\tpath\ttest
\tmod\ttest\t(devel)\t
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_package_path_names_the_tool_because_that_is_what_go_install_takes() {
    let binaries = parse_listing(LISTING);
    let packages: Vec<&str> = binaries
        .iter()
        .map(|binary| binary.package.as_str())
        .collect();

    assert_eq!(
        packages,
        vec![
            "golang.org/x/tools/cmd/goimports",
            "golang.org/x/tools/gopls"
        ]
    );
}

#[test]
fn the_module_is_kept_apart_from_the_package_because_only_it_can_be_queried() {
    let binaries = parse_listing(LISTING);

    assert_eq!(binaries[0].module, "golang.org/x/tools");
    assert_eq!(binaries[0].package, "golang.org/x/tools/cmd/goimports");
}

#[test]
fn the_leading_v_is_dropped_so_versions_render_like_every_other_source() {
    assert_eq!(parse_listing(LISTING)[1].version, "0.15.3");
}

#[test]
fn a_locally_built_binary_has_no_released_version_to_compare_against() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|binary| binary.package == "test"));
}

#[test]
fn dependency_rows_are_not_binaries() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|binary| binary.module.contains("BurntSushi")));
}

#[test]
fn output_that_is_not_a_listing_is_empty_rather_than_fatal() {
    assert!(parse_listing("go: no such tool\n").is_empty());
    assert!(parse_listing("").is_empty());
}

#[test]
fn the_go_update_command_installs_the_package_at_latest() {
    let go = Go {
        command: PathBuf::from("go"),
    };

    assert_eq!(
        arguments(&go.update_command("golang.org/x/tools/gopls").unwrap()),
        vec!["install", "golang.org/x/tools/gopls@latest"]
    );
}

#[test]
fn go_has_no_uninstall_of_its_own() {
    let go = Go {
        command: PathBuf::from("go"),
    };

    assert!(go.uninstall_command("golang.org/x/tools/gopls").is_none());
}

#[test]
#[ignore = "runs the real go: cargo test -- --ignored --exact source::golang::tests::the_real_go_listing_still_parses"]
fn the_real_go_listing_still_parses() {
    let go = Go::new().unwrap();
    let bin = go.bin_dir().expect("a go bin directory");
    let binaries = parse_listing(&go.run(&["version", "-m", &bin.to_string_lossy()]));

    assert!(
        !binaries.is_empty(),
        "the go bin directory was read as a whole"
    );
    assert!(binaries
        .iter()
        .all(|binary| !binary.package.is_empty() && !binary.version.is_empty()));
}
