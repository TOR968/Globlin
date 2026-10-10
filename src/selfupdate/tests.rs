use super::*;

const X64_SHA: &str = "globlin.exe.sha256";
const ARM64_SHA: &str = "globlin-arm64.exe.sha256";

#[test]
fn the_release_endpoint_points_at_this_repository() {
    assert_eq!(
        LATEST_URL,
        "https://api.github.com/repos/TOR968/globlin/releases/latest"
    );
}

#[test]
fn the_running_version_parses_as_semver() {
    assert!(Version::parse(env!("CARGO_PKG_VERSION")).is_ok());
}

#[test]
#[ignore = "hits the network: cargo test -- --ignored --exact selfupdate::tests::the_real_repository_answers_the_release_lookup"]
fn the_real_repository_answers_the_release_lookup() {
    let outcome = latest().unwrap();
    println!("latest(): {outcome:?}");
}

#[test]
#[ignore = "downloads and installs over the running exe: cargo test -- --ignored --exact selfupdate::tests::installs_the_published_build_for_real"]
fn installs_the_published_build_for_real() {
    let release = latest().unwrap().expect("no newer release is published");
    let version = apply(&release).unwrap();
    println!("installed {version}");
}

#[test]
fn a_release_that_already_failed_is_not_auto_applied() {
    let release = Version::parse("0.2.0").unwrap();
    assert!(!should_auto_apply(&release, true, Some(&release)));
}

#[test]
fn a_newer_release_is_auto_applied_after_an_older_one_failed() {
    let blocked = Version::parse("0.2.0").unwrap();
    let release = Version::parse("0.3.0").unwrap();
    assert!(should_auto_apply(&release, true, Some(&blocked)));
}

#[test]
fn nothing_is_auto_applied_while_auto_update_is_off() {
    let release = Version::parse("0.2.0").unwrap();
    assert!(!should_auto_apply(&release, false, None));
}

#[test]
fn an_installed_but_unlaunched_version_is_not_offered_again() {
    let installed = Version::parse("0.2.0").unwrap();
    assert!(!supersedes(&installed, Some(&installed)));
}

#[test]
fn a_release_newer_than_the_installed_one_is_still_offered() {
    let installed = Version::parse("0.2.0").unwrap();
    let release = Version::parse("0.3.0").unwrap();
    assert!(supersedes(&release, Some(&installed)));
}

