use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use ureq::Agent;

use crate::Result;

const REGISTRY: &str = "https://registry.npmjs.org";
const FEED: &str = "https://pypi.org/rss/project";
const CRATES: &str = "https://crates.io/api/v1/crates";
const NUGET: &str = "https://api.nuget.org/v3-flatcontainer";
const GALLERY: &str = "https://www.powershellgallery.com/api/v2/package";
const NPM_HOST: &str = "registry.npmjs.org";
const PYPI_HOST: &str = "pypi.org";
const CRATES_HOST: &str = "crates.io";
const NUGET_HOST: &str = "api.nuget.org";
const GALLERY_HOST: &str = "www.powershellgallery.com";
const BATCH: usize = 100;
const USER_AGENT: &str = concat!(
    "globlin/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/TOR968/Globlin)"
);
const WORKERS: usize = 6;
const TIMEOUT: Duration = Duration::from_secs(10);

pub fn npm_latest(names: &[String]) -> Result<HashMap<String, Version>> {
    fetch_all(names, NPM_HOST, npm_dist_tag)
}

pub fn pypi_latest(names: &[String]) -> Result<HashMap<String, String>> {
    fetch_all(names, PYPI_HOST, pypi_newest_release)
}

pub fn crates_latest(names: &[String]) -> Result<HashMap<String, Version>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let client = Client::new();
    let found = names
        .chunks(BATCH)
        .filter_map(|chunk| crates_page(&client, chunk))
        .flatten()
        .collect();
    client.verdict(CRATES_HOST, found)
}

pub fn nuget_latest(names: &[String]) -> Result<HashMap<String, String>> {
    fetch_all(names, NUGET_HOST, nuget_newest_release)
}

pub fn psgallery_latest(names: &[String]) -> Result<HashMap<String, String>> {
    fetch_all(names, GALLERY_HOST, gallery_newest_release)
}

pub fn numeric_is_newer(latest: &str, current: &str) -> Option<bool> {
    let mut latest = numeric(latest)?;
    let mut current = numeric(current)?;
    let width = latest.len().max(current.len());
    latest.resize(width, 0);
    current.resize(width, 0);
    Some(latest > current)
}

pub fn is_newer(latest: &str, current: &str) -> Option<bool> {
    let semantic = |raw: &str| Version::parse(raw.trim_start_matches('v')).ok();
    match (semantic(latest), semantic(current)) {
        (Some(latest), Some(current)) => Some(latest > current),
        _ => numeric_is_newer(
            latest.trim_start_matches('v'),
            current.trim_start_matches('v'),
        ),
    }
}

struct Client {
    agent: Agent,
    reached: AtomicBool,
}

impl Client {
    fn new() -> Self {
        Self {
            agent: Agent::config_builder()
                .timeout_global(Some(TIMEOUT))
                .user_agent(USER_AGENT)
                .build()
                .into(),
            reached: AtomicBool::new(false),
        }
    }

    fn answered<T>(&self, reply: std::result::Result<T, ureq::Error>) -> Option<T> {
        match reply {
            Ok(value) => {
                self.reached.store(true, Ordering::Relaxed);
                Some(value)
            }
            Err(ureq::Error::StatusCode(_)) => {
                self.reached.store(true, Ordering::Relaxed);
                None
            }
            Err(_) => None,
        }
    }

    fn verdict<T>(&self, host: &str, found: HashMap<String, T>) -> Result<HashMap<String, T>> {
        if found.is_empty() && !self.reached.load(Ordering::Relaxed) {
            return Err(format!("{host} could not be reached").into());
        }
        Ok(found)
    }
}

fn fetch_all<T: Send>(
    names: &[String],
    host: &str,
    fetch: impl Fn(&Client, &str) -> Option<T> + Sync,
) -> Result<HashMap<String, T>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let client = Client::new();
    let per_worker = names.len().div_ceil(WORKERS);
    let found = std::thread::scope(|scope| {
        let workers: Vec<_> = names
            .chunks(per_worker)
            .map(|chunk| scope.spawn(|| fetch_chunk(&client, chunk, &fetch)))
            .collect();
        workers
            .into_iter()
            .filter_map(|worker| worker.join().ok())
            .flatten()
            .collect()
    });
    client.verdict(host, found)
}

