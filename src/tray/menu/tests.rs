use super::*;
use crate::model::{Status, UpdateTarget};
use semver::Version;

fn package(name: &str, source: SourceKind, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: "1.2.3".to_string(),
        source,
        status,
    }
}

fn behind(name: &str, latest: &str) -> Package {
    package(
        name,
        SourceKind::Npm,
        Status::Outdated {
            latest: latest.to_string(),
        },
    )
}

fn view<'a>(packages: &'a [Package], activity: Option<&'a Activity>, frame: u32) -> View<'a> {
    View {
        packages,
        activity,
        autostart: false,
        self_update: SelfUpdate::Own {
            release: None,
            auto_update: false,
            log: false,
        },
        pending_restart: None,
        frame,
        elapsed: Duration::ZERO,
    }
}

fn updating_view(activity: &Activity, elapsed: Duration) -> View<'_> {
    View {
        packages: &[],
        activity: Some(activity),
        autostart: false,
        self_update: SelfUpdate::Own {
            release: None,
            auto_update: false,
            log: false,
        },
        pending_restart: None,
        frame: 0,
        elapsed,
    }
}

fn updating(name: &str, index: usize, total: usize) -> Activity {
    let targets = (0..total)
        .map(|position| UpdateTarget {
            name: if position == index {
                name.to_string()
            } else {
                format!("filler-{position}")
            },
            source: SourceKind::Npm,
            from: "1.2.3".to_string(),
            to: "2.0.0".to_string(),
        })
        .collect();
    let mut batch = crate::model::Batch::new(targets);
    batch.start(index);
    Activity::Updating { batch }
}

fn top_level_ids(built: &Built) -> Vec<String> {
    built
        .menu
        .items()
        .iter()
        .map(|item| item.id().as_ref().to_string())
        .collect()
}

#[test]
fn fixed_menu_ids_map_to_their_actions() {
    assert_eq!(Action::from_key("quit"), Some(Action::Quit));
    assert_eq!(Action::from_key("check-now"), Some(Action::CheckNow));
    assert_eq!(Action::from_key("update-all"), Some(Action::UpdateAll));
    assert_eq!(Action::from_key("autostart"), Some(Action::ToggleAutostart));
    assert_eq!(Action::from_key("open-log"), Some(Action::OpenLog));
    assert_eq!(Action::from_key("open-window"), Some(Action::OpenWindow));
    assert_eq!(Action::from_key("window-ready"), Some(Action::WindowReady));
}

#[test]
fn an_update_id_round_trips_including_scoped_names() {
    for name in ["prettier", "@salesforce/cli"] {
        let target = behind(name, "2.0.0");
        assert_eq!(
            Action::from_key(&update_id(&target)),
            Some(Action::Update {
                name: name.to_string(),
                source: SourceKind::Npm
            })
        );
    }
}

#[test]
fn every_source_label_is_a_usable_id_segment() {
    for kind in crate::model::KINDS {
        let entry = package("thing", kind, Status::Current);
        assert_eq!(
            Action::from_key(&update_id(&entry)),
            Some(Action::Update {
                name: "thing".to_string(),
                source: kind
            })
        );
    }
}

#[test]
fn the_same_name_from_two_sources_gets_two_distinct_ids() {
    let from_npm = package("typescript", SourceKind::Npm, Status::Current);
    let via_pnpm = package("typescript", SourceKind::Pnpm, Status::Current);

    assert_ne!(update_id(&from_npm), update_id(&via_pnpm));
    assert_eq!(
        Action::from_key(&update_id(&via_pnpm)),
        Some(Action::Update {
            name: "typescript".to_string(),
            source: SourceKind::Pnpm
        })
    );
}

#[test]
fn unknown_ids_are_ignored() {
    assert_eq!(Action::from_key("something-else"), None);
    assert_eq!(Action::from_key("update:cargo:prettier"), None);
    assert_eq!(Action::from_key("update:npm"), None);
    assert_eq!(Action::from_key("update:npm:"), None);
}

#[test]
fn a_bulk_update_id_carries_every_target_it_names() {
    assert_eq!(
        Action::from_key("update-many:npm:@salesforce/cli|pnpm:vite"),
        Some(Action::UpdateMany {
            refs: vec![
                PackageRef {
                    name: "@salesforce/cli".to_string(),
                    source: SourceKind::Npm,
                },
                PackageRef {
                    name: "vite".to_string(),
                    source: SourceKind::Pnpm,
                },
            ]
        })
    );
}

