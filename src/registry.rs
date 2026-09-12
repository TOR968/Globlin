use std::collections::HashMap;
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
const BATCH: usize = 100;
const USER_AGENT: &str = concat!(
    "globlin/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/TOR968/Globlin)"
);
const WORKERS: usize = 6;
const TIMEOUT: Duration = Duration::from_secs(10);

pub fn npm_latest(names: &[String]) -> Result<HashMap<String, Version>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let found = fetch_all(names, npm_dist_tag);
    if found.is_empty() {
        return Err(format!(
            "the registry returned no versions for any of the {} packages checked",
            names.len()
        )
        .into());
    }
    Ok(found)
}

pub fn pypi_latest(names: &[String]) -> HashMap<String, String> {
    if names.is_empty() {
        return HashMap::new();
    }
    fetch_all(names, pypi_newest_release)
}

pub fn crates_latest(names: &[String]) -> HashMap<String, Version> {
    if names.is_empty() {
        return HashMap::new();
    }
    let agent = agent();
    names
        .chunks(BATCH)
        .filter_map(|chunk| crates_page(&agent, chunk))
        .flatten()
        .collect()
}

pub fn nuget_latest(names: &[String]) -> HashMap<String, String> {
    if names.is_empty() {
        return HashMap::new();
    }
    fetch_all(names, nuget_newest_release)
}

pub fn psgallery_latest(names: &[String]) -> HashMap<String, String> {
    if names.is_empty() {
        return HashMap::new();
    }
    fetch_all(names, gallery_newest_release)
}

pub fn numeric_is_newer(latest: &str, current: &str) -> Option<bool> {
    Some(numeric(latest)? > numeric(current)?)
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn fetch_all<T: Send>(
    names: &[String],
    fetch: impl Fn(&Agent, &str) -> Option<T> + Sync,
) -> HashMap<String, T> {
    let agent = agent();
    let per_worker = names.len().div_ceil(WORKERS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = names
            .chunks(per_worker)
            .map(|chunk| scope.spawn(|| fetch_chunk(&agent, chunk, &fetch)))
            .collect();
        workers
            .into_iter()
            .filter_map(|worker| worker.join().ok())
            .flatten()
            .collect()
    })
}

fn fetch_chunk<T>(
    agent: &Agent,
    names: &[String],
    fetch: &impl Fn(&Agent, &str) -> Option<T>,
) -> Vec<(String, T)> {
    names
        .iter()
        .filter_map(|name| Some((name.clone(), fetch(agent, name)?)))
        .collect()
}

fn crates_page(agent: &Agent, names: &[String]) -> Option<Vec<(String, Version)>> {
    let page: CratesPage = serde_json::from_str(&body(agent, &crates_url(names))?).ok()?;
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
        .map(|name| format!("ids%5B%5D={name}"))
        .collect();
    format!("{CRATES}?per_page={BATCH}&{}", ids.join("&"))
}

fn nuget_newest_release(agent: &Agent, name: &str) -> Option<String> {
    let index: NuGetIndex = serde_json::from_str(&body(agent, &nuget_url(name))?).ok()?;
    index
        .versions
        .into_iter()
        .rfind(|release| numeric(release).is_some())
}

fn gallery_newest_release(agent: &Agent, name: &str) -> Option<String> {
    let response = agent
        .get(format!("{GALLERY}/{name}"))
        .config()
        .max_redirects(0)
        .build()
        .call()
        .ok()?;
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
    format!("{NUGET}/{}/index.json", name.to_lowercase())
}

fn npm_dist_tag(agent: &Agent, name: &str) -> Option<Version> {
    let tags: DistTags = serde_json::from_str(&body(agent, &dist_tags_url(name))?).ok()?;
    Version::parse(&tags.latest).ok()
}

fn pypi_newest_release(agent: &Agent, name: &str) -> Option<String> {
    newest_stable(&body(agent, &feed_url(name))?)
}

fn body(agent: &Agent, url: &str) -> Option<String> {
    agent.get(url).call().ok()?.body_mut().read_to_string().ok()
}

fn dist_tags_url(name: &str) -> String {
    format!(
        "{REGISTRY}/-/package/{}/dist-tags",
        name.replace('/', "%2f")
    )
}

fn feed_url(name: &str) -> String {
    format!("{FEED}/{name}/releases.xml")
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
