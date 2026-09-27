use super::*;

const LISTING: &[u8] = br#"{
  "name": "npm",
  "problems": ["invalid: @kilocode/cli@ C:\\Users\\x\\AppData\\Roaming\\npm\\node_modules\\@kilocode"],
  "dependencies": {
    "prettier": { "version": "3.9.6", "resolved": "https://registry.npmjs.org/prettier/-/prettier-3.9.6.tgz" },
    "@salesforce/cli": { "version": "2.145.6" },
    "vanished": { "required": "^1.0.0", "missing": true },
    "not-semver": { "version": "latest" }
  }
}"#;

#[test]
fn parses_scoped_and_plain_packages() {
    let installed = parse_listing(LISTING).unwrap();
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["@salesforce/cli", "prettier"]);
}

#[test]
fn skips_entries_without_a_usable_version() {
    let installed = parse_listing(LISTING).unwrap();

    assert!(!installed.iter().any(|item| item.name == "vanished"));
    assert!(!installed.iter().any(|item| item.name == "not-semver"));
}

#[test]
fn reads_the_version_and_tags_the_source() {
    let installed = parse_listing(LISTING).unwrap();
    let prettier = installed
        .iter()
        .find(|item| item.name == "prettier")
        .unwrap();

    assert_eq!(prettier.version, "3.9.6");
    assert_eq!(prettier.source, SourceKind::Npm);
}

