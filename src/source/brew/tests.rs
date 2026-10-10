use super::*;

const INFO: &str = r#"{
  "formulae": [
    {
      "name": "wget",
      "versions": { "stable": "1.25.0", "head": null, "bottle": true },
      "revision": 0,
      "installed": [{ "version": "1.24.5", "installed_on_request": true }],
      "outdated": true,
      "pinned": false
    },
    {
      "name": "openssl@3",
      "versions": { "stable": "3.4.0", "head": null, "bottle": true },
      "revision": 1,
      "installed": [{ "version": "3.4.0" }],
      "outdated": true
    },
    {
      "name": "jq",
      "versions": { "stable": "1.7.1", "head": "HEAD", "bottle": true },
      "revision": 0,
      "installed": [{ "version": "1.7.1" }],
      "outdated": false
    },
    {
      "name": "ghost",
      "versions": { "stable": "1.0.0" },
      "installed": [],
      "outdated": false
    }
  ],
  "casks": [
    {
      "token": "firefox",
      "version": "131.0",
      "installed": "130.0.1",
      "outdated": true
    },
    {
      "token": "iterm2",
      "version": "3.5.4",
      "installed": "3.5.4",
      "outdated": false
    }
  ]
}"#;

fn listing() -> Vec<Installed> {
    parse_info(INFO.as_bytes()).unwrap()
}

fn find<'a>(listing: &'a [Installed], name: &str) -> &'a Installed {
    listing
        .iter()
        .find(|installed| installed.name == name)
        .unwrap_or_else(|| panic!("{name} is missing from {listing:?}"))
}

fn arguments(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn an_outdated_formula_offers_its_stable_version() {
    let listing = listing();
    let wget = find(&listing, "wget");

    assert_eq!(wget.version, "1.24.5");
    assert_eq!(wget.available.as_deref(), Some("1.25.0"));
    assert_eq!(wget.source, SourceKind::Brew);
}

#[test]
fn a_formula_revision_is_part_of_the_version_it_offers() {
    let listing = listing();
    let openssl = find(&listing, "openssl@3");

    assert_eq!(openssl.available.as_deref(), Some("3.4.0_1"));
}

#[test]
fn a_formula_brew_does_not_call_outdated_offers_the_installed_version() {
    let listing = listing();
    let jq = find(&listing, "jq");

    assert_eq!(jq.available.as_deref(), Some("1.7.1"));
}

#[test]
fn a_formula_without_an_installed_keg_is_not_listed() {
    assert!(!listing().iter().any(|installed| installed.name == "ghost"));
}

#[test]
fn an_outdated_cask_offers_its_current_version() {
    let listing = listing();
    let firefox = find(&listing, "firefox");

    assert_eq!(firefox.version, "130.0.1");
    assert_eq!(firefox.available.as_deref(), Some("131.0"));
}

#[test]
fn a_current_cask_offers_the_installed_version() {
    let listing = listing();
    let iterm = find(&listing, "iterm2");

    assert_eq!(iterm.available.as_deref(), Some("3.5.4"));
}

#[test]
fn a_reply_with_no_casks_section_still_parses() {
    let listing = parse_info(br#"{"formulae": []}"#).unwrap();

    assert!(listing.is_empty());
}

#[test]
fn a_reply_that_is_not_json_is_an_error() {
    assert!(parse_info(b"Error: no such keg").is_err());
}

#[test]
fn updating_upgrades_the_named_formula_or_cask() {
    let brew = Brew {
        command: PathBuf::from("brew"),
    };
    let command = brew.update_command("wget").unwrap();

    assert_eq!(arguments(&command), ["upgrade", "wget"]);
}

#[test]
fn uninstalling_removes_the_named_formula_or_cask() {
    let brew = Brew {
        command: PathBuf::from("brew"),
    };
    let command = brew.uninstall_command("firefox").unwrap();

    assert_eq!(arguments(&command), ["uninstall", "firefox"]);
}

#[test]
#[ignore = "runs the real brew: cargo test -- --ignored --exact source::brew::tests::the_real_brew_listing_still_parses"]
fn the_real_brew_listing_still_parses() {
    let listing = Brew::new().unwrap().installed().unwrap();
    println!("{listing:#?}");
}
