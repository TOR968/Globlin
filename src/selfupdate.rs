use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::Result;

pub const EXE_ASSET: &str = "npm-globals-tray.exe";
pub const SHA_ASSET: &str = "npm-globals-tray.exe.sha256";

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
    hash.iter()
        .fold(String::with_capacity(64), |mut s, byte| {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