#[test]
fn a_listing_without_dependencies_is_empty_not_an_error() {
    let installed = parse_listing(br#"{"name":"npm"}"#).unwrap();
    assert!(installed.is_empty());
}

#[test]
fn unparseable_output_is_an_error() {
    assert!(parse_listing(b"").is_err());
    assert!(parse_listing(b"npm ERR! code ENOENT").is_err());
}

const STRICT_REFUSAL: &str = "npm warn install-scripts .npmrc allow-scripts setting is being ignored because --allow-scripts was passed on the command line\r
npm error code ESTRICTALLOWSCRIPTS\r
npm error --strict-allow-scripts: 5 package(s) have install scripts not covered by allowScripts:\r
npm error   @deepseek-ai/dsh-subprocess-local@0.1.7-rc.2 (postinstall: node scripts/ensure-spawn-helper.mjs)\r
npm error   koffi@3.3.2 (install: node ./cnoke.cjs -P . -D src/koffi --prebuild --release)\r
npm error   node-pty@1.2.0-beta.15 (install: node scripts/prebuild.js || node-gyp rebuild; postinstall: node scripts/post-install.js)\r
npm error   @google/genai@1.52.0 (preinstall: echo 'preinstall: no-op')\r
npm error   protobufjs@7.6.6 (postinstall: node scripts/postinstall)\r
npm error Allow them with `--allow-scripts`, persist them with `npm config set allow-scripts=@deepseek-ai/dsh-subprocess-local,koffi,node-pty,@google/genai,protobufjs --location=user`, or bypass this check with `--dangerously-allow-all-scripts`.\r
npm error A complete log of this run can be found in: C:\\Users\\x\\AppData\\Local\\npm-cache\\_logs\\2026-09-27T18_36_45_426Z-debug-0.log\r
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_npm_update_runs_the_packages_own_install_scripts_and_fails_on_any_other_blocked_one() {
    assert_eq!(
        install_arguments("@anthropic-ai/claude-code", ""),
        vec![
            "install",
            "-g",
            "@anthropic-ai/claude-code@latest",
            "--allow-scripts=@anthropic-ai/claude-code",
            "--strict-allow-scripts",
        ]
    );
}

#[test]
fn the_npm_update_keeps_the_scripts_the_user_already_approved_in_npmrc() {
    assert_eq!(
        install_arguments(
            "@deepseek-ai/dsh",
            "@stripe/cli,koffi,node-pty
"
        ),
        vec![
            "install",
            "-g",
            "@deepseek-ai/dsh@latest",
            "--allow-scripts=@stripe/cli,koffi,node-pty,@deepseek-ai/dsh",
            "--strict-allow-scripts",
        ]
    );
}

#[test]
fn a_package_already_approved_in_npmrc_is_not_listed_twice() {
    assert_eq!(
        merge_allow_list(
            "@stripe/cli,@anthropic-ai/claude-code",
            ["@anthropic-ai/claude-code".to_string()]
        ),
        "@stripe/cli,@anthropic-ai/claude-code"
    );
}

#[test]
fn approvals_are_appended_after_the_entries_already_in_npmrc() {
    assert_eq!(
        merge_allow_list(
            "@stripe/cli\n",
            ["koffi@3.3.2".to_string(), "@stripe/cli".to_string()]
        ),
        "@stripe/cli,koffi@3.3.2"
    );
}

#[test]
fn a_strict_refusal_lists_every_blocked_package_with_its_scripts() {
    let blocked = parse_blocked_scripts(STRICT_REFUSAL).unwrap();
    let packages: Vec<String> = blocked.iter().map(BlockedScript::package).collect();

    assert_eq!(
        packages,
        vec![
            "@deepseek-ai/dsh-subprocess-local@0.1.7-rc.2",
            "koffi@3.3.2",
            "node-pty@1.2.0-beta.15",
            "@google/genai@1.52.0",
            "protobufjs@7.6.6",
        ]
    );
    assert_eq!(
        blocked[2].scripts,
        "install: node scripts/prebuild.js || node-gyp rebuild; postinstall: node scripts/post-install.js"
    );
}

#[test]
fn a_scoped_blocked_package_splits_at_the_version_not_the_scope() {
    let blocked = parse_blocked_scripts(STRICT_REFUSAL).unwrap();

    assert_eq!(blocked[3].name, "@google/genai");
    assert_eq!(blocked[3].version, "1.52.0");
}

#[test]
fn a_blocked_package_npm_printed_without_a_version_keeps_an_empty_version() {
    let blocked = parse_blocked_scripts(
        "npm error code ESTRICTALLOWSCRIPTS\nnpm error   linked-thing (postinstall: node x.js)\n",
    )
    .unwrap();

    assert_eq!(blocked[0].name, "linked-thing");
    assert_eq!(blocked[0].version, "");
}

#[test]
fn a_failure_that_is_not_a_strict_refusal_is_not_blocked() {
    assert_eq!(parse_blocked_scripts(""), None);
    assert_eq!(
        parse_blocked_scripts("npm error code E404\nnpm error   404 Not Found - GET https://registry.npmjs.org/nope\n"),
        None
    );
}

#[test]
fn a_strict_refusal_whose_package_lines_cannot_be_read_is_not_blocked() {
    assert_eq!(
        parse_blocked_scripts("npm error code ESTRICTALLOWSCRIPTS\nnpm error something new\n"),
        None
    );
}

#[test]
fn the_npm_source_reports_a_strict_refusal_as_blocked() {
    let npm = Npm {
        command: PathBuf::from("npm"),
    };

    assert_eq!(
        npm.blocked_scripts(STRICT_REFUSAL).map(|found| found.len()),
        Some(5)
    );
}

#[test]
#[ignore = "runs the real npm config get/set against a temporary userconfig: cargo test -- --ignored --exact source::npm::tests::approving_scripts_appends_pinned_entries_to_the_user_npmrc"]
fn approving_scripts_appends_pinned_entries_to_the_user_npmrc() {
    let userconfig = std::env::temp_dir().join("globlin-npmrc-approval-test");
    std::fs::write(&userconfig, "allow-scripts=@stripe/cli\n").unwrap();
    std::env::set_var("npm_config_userconfig", &userconfig);
    let npm = Npm::new(None).unwrap();

    npm.approve_scripts(&parse_blocked_scripts(STRICT_REFUSAL).unwrap())
        .unwrap();

    let written = std::fs::read_to_string(&userconfig).unwrap();
    std::fs::remove_file(&userconfig).ok();
    assert!(written.contains(
        "allow-scripts=@stripe/cli,@deepseek-ai/dsh-subprocess-local@0.1.7-rc.2,koffi@3.3.2,node-pty@1.2.0-beta.15,@google/genai@1.52.0,protobufjs@7.6.6"
    ));
}

#[test]
fn the_npm_uninstall_command_removes_the_package_globally() {
    let npm = Npm {
        command: PathBuf::from("npm"),
    };

    assert_eq!(
        arguments(&npm.uninstall_command("@salesforce/cli").unwrap()),
        vec!["uninstall", "-g", "@salesforce/cli"]
    );
}
