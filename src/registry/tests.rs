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
