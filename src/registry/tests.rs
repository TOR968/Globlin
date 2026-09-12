use super::*;

const FEED_BODY: &str = r"<?xml version='1.0' encoding='UTF-8'?>
<rss version='2.0'>
  <channel>
    <title>PyPI recent updates for typing-extensions</title>
    <item>
      <title>4.16.0rc2</title>
      <link>https://pypi.org/project/typing-extensions/4.16.0rc2/</link>
    </item>
    <item>
      <title>4.15.0</title>
      <link>https://pypi.org/project/typing-extensions/4.15.0/</link>
    </item>
    <item>
      <title>4.14.1</title>
      <link>https://pypi.org/project/typing-extensions/4.14.1/</link>
    </item>
  </channel>
</rss>";

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
    assert!(npm_latest(&[]).unwrap().is_empty());
    assert!(pypi_latest(&[]).is_empty());
}

#[test]
fn a_project_is_asked_for_its_release_feed_not_its_full_metadata() {
    assert_eq!(
        feed_url("black"),
        "https://pypi.org/rss/project/black/releases.xml"
    );
}

#[test]
fn the_channel_title_is_not_mistaken_for_a_release() {
    assert_eq!(newest_stable(FEED_BODY), Some("4.15.0".to_string()));
}

#[test]
fn a_prerelease_at_the_top_of_the_feed_is_skipped() {
    assert_ne!(newest_stable(FEED_BODY).as_deref(), Some("4.16.0rc2"));
}

#[test]
fn a_feed_without_releases_yields_nothing() {
    assert_eq!(newest_stable(""), None);
    assert_eq!(newest_stable("<item><title>nightly</title></item>"), None);
}

#[test]
fn feed_versions_compare_numerically_not_as_strings() {
    assert_eq!(numeric_is_newer("24.10.0", "24.9.0"), Some(true));
    assert_eq!(numeric_is_newer("24.9.0", "24.10.0"), Some(false));
}

#[test]
fn a_calendar_version_compares_like_any_other_number_run() {
    assert_eq!(numeric_is_newer("2025.9.5", "2024.12.13"), Some(true));
}

#[test]
fn an_equal_version_is_not_newer() {
    assert_eq!(numeric_is_newer("1.8.4", "1.8.4"), Some(false));
}

#[test]
fn a_shorter_version_is_not_newer_than_the_same_one_with_a_trailing_zero() {
    assert_eq!(numeric_is_newer("1.8", "1.8.0"), Some(false));
}

#[test]
fn a_version_that_is_not_a_run_of_numbers_cannot_be_compared() {
    assert_eq!(numeric_is_newer("1.0.0", "1.0.0rc1"), None);
    assert_eq!(numeric_is_newer("1.0.0.post1", "1.0.0"), None);
}

#[test]
#[ignore = "hits the real PyPI feed: cargo test -- --ignored --exact registry::tests::the_real_pypi_feed_still_answers_with_a_version"]
fn the_real_pypi_feed_still_answers_with_a_version() {
    let found = pypi_latest(&["black".to_string()]);

    assert!(numeric(&found["black"]).is_some());
}

#[test]
fn crate_ids_are_sent_as_a_repeated_array_parameter() {
    let names = vec!["ripgrep".to_string(), "cargo-edit".to_string()];

    assert_eq!(
        crates_url(&names),
        "https://crates.io/api/v1/crates?per_page=100&ids%5B%5D=ripgrep&ids%5B%5D=cargo-edit"
    );
}

#[test]
fn a_crate_page_is_read_by_name_not_by_position() {
    let body = r#"{"crates":[{"name":"ripgrep","max_stable_version":"14.1.0"},
                             {"name":"cargo-edit","max_stable_version":"0.13.6"}]}"#;
    let page: CratesPage = serde_json::from_str(body).unwrap();
    let names: Vec<&str> = page
        .crates
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();

    assert_eq!(names, vec!["ripgrep", "cargo-edit"]);
    assert_eq!(page.crates[1].max_stable_version.as_deref(), Some("0.13.6"));
}

