use std::time::Duration;

use super::*;
use crate::config::Sources;
use crate::model::{Batch, UpdateTarget};

fn package(name: &str, source: SourceKind, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: "1.2.3".to_string(),
        source,
        status,
    }
}

fn outdated(name: &str, source: SourceKind, latest: &str) -> Package {
    package(
        name,
        source,
        Status::Outdated {
            latest: latest.to_string(),
        },
    )
}

fn view<'a>(packages: &'a [Package], activity: Option<&'a Activity>) -> View<'a> {
    View {
        packages,
        activity,
        autostart: true,
        self_update: SelfUpdate::Own {
            release: None,
            auto_update: false,
            log: false,
        },
        pending_restart: None,
        frame: 0,
        elapsed: Duration::ZERO,
    }
}

fn config() -> Config {
    Config {
        sources: Sources {
            npm: true,
            bun: false,
            pnpm: false,
            yarn: false,
            pipx: false,
            uv: false,
            scoop: false,
            cargo: false,
            go: false,
            dotnet: false,
            psgallery: false,
            gem: false,
            winget: true,
            choco: false,
        },
        ignore: Vec::new(),
        ..Default::default()
    }
}

#[test]
fn every_package_reaches_the_window_with_the_ids_that_act_on_it() {
    let packages = vec![outdated("prettier", SourceKind::Npm, "2.0.0")];
    let snapshot = snapshot(&view(&packages, None), &config());
    let row = &snapshot.packages[0];

    assert_eq!(row.key, "npm:prettier");
    assert_eq!(row.update_id, "update:npm:prettier");
    assert_eq!(row.ignore_id, "ignore:npm:prettier");
    assert_eq!(row.remove_id, "remove:npm:prettier");
    assert_eq!(row.current, "1.2.3");
    assert_eq!(row.latest.as_deref(), Some("2.0.0"));
    assert_eq!(row.status, "outdated");
    assert!(!row.read_only);
}

#[test]
fn a_read_only_source_is_flagged_so_the_window_can_disable_its_update_button() {
    let packages = vec![outdated("Git.Git", SourceKind::Winget, "2.47.1")];
    let snapshot = snapshot(&view(&packages, None), &config());

    assert!(snapshot.packages[0].read_only);
    assert_eq!(snapshot.packages[0].status, "outdated");
}

#[test]
fn each_status_reaches_the_window_under_its_own_name() {
    let packages = vec![
        outdated("a", SourceKind::Npm, "2.0.0"),
        package("b", SourceKind::Npm, Status::Current),
        package("c", SourceKind::Npm, Status::Unknown),
        package("d", SourceKind::Npm, Status::Ignored),
    ];
    let snapshot = snapshot(&view(&packages, None), &config());
    let statuses: Vec<&str> = snapshot.packages.iter().map(|row| row.status).collect();

    assert_eq!(statuses, vec!["outdated", "current", "unknown", "ignored"]);
}

#[test]
fn a_package_that_is_not_checked_is_never_reported_as_current() {
    let packages = vec![package("c", SourceKind::Npm, Status::Unknown)];
    let snapshot = snapshot(&view(&packages, None), &config());

    assert_eq!(snapshot.packages[0].status, "unknown");
    assert_eq!(snapshot.packages[0].latest, None);
}

#[test]
fn every_source_gets_a_sidebar_row_even_when_it_is_switched_off() {
    let snapshot = snapshot(&view(&[], None), &config());
    let labels: Vec<&str> = snapshot.sources.iter().map(|row| row.label).collect();

    assert_eq!(
        labels,
        vec![
            "npm",
            "bun",
            "pnpm",
            "yarn",
            "pipx",
            "uv",
            "scoop",
            "cargo",
            "go",
            "dotnet",
            "psgallery",
            "gem",
            "winget",
            "choco"
        ]
    );
}

#[test]
fn a_sidebar_row_carries_the_toggle_that_switches_its_source() {
    let snapshot = snapshot(&view(&[], None), &config());
    let winget = snapshot
        .sources
        .iter()
        .find(|row| row.label == "winget")
        .unwrap();

    assert_eq!(winget.id, "source:winget");
    assert!(winget.enabled);
    assert!(winget.read_only);
}

#[test]
fn a_source_that_is_off_in_the_config_says_so() {
    let snapshot = snapshot(&view(&[], None), &config());
    let choco = snapshot
        .sources
        .iter()
        .find(|row| row.label == "choco")
        .unwrap();

    assert!(!choco.enabled);
    assert_eq!(choco.total, 0);
}

#[test]
fn the_sidebar_counts_are_per_source_not_global() {
    let packages = vec![
        outdated("prettier", SourceKind::Npm, "2.0.0"),
        package("typescript", SourceKind::Npm, Status::Current),
        outdated("Git.Git", SourceKind::Winget, "2.47.1"),
    ];
    let snapshot = snapshot(&view(&packages, None), &config());
    let npm = snapshot
        .sources
        .iter()
        .find(|row| row.label == "npm")
        .unwrap();
    let winget = snapshot
        .sources
        .iter()
        .find(|row| row.label == "winget")
        .unwrap();

    assert_eq!((npm.total, npm.outdated), (2, 1));
    assert_eq!((winget.total, winget.outdated), (1, 1));
}

