use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ureq::Agent;

use crate::Result;

pub const EXE_ASSET: &str = "globlin.exe";
pub const SHA_ASSET: &str = "globlin.exe.sha256";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub exe_url: String,
    pub sha_url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

fn offer(body: &str, current: &Version) -> Option<Release> {
    let release: ApiRelease = serde_json::from_str(body).ok()?;
    let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
    if version <= *current {
        return None;
    }
    Some(Release {
        version,
        exe_url: asset_url(&release.assets, EXE_ASSET)?,
        sha_url: asset_url(&release.assets, SHA_ASSET)?,
    })
}

pub fn should_auto_apply(release: &Version, auto_update: bool, blocked: Option<&Version>) -> bool {
    auto_update && blocked != Some(release)
}

pub fn supersedes(release: &Version, installed: Option<&Version>) -> bool {
    installed.is_none_or(|installed| release > installed)
}

fn asset_url(assets: &[ApiAsset], name: &str) -> Option<String> {
    assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.clone())
}

fn published_hash(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let (hash, name) = line.trim().split_once(char::is_whitespace)?;
        if name.trim() != EXE_ASSET || hash.len() != 64 {
            return None;
        }
        if !hash.chars().all(|character| character.is_ascii_hexdigit()) {
            return None;
        }
        Some(hash.to_ascii_lowercase())
    })
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    hash.iter().fold(String::with_capacity(64), |mut s, byte| {
        use std::fmt::Write;
        write!(s, "{byte:02x}").unwrap();
        s
    })
}

fn verify(bytes: &[u8], sha_body: &str) -> Result<()> {
    let Some(published) = published_hash(sha_body) else {
        return Err("the release did not publish a usable checksum".into());
    };
    let actual = digest(bytes);
    if actual == published {
        return Ok(());
    }
    Err(format!("checksum mismatch: expected {published}, got {actual}").into())
}

fn staged_path(current: &Path) -> PathBuf {
    sibling(current, "new")
}

fn previous_path(current: &Path) -> PathBuf {
    sibling(current, "old")
}

fn sibling(current: &Path, suffix: &str) -> PathBuf {
    let mut name = current.as_os_str().to_os_string();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

fn swap(current: &Path, staged: &Path) -> Result<()> {
    let previous = previous_path(current);
    fs::rename(current, &previous)?;
    match fs::rename(staged, current) {
        Ok(()) => Ok(()),
        Err(error) => {
            match fs::rename(&previous, current) {
                Ok(()) => Err(error.into()),
                Err(rollback_error) => Err(format!(
                    "failed to activate new executable: {}; rollback also failed: {}; {} is now missing, but the previous build is intact at {} and renaming it back to {} restores it",
                    error, rollback_error, current.display(), previous.display(), current.display()
                ).into())
            }
        }
    }
}

pub fn clean_stale() {
    if let Ok(current) = std::env::current_exe() {
        remove_leftovers(&current);
    }
}

fn remove_leftovers(current: &Path) {
    fs::remove_file(previous_path(current)).ok();
    fs::remove_file(staged_path(current)).ok();
}

const LATEST_URL: &str = "https://api.github.com/repos/TOR968/globlin/releases/latest";
const USER_AGENT: &str = "globlin";
const TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_mins(2);
pub const RESTART_FLAG: &str = "--replaced";

pub fn latest() -> Result<Option<Release>> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let body = get(&agent(TIMEOUT), LATEST_URL)?.read_to_string()?;
    Ok(offer(&body, &current))
}

pub fn apply(release: &Release) -> Result<Version> {
    clean_stale();
    let binary = get(&agent(DOWNLOAD_TIMEOUT), &release.exe_url)?.read_to_vec()?;
    let checksum = get(&agent(TIMEOUT), &release.sha_url)?.read_to_string()?;
    verify(&binary, &checksum)?;

    let current = std::env::current_exe()?;
    let staged = staged_path(&current);
    fs::write(&staged, &binary)?;
    match swap(&current, &staged) {
        Ok(()) => Ok(release.version.clone()),
        Err(error) => {
            if should_discard_staged(&current) {
                fs::remove_file(&staged).ok();
            }
            Err(error)
        }
    }
}

fn should_discard_staged(current: &Path) -> bool {
    current.exists()
}

pub fn relaunch() -> Result<()> {
    Command::new(std::env::current_exe()?)
        .arg(RESTART_FLAG)
        .spawn()?;
    Ok(())
}

