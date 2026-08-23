use super::*;

#[test]
fn defaults_ignore_the_self_updating_packages() {
    let config = Config::default();
    assert!(config.is_ignored("npm"));
    assert!(config.is_ignored("@anthropic-ai/claude-code"));
    assert!(!config.is_ignored("prettier"));
}

#[test]
fn round_trips_through_json() {
    let config = Config {
        check_interval_hours: 12,
        last_notified: vec!["npm:prettier@3.10.0".to_string()],
        sources: Sources {
            npm: true,
            bun: false,
        },
        ..Default::default()
    };

    let body = serde_json::to_string(&config).unwrap();
    let restored: Config = serde_json::from_str(&body).unwrap();

    assert_eq!(restored.check_interval_hours, 12);
    assert_eq!(restored.last_notified, config.last_notified);
    assert!(!restored.sources.bun);
    assert!(restored.sources.npm);
}

#[test]
fn missing_fields_fall_back_to_defaults() {
    let restored: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(restored.check_interval_hours, 6);
    assert!(restored.sources.npm);
    assert!(restored.sources.bun);
}

#[test]
fn an_explicit_empty_ignore_list_is_respected() {
    let restored: Config = serde_json::from_str(r#"{"ignore":[]}"#).unwrap();
    assert!(!restored.is_ignored("npm"));
}

#[test]
fn interval_never_collapses_to_zero() {
    let config = Config {
        check_interval_hours: 0,
        ..Default::default()
    };
    assert_eq!(config.interval(), Duration::from_hours(1));
}

#[test]
fn ignoring_the_same_package_twice_does_not_duplicate_the_entry() {
    let mut config = Config::default();
    config.set_ignored("prettier", true);
    config.set_ignored("prettier", true);

    assert_eq!(
        config
            .ignore
            .iter()
            .filter(|name| *name == "prettier")
            .count(),
        1
    );
    assert!(config.is_ignored("prettier"));
}

#[test]
fn un_ignoring_a_package_that_was_never_ignored_changes_nothing() {
    let mut config = Config::default();
    let before = config.ignore.clone();
    config.set_ignored("prettier", false);

    assert_eq!(config.ignore, before);
}

#[test]
fn un_ignoring_removes_only_the_named_package() {
    let mut config = Config::default();
    config.set_ignored("npm", false);

    assert!(!config.is_ignored("npm"));
    assert!(config.is_ignored("@anthropic-ai/claude-code"));
}

#[test]
fn self_updating_is_off_until_it_is_asked_for() {
    let config = Config::default();
    assert!(!config.auto_update);
    assert_eq!(config.last_self_notice, None);
}

#[test]
fn an_older_config_file_without_the_new_fields_still_loads() {
    let raw = r#"{"check_interval_hours":3,"ignore":["npm"]}"#;
    let config: Config = serde_json::from_str(raw).unwrap();
    assert_eq!(config.check_interval_hours, 3);
    assert!(!config.auto_update);
}