#[test]
fn an_idle_window_carries_no_batch_rows() {
    let snapshot = snapshot(&view(&[], None), &config());

    assert!(snapshot.batch.is_empty());
    assert!(!snapshot.busy);
}

#[test]
fn a_running_batch_reaches_the_window_with_one_row_per_target_and_its_state() {
    let mut batch = Batch::new(vec![
        UpdateTarget {
            name: "alpha".to_string(),
            source: SourceKind::Npm,
            from: "1.0.0".to_string(),
            to: "2.0.0".to_string(),
        },
        UpdateTarget {
            name: "beta".to_string(),
            source: SourceKind::Npm,
            from: "1.0.0".to_string(),
            to: "2.0.0".to_string(),
        },
    ]);
    batch.finish(0, false);
    batch.start(1);
    let activity = Activity::Updating { batch };
    let snapshot = snapshot(&view(&[], Some(&activity)), &config());

    assert!(snapshot.busy);
    assert_eq!(snapshot.batch.len(), 2);
    assert_eq!(snapshot.batch[0].state, "failed");
    assert_eq!(snapshot.batch[1].state, "active");
    assert!(snapshot.batch[0].text.contains("alpha"));
}

#[test]
fn the_tick_carries_only_what_the_animation_changes() {
    let activity = Activity::Checking;
    let update = tick(&view(&[], Some(&activity)));

    assert!(update.busy);
    assert!(update.batch.is_empty());
    assert_eq!(update.headline, "Checking for updates");
}

#[test]
fn a_winget_managed_install_tells_the_window_not_to_offer_self_updates() {
    let managed = View {
        packages: &[],
        activity: None,
        autostart: false,
        self_update: SelfUpdate::Winget,
        pending_restart: None,
        frame: 0,
        elapsed: Duration::ZERO,
    };
    let snapshot = snapshot(&managed, &config());

    assert_eq!(snapshot.managed_by, Some("winget"));
    assert_eq!(snapshot.self_update, None);
    assert!(!snapshot.auto_update);
}

#[test]
fn the_render_script_calls_into_the_page_with_a_json_payload() {
    let snapshot = snapshot(&view(&[], None), &config());
    let call = script(&snapshot);

    assert!(call.starts_with("window.globlin.render({"), "{call}");
    assert!(call.ends_with(')'), "{call}");
    assert!(call.contains("\"headline\""), "{call}");
}

#[test]
fn a_package_name_carrying_quotes_cannot_break_out_of_the_script() {
    let packages = vec![package(
        "evil\");alert(\"",
        SourceKind::Npm,
        Status::Current,
    )];
    let snapshot = snapshot(&view(&packages, None), &config());
    let call = script(&snapshot);

    assert!(
        !call.contains("evil\");"),
        "the quote must stay escaped: {call}"
    );
    assert!(call.contains("evil\\\");alert(\\\""), "{call}");
}

#[test]
fn the_page_is_shipped_inside_the_binary() {
    assert!(
        UI.contains("window.globlin"),
        "the UI must define the bridge"
    );
    assert!(
        UI.contains("window-ready"),
        "the UI must announce when it can be rendered into"
    );
}

#[test]
#[cfg(windows)]
#[ignore = "needs a real WebView2 runtime: cargo test -- --ignored --exact window::tests::the_window_shell_starts_against_a_real_webview"]
fn the_window_shell_starts_against_a_real_webview() {
    use tao::event_loop::EventLoopBuilder;
    use tao::platform::windows::EventLoopBuilderExtWindows;

    let event_loop = EventLoopBuilder::<crate::Message>::with_user_event()
        .with_any_thread(true)
        .build();
    let window = Window::new(&event_loop, event_loop.create_proxy()).unwrap();

    assert!(!window.visible());
    window.render(&snapshot(&view(&[], None), &config()));
    window.show();
    assert!(window.visible());
}

#[test]
fn a_source_with_no_uninstall_of_its_own_offers_no_remove_button() {
    let packages = vec![Package {
        name: "golang.org/x/tools/gopls".to_string(),
        current: "0.15.3".to_string(),
        source: SourceKind::Go,
        status: Status::Current,
    }];
    let snapshot = snapshot(&view(&packages, None), &config());

    assert!(!snapshot.packages[0].removable);
    assert!(!snapshot.packages[0].read_only);
}

#[test]
fn a_source_globlin_can_uninstall_from_says_so() {
    let packages = vec![Package {
        name: "prettier".to_string(),
        current: "3.9.6".to_string(),
        source: SourceKind::Npm,
        status: Status::Current,
    }];
    let snapshot = snapshot(&view(&packages, None), &config());

    assert!(snapshot.packages[0].removable);
}