fn fetch_chunk<T>(
    client: &Client,
    names: &[String],
    fetch: &impl Fn(&Client, &str) -> Option<T>,
) -> Vec<(String, T)> {
    names
        .iter()
        .filter_map(|name| Some((name.clone(), fetch(client, name)?)))
        .collect()
}

fn crates_page(client: &Client, names: &[String]) -> Option<Vec<(String, Version)>> {
    let page: CratesPage = serde_json::from_str(&body(client, &crates_url(names))?).ok()?;
    Some(
        page.crates
            .into_iter()
            .filter_map(|entry| {
                Some((entry.name, Version::parse(&entry.max_stable_version?).ok()?))
            })
            .collect(),
    )
}

fn crates_url(names: &[String]) -> String {
    let ids: Vec<String> = names
        .iter()
        .map(|name| format!("ids%5B%5D={}", encoded(name)))
        .collect();
    format!("{CRATES}?per_page={BATCH}&{}", ids.join("&"))
}

fn nuget_newest_release(client: &Client, name: &str) -> Option<String> {
    let index: NuGetIndex = serde_json::from_str(&body(client, &nuget_url(name))?).ok()?;
    index
        .versions
        .into_iter()
        .rfind(|release| numeric(release).is_some())
}

fn gallery_newest_release(client: &Client, name: &str) -> Option<String> {
    let response = client.answered(
        client
            .agent
            .get(format!("{GALLERY}/{}", encoded(name)))
            .config()
            .max_redirects(0)
            .build()
            .call(),
    )?;
    let location = response.headers().get("location")?.to_str().ok()?;
    version_in_package_url(location, name)
}

fn version_in_package_url(location: &str, name: &str) -> Option<String> {
    let file = location.rsplit('/').next()?.strip_suffix(".nupkg")?;
    let prefix = format!("{}.", name.to_lowercase());
    let version = file.to_lowercase().strip_prefix(&prefix)?.to_string();
    numeric(&version).map(|_| version)
}

fn nuget_url(name: &str) -> String {
    format!("{NUGET}/{}/index.json", encoded(&name.to_lowercase()))
}

fn npm_dist_tag(client: &Client, name: &str) -> Option<Version> {
    let tags: DistTags = serde_json::from_str(&body(client, &dist_tags_url(name))?).ok()?;
    Version::parse(&tags.latest).ok()
}

fn pypi_newest_release(client: &Client, name: &str) -> Option<String> {
    newest_stable(&body(client, &feed_url(name))?)
}

fn body(client: &Client, url: &str) -> Option<String> {
    client
        .answered(client.agent.get(url).call())?
        .body_mut()
        .read_to_string()
        .ok()
}

fn dist_tags_url(name: &str) -> String {
    format!("{REGISTRY}/-/package/{}/dist-tags", encoded(name))
}

fn feed_url(name: &str) -> String {
    format!("{FEED}/{}/releases.xml", encoded(name))
}

fn encoded(name: &str) -> String {
    use std::fmt::Write;

    name.bytes().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'@') {
            out.push(char::from(byte));
        } else {
            write!(out, "%{byte:02x}").ok();
        }
        out
    })
}

fn newest_stable(feed: &str) -> Option<String> {
    feed.split("<item>")
        .skip(1)
        .filter_map(title)
        .find(|release| numeric(release).is_some())
        .map(str::to_string)
}

fn title(item: &str) -> Option<&str> {
    let (_, rest) = item.split_once("<title>")?;
    let (title, _) = rest.split_once("</title>")?;
    Some(title.trim())
}

fn numeric(version: &str) -> Option<Vec<u64>> {
    version.split('.').map(|part| part.parse().ok()).collect()
}

#[derive(Deserialize)]
struct DistTags {
    latest: String,
}

#[derive(Deserialize)]
struct NuGetIndex {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct CratesPage {
    crates: Vec<CrateEntry>,
}

#[derive(Deserialize)]
struct CrateEntry {
    name: String,
    max_stable_version: Option<String>,
}

#[cfg(test)]
mod tests;
