use super::*;
use crate::model::UpdateTarget;
use semver::Version;

fn package(name: &str, source: SourceKind, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: Version::parse("1.2.3").unwrap(),
        source,
        status,
    }
}

fn behind(name: &str, latest: &str) -> Package {
    package(
        name,
        SourceKind::Npm,
        Status::Outdated {
            latest: Version::parse(latest).unwrap(),
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
            from: Version::parse("1.2.3").unwrap(),
            to: Version::parse("2.0.0").unwrap(),
        })
        .collect();
    let mut batch = crate::model::Batch::new(targets);
    batch.start(index);
    Activity::Updating { batch }
}

#[test]
fn fixed_menu_ids_map_to_their_actions() {
    assert_eq!(Action::from_key("quit"), Some(Action::Quit));
    assert_eq!(Action::from_key("check-now"), Some(Action::CheckNow));
    assert_eq!(Action::from_key("update-all"), Some(Action::UpdateAll));
    assert_eq!(Action::from_key("autostart"), Some(Action::ToggleAutostart));
    assert_eq!(Action::from_key("open-log"), Some(Action::OpenLog));
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
fn the_same_name_from_two_sources_gets_two_distinct_ids() {
    let from_npm = package("typescript", SourceKind::Npm, Status::Current);
    let from_bun = package("typescript", SourceKind::Bun, Status::Current);

    assert_ne!(update_id(&from_npm), update_id(&from_bun));
    assert_eq!(
        Action::from_key(&update_id(&from_bun)),
        Some(Action::Update {
            name: "typescript".to_string(),
            source: SourceKind::Bun
        })
    );
}

#[test]
fn unknown_ids_are_ignored() {
    assert_eq!(Action::from_key("something-else"), None);
    assert_eq!(Action::from_key("update:pnpm:prettier"), None);
    assert_eq!(Action::from_key("update:npm"), None);
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
fn each_status_gets_its_own_marker() {
    let outdated = row(&behind("a", "2.0.0"));
    let current = row(&package("b", SourceKind::Npm, Status::Current));
    let unknown = row(&package("c", SourceKind::Npm, Status::Unknown));
    let ignored = row(&package("d", SourceKind::Npm, Status::Ignored));

    assert!(outdated.starts_with('↑'), "{outdated}");
    assert!(current.starts_with('✓'), "{current}");
    assert!(
        unknown.starts_with('?') && unknown.contains("not checked"),
        "{unknown}"
    );
    assert!(
        ignored.starts_with('·') && ignored.contains("ignored"),
        "{ignored}"
    );
}

#[test]
fn an_outdated_row_shows_installed_and_target_versions() {
    assert!(row(&behind("prettier", "2.0.0")).contains("1.2.3 → 2.0.0"));
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
fn bun_packages_are_labelled_so_duplicates_are_distinguishable() {
    let from_bun = package("typescript", SourceKind::Bun, Status::Current);
    let from_npm = package("typescript", SourceKind::Npm, Status::Current);

    assert!(row(&from_bun).contains("(bun)"));
    assert!(!row(&from_npm).contains("(bun)"));
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
    assert_eq!(Action::from_key("ignore:pnpm:prettier"), None);
    assert_eq!(Action::from_key("ignore:prettier"), None);
}

#[test]
fn only_an_outdated_package_offers_an_update_item() {
    assert!(offers_update(&behind("prettier", "2.0.0")));
    assert!(!offers_update(&package(
        "npm",
        SourceKind::Npm,
        Status::Ignored
    )));
    assert!(!offers_update(&package(
        "typescript",
        SourceKind::Npm,
        Status::Current
    )));
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
    let top_level_ids: Vec<String> = top_level
        .iter()
        .map(|item| item.id().as_ref().to_string())
        .collect();
    assert!(
        !top_level_ids.contains(&ID_UPDATE_SELF.to_string()),
        "the self-update row should not sit at the top level: {top_level_ids:?}"
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

fn confirm_ids(built: &Built) -> Vec<String> {
    let mut ids = Vec::new();
    for item in built.menu.items() {
        let Some(row) = item.as_submenu() else {
            continue;
        };
        for entry in row.items() {
            let Some(nested) = entry.as_submenu() else {
                continue;
            };
            for leaf in nested.items() {
                ids.push(leaf.id().as_ref().to_string());
            }
        }
    }
    ids
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
    assert_eq!(Action::from_key("remove:pnpm:prettier"), None);
    assert_eq!(Action::from_key("remove:prettier"), None);
}

#[test]
fn every_row_offers_a_confirmed_uninstall_whatever_its_status() {
    let packages = vec![
        behind("prettier", "2.0.0"),
        package("typescript", SourceKind::Npm, Status::Current),
        package("npm", SourceKind::Npm, Status::Ignored),
        package("mystery", SourceKind::Bun, Status::Unknown),
    ];
    let built = build(&view(&packages, None, 0)).unwrap();
    let ids = confirm_ids(&built);

    for expected in [
        "remove:npm:prettier",
        "remove:npm:typescript",
        "remove:npm:npm",
        "remove:bun:mystery",
    ] {
        assert!(
            ids.contains(&expected.to_string()),
            "{expected} missing from {ids:?}"
        );
    }
}

#[test]
fn the_uninstall_submenu_itself_triggers_no_action() {
    let packages = vec![behind("prettier", "2.0.0")];
    let built = build(&view(&packages, None, 0)).unwrap();
    let mut seen = 0;

    for item in built.menu.items() {
        let Some(row) = item.as_submenu() else {
            continue;
        };
        for entry in row.items() {
            let Some(nested) = entry.as_submenu() else {
                continue;
            };
            seen += 1;
            assert_eq!(nested.text(), "Uninstall");
            assert_eq!(Action::from_id(nested.id()), None);
        }
    }

    assert_eq!(seen, 1);
}

#[test]
fn a_busy_menu_disables_the_confirm_item() {
    let packages = vec![package("typescript", SourceKind::Npm, Status::Current)];
    let activity = updating("alpha", 0, 1);
    let built = build(&view(&packages, Some(&activity), 0)).unwrap();
    let mut checked = 0;

    for item in built.menu.items() {
        let Some(row) = item.as_submenu() else {
            continue;
        };
        for entry in row.items() {
            let Some(nested) = entry.as_submenu() else {
                continue;
            };
            for leaf in nested.items() {
                let leaf = leaf.as_menuitem().expect("the confirm row is a plain item");
                assert!(!leaf.is_enabled(), "{} should be disabled", leaf.text());
                checked += 1;
            }
        }
    }

    assert_eq!(checked, 1);
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