fn body(tag: &str, assets: &[&str]) -> String {
    let assets: Vec<String> = assets
        .iter()
        .map(|name| {
            format!(r#"{{"name":"{name}","browser_download_url":"https://example.test/{name}"}}"#)
        })
        .collect();
    format!(r#"{{"tag_name":"{tag}","assets":[{}]}}"#, assets.join(","))
}

fn current() -> Version {
    Version::parse("0.1.1").unwrap()
}

#[test]
fn a_newer_tag_with_both_assets_is_offered() {
    let release = offer(
        &body("v0.2.0", &[X64_ASSET, X64_SHA]),
        &current(),
        X64_ASSET,
    )
    .unwrap();
    assert_eq!(release.version, Version::parse("0.2.0").unwrap());
    assert_eq!(release.exe_url, format!("https://example.test/{X64_ASSET}"));
    assert_eq!(release.sha_url, format!("https://example.test/{X64_SHA}"));
}

#[test]
fn a_tag_without_the_v_prefix_is_still_read() {
    let release = offer(&body("0.2.0", &[X64_ASSET, X64_SHA]), &current(), X64_ASSET).unwrap();
    assert_eq!(release.version, Version::parse("0.2.0").unwrap());
}

#[test]
fn the_running_version_is_not_offered_to_itself() {
    assert!(offer(
        &body("v0.1.1", &[X64_ASSET, X64_SHA]),
        &current(),
        X64_ASSET
    )
    .is_none());
}

#[test]
fn an_older_release_is_not_offered() {
    assert!(offer(
        &body("v0.1.0", &[X64_ASSET, X64_SHA]),
        &current(),
        X64_ASSET
    )
    .is_none());
}

#[test]
fn a_release_without_an_exe_asset_is_not_offered() {
    assert!(offer(&body("v0.2.0", &[X64_SHA]), &current(), X64_ASSET).is_none());
}

#[test]
fn a_release_without_a_checksum_asset_is_not_offered() {
    assert!(offer(&body("v0.2.0", &[X64_ASSET]), &current(), X64_ASSET).is_none());
}

#[test]
fn an_unparsable_tag_is_not_offered() {
    assert!(offer(
        &body("nightly", &[X64_ASSET, X64_SHA]),
        &current(),
        X64_ASSET
    )
    .is_none());
}

#[test]
fn a_body_that_is_not_a_release_is_not_offered() {
    assert!(offer(r#"{"message":"Not Found"}"#, &current(), X64_ASSET).is_none());
}

#[test]
fn the_published_hash_is_the_first_field_of_the_checksum_line() {
    let body = format!("{}  {X64_ASSET}\n", "a".repeat(64));
    assert_eq!(published_hash(&body, X64_ASSET).unwrap(), "a".repeat(64));
}

#[test]
fn a_checksum_line_is_read_despite_stray_whitespace() {
    let body = format!("  {}   {X64_ASSET}  \r\n", "b".repeat(64));
    assert_eq!(published_hash(&body, X64_ASSET).unwrap(), "b".repeat(64));
}

#[test]
fn an_uppercase_published_hash_is_lowercased() {
    let body = format!("{}  {X64_ASSET}\n", "A".repeat(64));
    assert_eq!(published_hash(&body, X64_ASSET).unwrap(), "a".repeat(64));
}

#[test]
fn the_published_hash_is_picked_out_of_a_multi_line_checksum_body() {
    let body = format!(
        "{}  other-asset.zip\n{}  {X64_ASSET}\n{}  another-asset.tar.gz\n",
        "c".repeat(64),
        "d".repeat(64),
        "e".repeat(64),
    );
    assert_eq!(published_hash(&body, X64_ASSET).unwrap(), "d".repeat(64));
}

#[test]
fn a_checksum_body_that_is_not_a_hash_is_rejected() {
    assert!(published_hash("not found\n", X64_ASSET).is_none());
    assert!(published_hash("", X64_ASSET).is_none());
    assert!(published_hash(&format!("{}  file\n", "z".repeat(64)), X64_ASSET).is_none());
}

#[test]
fn the_digest_matches_a_known_vector() {
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn matching_bytes_verify() {
    let body = format!("{}  {X64_ASSET}\n", digest(b"abc"));
    assert!(verify(b"abc", &body, X64_ASSET).is_ok());
}

#[test]
fn bytes_that_do_not_match_the_published_hash_are_refused() {
    let body = format!("{}  {X64_ASSET}\n", digest(b"abc"));
    assert!(verify(b"abd", &body, X64_ASSET).is_err());
}

#[test]
fn a_missing_published_hash_is_refused() {
    assert!(verify(b"abc", "404: Not Found", X64_ASSET).is_err());
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("globlin-test-{label}"));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_staged_and_previous_files_sit_next_to_the_executable() {
    let current = PathBuf::from(r"C:\tools\globlin.exe");
    assert_eq!(
        staged_path(&current),
        PathBuf::from(r"C:\tools\globlin.exe.new")
    );
    assert_eq!(
        previous_path(&current),
        PathBuf::from(r"C:\tools\globlin.exe.old")
    );
}

#[test]
fn a_swap_moves_the_staged_file_into_place_and_keeps_the_old_one() {
    let dir = scratch("swap");
    let current = dir.join("globlin.exe");
    let staged = staged_path(&current);
    fs::write(&current, b"old build").unwrap();
    fs::write(&staged, b"new build").unwrap();

    swap(&current, &staged).unwrap();

    assert_eq!(fs::read(&current).unwrap(), b"new build");
    assert_eq!(fs::read(previous_path(&current)).unwrap(), b"old build");
    assert!(!staged.exists());
}

#[test]
fn a_swap_that_cannot_finish_puts_the_original_back() {
    let dir = scratch("rollback");
    let current = dir.join("globlin.exe");
    let staged = staged_path(&current);
    fs::write(&current, b"old build").unwrap();

    assert!(swap(&current, &staged).is_err());

    assert_eq!(fs::read(&current).unwrap(), b"old build");
    assert!(!previous_path(&current).exists());
}

#[test]
fn a_swap_that_cannot_activate_the_new_file_returns_the_activation_error_when_rollback_succeeds() {
    let dir = scratch("activation-error");
    let current = dir.join("globlin.exe");
    let staged = staged_path(&current);
    fs::write(&current, b"old build").unwrap();

    let err = swap(&current, &staged).unwrap_err();
    let msg = err.to_string().to_lowercase();

    assert!(!msg.contains("rollback"));
    assert_eq!(fs::read(&current).unwrap(), b"old build");
    assert!(!previous_path(&current).exists());
}

#[test]
fn the_staged_file_is_kept_when_the_live_executable_is_missing() {
    let dir = scratch("discard-missing");
    let current = dir.join("globlin.exe");

    assert!(!should_discard_staged(&current));
}

#[test]
fn the_staged_file_is_discarded_when_the_live_executable_is_present() {
    let dir = scratch("discard-present");
    let current = dir.join("globlin.exe");
    fs::write(&current, b"old build").unwrap();

    assert!(should_discard_staged(&current));
}

#[test]
fn clean_stale_removes_both_the_previous_and_the_staged_build() {
    let dir = scratch("clean-stale");
    let current = dir.join("globlin.exe");
    fs::write(&current, b"current build").unwrap();
    fs::write(previous_path(&current), b"old build").unwrap();
    fs::write(staged_path(&current), b"new build").unwrap();

    remove_leftovers(&current);

    assert!(current.exists());
    assert!(!previous_path(&current).exists());
    assert!(!staged_path(&current).exists());
}

#[test]
fn a_swap_replaces_a_leftover_previous_build() {
    let dir = scratch("leftover");
    let current = dir.join("globlin.exe");
    let staged = staged_path(&current);
    fs::write(&current, b"old build").unwrap();
    fs::write(&staged, b"new build").unwrap();
    fs::write(previous_path(&current), b"ancient build").unwrap();

    swap(&current, &staged).unwrap();

    assert_eq!(fs::read(&current).unwrap(), b"new build");
    assert_eq!(fs::read(previous_path(&current)).unwrap(), b"old build");
}

#[test]
fn an_x64_build_updates_from_the_asset_every_released_version_already_reads() {
    assert_eq!(asset_for("windows", "x86_64"), Some("globlin.exe"));
}

#[test]
fn an_arm64_build_updates_from_its_own_asset() {
    assert_eq!(asset_for("windows", "aarch64"), Some("globlin-arm64.exe"));
}

#[test]
fn an_architecture_without_a_published_build_gets_no_asset() {
    assert_eq!(asset_for("windows", "x86"), None);
    assert_eq!(asset_for("linux", "x86_64"), None);
    assert_eq!(asset_for("macos", "x86_64"), None);
}

#[test]
fn the_checksum_asset_is_named_after_its_binary() {
    assert_eq!(checksum_asset(ARM64_ASSET), ARM64_SHA);
}

#[test]
fn an_arm64_release_is_offered_only_when_its_own_assets_are_published() {
    assert!(offer(
        &body("v0.2.0", &[X64_ASSET, X64_SHA]),
        &current(),
        ARM64_ASSET
    )
    .is_none());

    let release = offer(
        &body("v0.2.0", &[X64_ASSET, X64_SHA, ARM64_ASSET, ARM64_SHA]),
        &current(),
        ARM64_ASSET,
    )
    .unwrap();

    assert_eq!(
        release.exe_url,
        format!("https://example.test/{ARM64_ASSET}")
    );
    assert_eq!(release.sha_url, format!("https://example.test/{ARM64_SHA}"));
}

#[test]
fn the_checksum_line_for_the_other_architecture_is_ignored() {
    let body = format!(
        "{}  {X64_ASSET}
{}  {ARM64_ASSET}
",
        "a".repeat(64),
        "b".repeat(64)
    );

    assert_eq!(published_hash(&body, ARM64_ASSET).unwrap(), "b".repeat(64));
    assert_eq!(published_hash(&body, X64_ASSET).unwrap(), "a".repeat(64));
}

#[test]
fn an_apple_silicon_build_updates_from_the_bare_macos_binary() {
    assert_eq!(asset_for("macos", "aarch64"), Some("globlin-macos-arm64"));
}