fn agent(timeout: Duration) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

fn get(agent: &Agent, url: &str) -> Result<ureq::Body> {
    Ok(agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .into_body())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                format!(
                    r#"{{"name":"{name}","browser_download_url":"https://example.test/{name}"}}"#
                )
            })
            .collect();
        format!(r#"{{"tag_name":"{tag}","assets":[{}]}}"#, assets.join(","))
    }

    fn current() -> Version {
        Version::parse("0.1.1").unwrap()
    }

    #[test]
    fn a_newer_tag_with_both_assets_is_offered() {
        let release = offer(&body("v0.2.0", &[EXE_ASSET, SHA_ASSET]), &current()).unwrap();
        assert_eq!(release.version, Version::parse("0.2.0").unwrap());
        assert_eq!(release.exe_url, format!("https://example.test/{EXE_ASSET}"));
        assert_eq!(release.sha_url, format!("https://example.test/{SHA_ASSET}"));
    }

    #[test]
    fn a_tag_without_the_v_prefix_is_still_read() {
        let release = offer(&body("0.2.0", &[EXE_ASSET, SHA_ASSET]), &current()).unwrap();
        assert_eq!(release.version, Version::parse("0.2.0").unwrap());
    }

    #[test]
    fn the_running_version_is_not_offered_to_itself() {
        assert!(offer(&body("v0.1.1", &[EXE_ASSET, SHA_ASSET]), &current()).is_none());
    }

    #[test]
    fn an_older_release_is_not_offered() {
        assert!(offer(&body("v0.1.0", &[EXE_ASSET, SHA_ASSET]), &current()).is_none());
    }

    #[test]
    fn a_release_without_an_exe_asset_is_not_offered() {
        assert!(offer(&body("v0.2.0", &[SHA_ASSET]), &current()).is_none());
    }

    #[test]
    fn a_release_without_a_checksum_asset_is_not_offered() {
        assert!(offer(&body("v0.2.0", &[EXE_ASSET]), &current()).is_none());
    }

    #[test]
    fn an_unparsable_tag_is_not_offered() {
        assert!(offer(&body("nightly", &[EXE_ASSET, SHA_ASSET]), &current()).is_none());
    }

    #[test]
    fn a_body_that_is_not_a_release_is_not_offered() {
        assert!(offer(r#"{"message":"Not Found"}"#, &current()).is_none());
    }

    #[test]
    fn the_published_hash_is_the_first_field_of_the_checksum_line() {
        let body = format!("{}  {EXE_ASSET}\n", "a".repeat(64));
        assert_eq!(published_hash(&body).unwrap(), "a".repeat(64));
    }

    #[test]
    fn a_checksum_line_is_read_despite_stray_whitespace() {
        let body = format!("  {}   {EXE_ASSET}  \r\n", "b".repeat(64));
        assert_eq!(published_hash(&body).unwrap(), "b".repeat(64));
    }

    #[test]
    fn an_uppercase_published_hash_is_lowercased() {
        let body = format!("{}  {EXE_ASSET}\n", "A".repeat(64));
        assert_eq!(published_hash(&body).unwrap(), "a".repeat(64));
    }

    #[test]
    fn the_published_hash_is_picked_out_of_a_multi_line_checksum_body() {
        let body = format!(
            "{}  other-asset.zip\n{}  {EXE_ASSET}\n{}  another-asset.tar.gz\n",
            "c".repeat(64),
            "d".repeat(64),
            "e".repeat(64),
        );
        assert_eq!(published_hash(&body).unwrap(), "d".repeat(64));
    }

    #[test]
    fn a_checksum_body_that_is_not_a_hash_is_rejected() {
        assert!(published_hash("not found\n").is_none());
        assert!(published_hash("").is_none());
        assert!(published_hash(&format!("{}  file\n", "z".repeat(64))).is_none());
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
        let body = format!("{}  {EXE_ASSET}\n", digest(b"abc"));
        assert!(verify(b"abc", &body).is_ok());
    }

    #[test]
    fn bytes_that_do_not_match_the_published_hash_are_refused() {
        let body = format!("{}  {EXE_ASSET}\n", digest(b"abc"));
        assert!(verify(b"abd", &body).is_err());
    }

    #[test]
    fn a_missing_published_hash_is_refused() {
        assert!(verify(b"abc", "404: Not Found").is_err());
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
    fn a_swap_that_cannot_activate_the_new_file_returns_the_activation_error_when_rollback_succeeds(
    ) {
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
}