#[test]
fn a_bulk_update_drops_targets_it_cannot_resolve_rather_than_the_whole_batch() {
    assert_eq!(
        Action::from_key("update-many:cargo:thing|npm:prettier"),
        Some(Action::UpdateMany {
            refs: vec![PackageRef {
                name: "prettier".to_string(),
                source: SourceKind::Npm,
            }]
        })
    );
    assert_eq!(Action::from_key("update-many:cargo:thing"), None);
}

#[test]
fn a_source_toggle_id_round_trips() {
    for kind in crate::model::KINDS {
        assert_eq!(
            Action::from_key(&source_id(kind)),
            Some(Action::ToggleSource { kind })
        );
    }
    assert_eq!(Action::from_key("source:cargo"), None);
}

#[test]
fn the_tray_menu_no_longer_lists_packages() {
    let packages = vec![
        behind("prettier", "2.0.0"),
        package("typescript", SourceKind::Npm, Status::Current),
    ];
    let built = build(&view(&packages, None, 0)).unwrap();
    let ids = top_level_ids(&built);

    assert!(
        !ids.iter().any(|id| id.starts_with(UPDATE_PREFIX)),
        "package rows belong to the window now: {ids:?}"
    );
    assert!(ids.contains(&ID_OPEN_WINDOW.to_string()), "{ids:?}");
    assert!(ids.contains(&ID_UPDATE_ALL.to_string()), "{ids:?}");
}

#[test]
fn update_all_counts_only_what_globlin_can_actually_update() {
    let packages = vec![
        behind("prettier", "2.0.0"),
        Package {
            name: "Git.Git".to_string(),
            current: "2.44.0".to_string(),
            source: SourceKind::Winget,
            status: Status::Outdated {
                latest: "2.47.1".to_string(),
            },
        },
    ];
    let built = build(&view(&packages, None, 0)).unwrap();
    let text = built
        .menu
        .items()
        .iter()
        .filter_map(|item| item.as_menuitem().map(tray_icon::menu::MenuItem::text))
        .find(|text| text.starts_with("Update all"))
        .expect("expected an update-all row");

    assert_eq!(text, "Update all (1)");
}

#[test]
fn a_read_only_source_offers_no_update_all_row_of_its_own() {
    let packages = vec![Package {
        name: "Git.Git".to_string(),
        current: "2.44.0".to_string(),
        source: SourceKind::Winget,
        status: Status::Outdated {
            latest: "2.47.1".to_string(),
        },
    }];
    let built = build(&view(&packages, None, 0)).unwrap();

    assert!(!top_level_ids(&built).contains(&ID_UPDATE_ALL.to_string()));
}

#[test]
fn the_idle_headline_counts_only_outdated_packages() {
    let packages = vec![
        behind("a", "2.0.0"),
        package("b", SourceKind::Npm, Status::Current),
        package("c", SourceKind::Npm, Status::Ignored),
    ];

    assert_eq!(
        headline(&view(&packages, None, 0)),
        "Globlin — 1 update available"
    );
}

#[test]
fn an_up_to_date_headline_reports_how_many_are_watched() {
    let packages = vec![
        package("a", SourceKind::Npm, Status::Current),
        package("b", SourceKind::Npm, Status::Current),
    ];

    assert_eq!(
        headline(&view(&packages, None, 0)),
        "Globlin — 2 packages, up to date"
    );
}

#[test]
fn the_update_headline_names_the_package_and_both_versions() {
    let activity = updating("prettier", 0, 1);
    let text = headline(&view(&[], Some(&activity), 0));

    assert!(
        text.starts_with("Updating prettier 1.2.3 → 2.0.0"),
        "{text}"
    );
    assert!(
        !text.contains('['),
        "a single target needs no counter: {text}"
    );
}

#[test]
fn a_batch_update_headline_shows_a_one_based_counter() {
    let activity = updating("vercel", 1, 3);
    assert!(
        headline(&view(&[], Some(&activity), 0)).contains("[2/3]"),
        "expected a 2/3 counter"
    );
}

#[test]
fn the_dots_grow_then_reset_so_the_header_animates() {
    let activity = Activity::Checking;
    let rendered: Vec<String> = (0..8)
        .map(|frame| headline(&view(&[], Some(&activity), frame)))
        .collect();

    assert_eq!(rendered[0], "Checking for updates");
    assert_eq!(rendered[2], "Checking for updates.");
    assert_eq!(rendered[4], "Checking for updates..");
    assert_eq!(rendered[6], "Checking for updates...");
}

#[test]
fn the_dot_animation_repeats_after_a_full_cycle() {
    assert_eq!(dots(0), dots(DOT_CYCLE * FRAMES_PER_DOT));
    assert_eq!(dots(3), dots(3 + DOT_CYCLE * FRAMES_PER_DOT));
}

