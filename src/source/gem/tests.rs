use super::*;

const LISTING: &str = "
*** LOCAL GEMS ***

bundler (default: 2.5.11)
nokogiri (1.16.5 x64-mingw-ucrt, 1.15.0 x64-mingw-ucrt)
rake (13.2.1, 13.0.6)
";

const OUTDATED: &str = "rake (13.2.1 < 13.3.0)
nokogiri (1.16.5 < 1.18.2, 1.18.3)
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_newest_installed_version_of_each_gem_is_the_one_reported() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["bundler", "nokogiri", "rake"]);
    assert_eq!(installed[2].version, "13.2.1");
    assert_eq!(installed[2].source, SourceKind::Gem);
}

#[test]
fn a_default_gem_and_a_platform_label_are_not_part_of_the_version() {
    let installed = parse_listing(LISTING);

    assert_eq!(installed[0].version, "2.5.11");
    assert_eq!(installed[1].version, "1.16.5");
}

#[test]
fn the_local_gems_banner_is_not_a_gem() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|item| item.name.contains('*')));
}

#[test]
fn the_outdated_report_supplies_the_upgrade() {
    let installed = merge(LISTING, Some(OUTDATED));

    assert_eq!(installed[2].available.as_deref(), Some("13.3.0"));
}

#[test]
fn a_gem_absent_from_a_successful_outdated_report_resolves_to_its_own_version() {
    let installed = merge(LISTING, Some(OUTDATED));

    assert_eq!(installed[0].available.as_deref(), Some("2.5.11"));
}

#[test]
fn a_failed_outdated_report_leaves_every_gem_unresolved_rather_than_current() {
    assert!(merge(LISTING, None)
        .iter()
        .all(|item| item.available.is_none()));
}

#[test]
fn a_cooldown_candidate_list_offers_its_first_version() {
    let upgrades = parse_outdated(OUTDATED);

    assert_eq!(upgrades.get("nokogiri").map(String::as_str), Some("1.18.2"));
}

#[test]
fn an_outdated_row_naming_the_same_version_twice_is_not_an_upgrade() {
    assert!(parse_outdated("rake (13.2.1 < 13.2.1)\n").is_empty());
}

fn gem(user_install: bool) -> Gem {
    Gem {
        command: PathBuf::from("gem"),
        user_install: OnceLock::from(user_install),
    }
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("globlin-test-gem-{label}"));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_specification(home: &Path, folder: &[&str], stem: &str) {
    let directory = folder
        .iter()
        .fold(home.join("specifications"), |path, part| path.join(part));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(format!("{stem}.gemspec")), "").unwrap();
}

fn names(installed: &[Installed]) -> Vec<&str> {
    installed.iter().map(|item| item.name.as_str()).collect()
}

#[test]
fn a_gem_whose_newest_version_sits_in_a_directory_the_user_cannot_write_is_not_listed() {
    let home = scratch("owned");
    with_specification(&home, &[], "rake-13.2.1");

    let kept = owned(parse_listing(LISTING), &specifications(&[home]));

    assert_eq!(names(&kept), vec!["rake"]);
}

#[test]
fn a_default_gem_and_a_native_gem_are_found_by_their_specification_files() {
    let home = scratch("default-and-native");
    with_specification(&home, &["default"], "bundler-2.5.11");
    with_specification(&home, &[], "nokogiri-1.16.5-x64-mingw-ucrt");

    let kept = owned(parse_listing(LISTING), &specifications(&[home]));

    assert_eq!(names(&kept), vec!["bundler", "nokogiri"]);
}

#[test]
fn an_older_version_in_a_writable_directory_does_not_keep_a_gem_whose_newest_is_out_of_reach() {
    let home = scratch("older-version");
    with_specification(&home, &[], "rake-13.0.6");
    with_specification(&home, &[], "rake-13.2.10");

    assert!(owned(parse_listing(LISTING), &specifications(&[home])).is_empty());
}

#[test]
fn a_gem_home_that_does_not_exist_yet_owns_nothing() {
    let home = scratch("missing").join("ruby");

    assert!(specifications(&[home]).is_empty());
}

#[cfg(unix)]
#[test]
fn a_directory_the_user_owns_is_writable() {
    assert!(platform::writable(&scratch("writable")));
}

#[test]
fn an_update_goes_to_the_user_directory_when_the_gem_home_cannot_be_written() {
    assert_eq!(
        arguments(&gem(true).update_command("rake").unwrap()),
        vec!["update", "rake", "--user-install"]
    );
}

#[test]
fn the_gem_update_command_updates_the_named_gem() {
    let gem = gem(false);

    assert_eq!(
        arguments(&gem.update_command("rake").unwrap()),
        vec!["update", "rake"]
    );
}

#[test]
fn the_gem_uninstall_command_never_waits_on_a_prompt_nobody_can_see() {
    let gem = gem(false);

    assert_eq!(
        arguments(&gem.uninstall_command("rake").unwrap()),
        vec!["uninstall", "rake", "--all", "--executables"]
    );
}

#[test]
#[ignore = "runs the real gem: cargo test -- --ignored --exact source::gem::tests::the_real_gem_listing_still_parses"]
fn the_real_gem_listing_still_parses() {
    let installed = Gem::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
