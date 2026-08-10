use std::collections::HashMap;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use ureq::Agent;

use crate::Result;

const REGISTRY: &str = "https://registry.npmjs.org";
const WORKERS: usize = 6;
const TIMEOUT: Duration = Duration::from_secs(10);

pub fn latest_versions(names: &[String]) -> Result<HashMap<String, Version>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let found = fetch_all(&agent(), names);
    if found.is_empty() {
        return Err(format!(
            "the registry returned no versions for any of the {} packages checked",
            names.len()
        )
        .into());
    }
    Ok(found)
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

fn fetch_all(agent: &Agent, names: &[String]) -> HashMap<String, Version> {
    let per_worker = names.len().div_ceil(WORKERS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = names
            .chunks(per_worker)
            .map(|chunk| scope.spawn(|| fetch_chunk(agent, chunk)))
            .collect();
        workers
            .into_iter()
            .filter_map(|worker| worker.join().ok())
            .flatten()
            .collect()
    })
}

fn fetch_chunk(agent: &Agent, names: &[String]) -> Vec<(String, Version)> {
    names
        .iter()
        .filter_map(|name| Some((name.clone(), fetch_latest(agent, name)?)))
        .collect()
}

fn fetch_latest(agent: &Agent, name: &str) -> Option<Version> {
    let body = agent
        .get(dist_tags_url(name))
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    let tags: DistTags = serde_json::from_str(&body).ok()?;
    Version::parse(&tags.latest).ok()
}

fn dist_tags_url(name: &str) -> String {
    format!(
        "{REGISTRY}/-/package/{}/dist-tags",
        name.replace('/', "%2f")
    )
}

#[derive(Deserialize)]
struct DistTags {
    latest: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_names_have_their_slash_encoded() {
        assert_eq!(
            dist_tags_url("@salesforce/cli"),
            "https://registry.npmjs.org/-/package/@salesforce%2fcli/dist-tags"
        );
    }

    #[test]
    fn plain_names_are_passed_through() {
        assert_eq!(
            dist_tags_url("prettier"),
            "https://registry.npmjs.org/-/package/prettier/dist-tags"
        );
    }

    #[test]
    fn only_the_latest_tag_is_read() {
        let body = r#"{"beta":"2.0.0-beta.74","latest":"2.146.3","nightly":"2.148.1"}"#;
        let tags: DistTags = serde_json::from_str(body).unwrap();
        assert_eq!(tags.latest, "2.146.3");
    }

    #[test]
    fn an_empty_request_needs_no_network() {
        assert!(latest_versions(&[]).unwrap().is_empty());
    }
}