#[test]
fn a_crate_with_only_prereleases_has_no_stable_version_to_offer() {
    let body = r#"{"crates":[{"name":"early","max_stable_version":null}]}"#;
    let page: CratesPage = serde_json::from_str(body).unwrap();

    assert_eq!(page.crates[0].max_stable_version, None);
}

#[test]
fn an_empty_crates_request_needs_no_network() {
    assert!(crates_latest(&[]).is_empty());
}

#[test]
fn the_agent_identifies_globlin_because_crates_io_rejects_anonymous_callers() {
    assert!(USER_AGENT.starts_with("globlin/"));
    assert!(USER_AGENT.contains("github.com/TOR968/Globlin"));
}

#[test]
fn a_nuget_id_is_lowercased_because_the_flat_container_only_answers_that_way() {
    assert_eq!(
        nuget_url("DotNetSay"),
        "https://api.nuget.org/v3-flatcontainer/dotnetsay/index.json"
    );
}

#[test]
fn the_nuget_index_is_read_from_the_end_because_it_is_sorted_oldest_first() {
    let index: NuGetIndex =
        serde_json::from_str(r#"{"versions":["1.0.0","2.1.4","9.0.0-preview.1"]}"#).unwrap();
    let newest = index
        .versions
        .into_iter()
        .rfind(|release| numeric(release).is_some());

    assert_eq!(newest.as_deref(), Some("2.1.4"));
}

#[test]
fn an_empty_nuget_request_needs_no_network() {
    assert!(nuget_latest(&[]).is_empty());
}

#[test]
fn the_gallery_redirect_names_the_newest_release_in_its_file_name() {
    assert_eq!(
        version_in_package_url(
            "https://cdn.powershellgallery.com/packages/pester.6.2.0.nupkg",
            "Pester"
        )
        .as_deref(),
        Some("6.2.0")
    );
}

#[test]
fn a_dotted_module_name_is_stripped_whole_not_at_its_first_dot() {
    assert_eq!(
        version_in_package_url(
            "https://cdn.powershellgallery.com/packages/az.accounts.3.0.4.nupkg",
            "Az.Accounts"
        )
        .as_deref(),
        Some("3.0.4")
    );
}

#[test]
fn a_prerelease_or_foreign_redirect_is_not_a_version() {
    assert_eq!(
        version_in_package_url(
            "https://cdn.powershellgallery.com/packages/pester.6.0.0-alpha1.nupkg",
            "Pester"
        ),
        None
    );
    assert_eq!(
        version_in_package_url("https://example.test/login", "Pester"),
        None
    );
}

#[test]
fn an_empty_gallery_request_needs_no_network() {
    assert!(psgallery_latest(&[]).is_empty());
}

#[test]
#[ignore = "hits the real NuGet index: cargo test -- --ignored --exact registry::tests::the_real_nuget_index_still_answers_with_a_version"]
fn the_real_nuget_index_still_answers_with_a_version() {
    let found = nuget_latest(&["dotnetsay".to_string()]);

    assert!(numeric(&found["dotnetsay"]).is_some());
}

#[test]
#[ignore = "hits the real PowerShell Gallery: cargo test -- --ignored --exact registry::tests::the_real_gallery_redirect_still_names_a_version"]
fn the_real_gallery_redirect_still_names_a_version() {
    let found = psgallery_latest(&["Pester".to_string()]);

    assert!(numeric(&found["Pester"]).is_some());
}

#[test]
#[ignore = "hits the real crates.io: cargo test -- --ignored --exact registry::tests::the_real_crates_io_still_answers_for_a_batch"]
fn the_real_crates_io_still_answers_for_a_batch() {
    let found = crates_latest(&["ripgrep".to_string(), "serde".to_string()]);

    assert_eq!(found.len(), 2);
}
