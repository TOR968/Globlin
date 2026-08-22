use semver::Version;
use serde::Deserialize;

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
}