#[test]
fn the_row_spinner_advances_every_frame_and_repeats() {
    let ticks: Vec<char> = (0..5).map(spinner_tick).collect();

    assert_eq!(ticks[0], ticks[4]);
    assert_eq!(
        ticks[..4]
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}

#[test]
fn only_the_active_row_carries_a_bar() {
    let activity = updating("beta", 1, 3);
    let view = updating_view(&activity, Duration::from_secs(2));

    let first = batch_row_text(&view, 0).unwrap();
    let active = batch_row_text(&view, 1).unwrap();
    let queued = batch_row_text(&view, 2).unwrap();

    assert!(active.contains('█') || active.contains('░'), "{active}");
    assert!(!first.contains('░'), "{first}");
    assert!(!queued.contains('░'), "{queued}");
    assert!(queued.contains("queued"), "{queued}");
}

#[test]
fn a_failed_row_never_renders_as_done() {
    let Activity::Updating { mut batch } = updating("alpha", 0, 2) else {
        panic!("expected an update activity");
    };
    batch.finish(0, false);
    batch.start(1);
    let activity = Activity::Updating { batch };
    let view = updating_view(&activity, Duration::ZERO);

    let failed = batch_row_text(&view, 0).unwrap();
    assert!(failed.contains("failed"), "{failed}");
    assert!(!failed.contains("done"), "{failed}");
    assert!(failed.starts_with('✗'), "{failed}");
}

#[test]
fn a_landed_row_reports_done_with_a_tick() {
    let Activity::Updating { mut batch } = updating("alpha", 0, 2) else {
        panic!("expected an update activity");
    };
    batch.finish(0, true);
    batch.start(1);
    let activity = Activity::Updating { batch };
    let view = updating_view(&activity, Duration::ZERO);

    let done = batch_row_text(&view, 0).unwrap();
    assert!(done.starts_with('✓') && done.contains("done"), "{done}");
}

#[test]
fn a_row_beyond_the_batch_has_no_text() {
    let activity = updating("alpha", 0, 1);
    let view = updating_view(&activity, Duration::ZERO);

    assert_eq!(batch_row_text(&view, 7), None);
}

#[test]
fn an_ignore_id_round_trips_including_scoped_names() {
    for name in ["prettier", "@salesforce/cli"] {
        let package = package(name, SourceKind::Npm, Status::Current);
        assert_eq!(
            Action::from_key(&ignore_id(&package)),
            Some(Action::ToggleIgnore {
                name: name.to_string()
            })
        );
    }
}

#[test]
fn the_same_name_from_two_sources_gets_two_distinct_ignore_ids() {
    let from_npm = package("typescript", SourceKind::Npm, Status::Current);
    let from_bun = package("typescript", SourceKind::Bun, Status::Current);

    assert_ne!(ignore_id(&from_npm), ignore_id(&from_bun));
}

#[test]
fn an_ignore_id_with_an_unknown_source_is_rejected() {
    assert_eq!(Action::from_key("ignore:cargo:prettier"), None);
    assert_eq!(Action::from_key("ignore:prettier"), None);
}

#[test]
fn a_remove_id_round_trips_including_scoped_names() {
    assert_eq!(
        Action::from_key("remove:npm:@salesforce/cli"),
        Some(Action::Remove {
            name: "@salesforce/cli".to_string(),
            source: SourceKind::Npm,
        })
    );
    assert_eq!(
        Action::from_key("remove:bun:prettier"),
        Some(Action::Remove {
            name: "prettier".to_string(),
            source: SourceKind::Bun,
        })
    );
}

#[test]
fn a_remove_id_with_an_unknown_source_is_rejected() {
    assert_eq!(Action::from_key("remove:cargo:prettier"), None);
    assert_eq!(Action::from_key("remove:prettier"), None);
}

#[test]
fn the_self_update_id_parses_back_to_its_action() {
    assert_eq!(Action::from_key(ID_UPDATE_SELF), Some(Action::UpdateSelf));
}

#[test]
fn the_self_update_log_action_round_trips_through_its_id() {
    assert_eq!(
        Action::from_key(ID_OPEN_SELF_LOG),
        Some(Action::OpenSelfLog)
    );
}

#[test]
fn the_auto_update_id_parses_back_to_its_action() {
    assert_eq!(
        Action::from_key(ID_AUTO_UPDATE),
        Some(Action::ToggleAutoUpdate)
    );
}

#[test]
fn a_package_batch_and_a_self_update_never_share_a_headline() {
    let empty_batch = Activity::Updating {
        batch: crate::model::Batch::new(Vec::new()),
    };
    let self_update = Activity::SelfUpdate;

    let batch_headline = headline(&view(&[], Some(&empty_batch), 0));
    let self_update_headline = headline(&view(&[], Some(&self_update), 0));

    assert_ne!(batch_headline, self_update_headline);
    assert!(!headline(&view(&[], Some(&Activity::Checking), 0)).contains("Globlin"));
}

#[test]
fn the_self_update_row_names_both_versions() {
    let release = crate::selfupdate::Release {
        version: semver::Version::parse("0.2.0").unwrap(),
        exe_url: String::new(),
        sha_url: String::new(),
    };
    assert_eq!(
        self_update_text(&release),
        format!("Update Globlin {} → 0.2.0", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_self_update_controls_live_inside_a_submenu_named_after_the_running_version() {
    let release = crate::selfupdate::Release {
        version: semver::Version::parse("9.9.9").unwrap(),
        exe_url: String::new(),
        sha_url: String::new(),
    };
    let pending_restart = Version::parse("9.9.9").unwrap();
    let self_view = View {
        packages: &[],
        activity: None,
        autostart: false,
        self_update: SelfUpdate::Own {
            release: Some(&release),
            auto_update: true,
            log: true,
        },
        pending_restart: Some(&pending_restart),
        frame: 0,
        elapsed: Duration::ZERO,
    };

    let built = build(&self_view).unwrap();
    let top_level = built.menu.items();
    let ids = top_level_ids(&built);
    assert!(
        !ids.contains(&ID_UPDATE_SELF.to_string()),
        "the self-update row should not sit at the top level: {ids:?}"
    );

    let self_block = top_level
        .iter()
        .filter_map(|item| item.as_submenu())
        .find(|submenu| submenu.text().starts_with("Globlin v"))
        .expect("expected a submenu labelled with the running version");
    assert_eq!(
        self_block.text(),
        format!("Globlin v{}", env!("CARGO_PKG_VERSION"))
    );

    let self_block_ids: Vec<String> = self_block
        .items()
        .iter()
        .map(|item| item.id().as_ref().to_string())
        .collect();
    assert!(self_block_ids.contains(&ID_UPDATE_SELF.to_string()));
    assert!(self_block_ids.contains(&ID_AUTO_UPDATE.to_string()));
    assert!(self_block_ids.contains(&ID_OPEN_SELF_LOG.to_string()));
}

#[test]
fn a_winget_managed_install_offers_no_self_update_and_no_auto_update() {
    let winget_view = View {
        packages: &[],
        activity: None,
        autostart: false,
        self_update: SelfUpdate::Winget,
        pending_restart: None,
        frame: 0,
        elapsed: Duration::ZERO,
    };

    let built = build(&winget_view).unwrap();
    let top_level = built.menu.items();
    let self_block = top_level
        .iter()
        .filter_map(|item| item.as_submenu())
        .find(|submenu| submenu.text().starts_with("Globlin v"))
        .expect("expected a submenu labelled with the running version");

    let items = self_block.items();
    let ids: Vec<String> = items
        .iter()
        .map(|item| item.id().as_ref().to_string())
        .collect();
    assert!(
        !ids.contains(&ID_UPDATE_SELF.to_string()),
        "winget owns the binary, so Globlin must not offer to replace it: {ids:?}"
    );
    assert!(
        !ids.contains(&ID_AUTO_UPDATE.to_string()),
        "an auto-update toggle that cannot act would be a lie: {ids:?}"
    );
    assert!(
        !ids.contains(&ID_OPEN_SELF_LOG.to_string()),
        "there is no self-update log when self-update never runs: {ids:?}"
    );

    let texts: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_menuitem().map(tray_icon::menu::MenuItem::text))
        .collect();
    assert!(
        texts.iter().any(|text| text.contains("winget")),
        "the submenu should say where updates come from instead: {texts:?}"
    );
}

#[test]
fn the_remove_headline_names_the_package_and_its_source() {
    let activity = Activity::Removing {
        target: crate::model::RemoveTarget {
            name: "prettier".to_string(),
            source: SourceKind::Bun,
        },
    };

    let text = headline(&view(&[], Some(&activity), 0));

    assert!(text.starts_with("Removing prettier (bun)"), "{text}");
}

#[test]
fn removing_and_checking_never_share_a_headline() {
    let activity = Activity::Removing {
        target: crate::model::RemoveTarget {
            name: "prettier".to_string(),
            source: SourceKind::Npm,
        },
    };

    assert_ne!(
        headline(&view(&[], Some(&activity), 0)),
        headline(&view(&[], Some(&Activity::Checking), 0))
    );
}
