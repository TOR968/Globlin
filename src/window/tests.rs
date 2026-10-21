use std::time::Duration;

use super::*;
use crate::config::Sources;
use crate::model::{Batch, Blocked, BlockedScript, UpdateTarget};

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
        approvals: &[],
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
            brew: false,
            winget: true,
            choco: false,
            skills: false,
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

fn snapshot(view: &View, config: &Config) -> Snapshot {
    snapshot_on(view, config, "windows")
}

#[test]
fn every_source_the_platform_can_have_gets_a_sidebar_row_even_when_it_is_switched_off() {
    let snapshot = snapshot(&view(&[], None), &config());
    let mut labels: Vec<&str> = snapshot.sources.iter().map(|row| row.label).collect();
    let mut expected: Vec<&str> = KINDS
        .iter()
        .filter(|kind| kind.available_on("windows"))
        .map(|kind| kind.label())
        .collect();
    labels.sort_unstable();
    expected.sort_unstable();

    assert_eq!(labels, expected);
}

#[test]
fn the_sidebar_lists_busy_sources_first_and_switched_off_ones_last() {
    let mut config = config();
    config.sources.pnpm = true;
    config.sources.scoop = true;
    let packages = vec![
        package("typescript", SourceKind::Npm, Status::Current),
        package("eslint", SourceKind::Npm, Status::Current),
        outdated("Git.Git", SourceKind::Winget, "2.47.1"),
        package("7zip", SourceKind::Scoop, Status::Current),
    ];
    let snapshot = snapshot(&view(&packages, None), &config);
    let labels: Vec<&str> = snapshot.sources.iter().map(|row| row.label).collect();

    assert_eq!(&labels[..5], ["winget", "npm", "scoop", "pnpm", "bun"]);
    assert_eq!(labels.last(), Some(&"skills"));
}

#[test]
fn packages_follow_the_sidebar_order_with_outdated_ones_first_in_each_source() {
    let packages = vec![
        package("eslint", SourceKind::Npm, Status::Current),
        outdated("prettier", SourceKind::Npm, "2.0.0"),
        package("typescript", SourceKind::Npm, Status::Unknown),
        outdated("Git.Git", SourceKind::Winget, "2.47.1"),
        outdated("Node.js", SourceKind::Winget, "22.0.0"),
    ];
    let snapshot = snapshot(&view(&packages, None), &config());
    let keys: Vec<&str> = snapshot
        .packages
        .iter()
        .map(|row| row.key.as_str())
        .collect();

    assert_eq!(
        keys,
        vec![
            "winget:Git.Git",
            "winget:Node.js",
            "npm:prettier",
            "npm:eslint",
            "npm:typescript"
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
    batch.finish(0, RowState::Failed);
    batch.start(1);
    let activity = Activity::Updating { batch };
    let snapshot = snapshot(&view(&[], Some(&activity)), &config());

    assert!(snapshot.busy);
    assert_eq!(snapshot.batch.len(), 2);
    assert_eq!(snapshot.batch[0].state, "failed");
    assert_eq!(snapshot.batch[1].state, "active");
    assert_eq!(snapshot.batch[0].name, "alpha");
}

#[test]
fn a_skipped_batch_row_reaches_the_page_as_skipped() {
    let mut batch = Batch::new(vec![UpdateTarget {
        name: "Docker.DockerDesktop".to_string(),
        source: SourceKind::Winget,
        from: "4.93.0".to_string(),
        to: "4.95.0".to_string(),
    }]);
    batch.finish(0, RowState::Skipped);
    let activity = Activity::Updating { batch };
    let snapshot = snapshot(&view(&[], Some(&activity)), &config());

    assert_eq!(snapshot.batch[0].state, "skipped");
}

#[test]
fn a_batch_row_carries_its_target_apart_so_the_window_can_group_it_by_source() {
    let batch = Batch::new(vec![UpdateTarget {
        name: "docx".to_string(),
        source: SourceKind::Bun,
        from: "9.9.0".to_string(),
        to: "9.10.0".to_string(),
    }]);
    let activity = Activity::Updating { batch };
    let snapshot = snapshot(&view(&[], Some(&activity)), &config());

    assert_eq!(
        snapshot.batch,
        vec![BatchRow {
            name: "docx".to_string(),
            source: "bun",
            from: "9.9.0".to_string(),
            to: "9.10.0".to_string(),
            state: "active",
        }]
    );
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
        approvals: &[],
        autostart: false,
        self_update: SelfUpdate::Managed(crate::install::Manager::Winget),
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
fn a_blocked_update_reaches_the_window_with_its_scripts_and_ids() {
    let approvals = vec![Blocked {
        target: UpdateTarget {
            name: "@deepseek-ai/dsh".to_string(),
            source: SourceKind::Npm,
            from: "0.1.5-rc.3".to_string(),
            to: "0.1.7-rc.2".to_string(),
        },
        scripts: vec![BlockedScript {
            name: "koffi".to_string(),
            version: "3.3.2".to_string(),
            scripts: "install: node ./cnoke.cjs".to_string(),
        }],
    }];
    let view = View {
        approvals: &approvals,
        ..view(&[], None)
    };

    let snapshot = snapshot(&view, &config());
    let row = &snapshot.approvals[0];

    assert_eq!(row.name, "@deepseek-ai/dsh");
    assert_eq!(row.source, "npm");
    assert_eq!(row.from, "0.1.5-rc.3");
    assert_eq!(row.to, "0.1.7-rc.2");
    assert_eq!(row.approve_id, "approve:npm:@deepseek-ai/dsh");
    assert_eq!(row.dismiss_id, "dismiss:npm:@deepseek-ai/dsh");
    assert_eq!(
        row.scripts,
        vec![ScriptRow {
            package: "koffi@3.3.2".to_string(),
            scripts: "install: node ./cnoke.cjs".to_string(),
        }]
    );
}

#[test]
fn a_window_with_nothing_blocked_has_no_approvals() {
    let snapshot = snapshot(&view(&[], None), &config());

    assert!(snapshot.approvals.is_empty());
}

#[test]
fn the_monochrome_icon_setting_reaches_the_window_from_the_config() {
    let mut config = config();
    assert!(!snapshot(&view(&[], None), &config).monochrome_icon);

    config.monochrome_icon = true;

    assert!(snapshot(&view(&[], None), &config).monochrome_icon);
}

#[test]
fn the_auto_approve_setting_reaches_the_window_from_the_config() {
    let mut config = config();
    assert!(!snapshot(&view(&[], None), &config).auto_approve);

    config.auto_approve_scripts = true;

    assert!(snapshot(&view(&[], None), &config).auto_approve);
}

#[test]
fn only_the_page_globlin_loaded_itself_counts_as_its_own() {
    assert!(is_own_page("about:blank"));
    assert!(is_own_page("data:text/html;charset=utf-8;base64,PGh0bWw+"));
    for foreign in [
        "file:///C:/Users/me/Downloads/evil.html",
        "https://example.com/",
        "about:blank#x",
        "data:image/svg+xml,<svg/>",
        "javascript:alert(1)",
    ] {
        assert!(!is_own_page(foreign), "{foreign}");
    }
}

#[test]
fn a_mac_sidebar_never_offers_scoop_winget_or_choco() {
    let snapshot = snapshot_on(&view(&[], None), &config(), "macos");
    let labels: Vec<&str> = snapshot.sources.iter().map(|row| row.label).collect();

    for windows_only in ["scoop", "winget", "choco"] {
        assert!(!labels.contains(&windows_only), "{labels:?}");
    }
    assert!(labels.contains(&"brew"), "{labels:?}");
}

#[test]
fn a_windows_sidebar_never_offers_homebrew() {
    let snapshot = snapshot(&view(&[], None), &config());

    assert!(!snapshot.sources.iter().any(|row| row.label == "brew"));
}

const SCREENSHOT_PACKAGES: [(&str, SourceKind, &str, &str, bool, bool); 22] = [
    (
        "@salesforce/cli",
        SourceKind::Npm,
        "2.153.5",
        "2.154.0",
        true,
        true,
    ),
    ("prettier", SourceKind::Npm, "3.9.10", "3.10.1", true, true),
    ("typescript", SourceKind::Npm, "5.9.2", "5.9.3", false, true),
    ("eslint", SourceKind::Npm, "9.36.0", "9.37.0", false, true),
    ("vercel", SourceKind::Npm, "48.1.0", "48.2.4", false, true),
    (
        "npm-check-updates",
        SourceKind::Npm,
        "19.0.0",
        "19.0.0",
        false,
        false,
    ),
    ("serve", SourceKind::Npm, "14.2.5", "14.2.5", false, false),
    ("nodemon", SourceKind::Npm, "3.1.10", "3.1.10", false, false),
    ("tsx", SourceKind::Npm, "4.20.6", "4.20.6", false, false),
    ("pm2", SourceKind::Npm, "6.0.13", "6.0.13", false, false),
    (
        "http-server",
        SourceKind::Npm,
        "14.1.1",
        "14.1.1",
        false,
        false,
    ),
    ("yo", SourceKind::Npm, "5.1.0", "5.1.0", false, false),
    ("docx", SourceKind::Bun, "9.9.0", "9.10.0", true, true),
    (
        "@biomejs/biome",
        SourceKind::Bun,
        "2.2.4",
        "2.2.5",
        false,
        true,
    ),
    (
        "wrangler",
        SourceKind::Bun,
        "4.42.0",
        "4.42.0",
        false,
        false,
    ),
    (
        "esbuild",
        SourceKind::Bun,
        "0.25.10",
        "0.25.10",
        false,
        false,
    ),
    (
        "golang.org/x/tools/gopls",
        SourceKind::Go,
        "0.19.1",
        "0.20.0",
        false,
        true,
    ),
    (
        "honnef.co/go/tools/cmd/staticcheck",
        SourceKind::Go,
        "0.6.0",
        "0.6.1",
        false,
        true,
    ),
    (
        "github.com/golangci/golangci-lint/v2/cmd/golangci-lint",
        SourceKind::Go,
        "2.4.0",
        "2.5.0",
        false,
        true,
    ),
    (
        "github.com/go-delve/delve/cmd/dlv",
        SourceKind::Go,
        "1.25.2",
        "1.25.2",
        false,
        false,
    ),
    ("7zip", SourceKind::Scoop, "24.09", "25.01", false, true),
    ("git", SourceKind::Scoop, "2.50.1", "2.51.0", false, true),
];

fn screenshot_packages(running: bool) -> Vec<Package> {
    let mut packages: Vec<Package> = SCREENSHOT_PACKAGES
        .iter()
        .map(|&(name, source, older, newer, idle, batch)| {
            let outdated = if running { batch } else { idle };
            Package {
                name: name.to_string(),
                current: if outdated { older } else { newer }.to_string(),
                source,
                status: if outdated {
                    Status::Outdated {
                        latest: newer.to_string(),
                    }
                } else {
                    Status::Current
                },
            }
        })
        .collect();
    packages.insert(
        12,
        Package {
            name: "npm".to_string(),
            current: "11.6.2".to_string(),
            source: SourceKind::Npm,
            status: Status::Ignored,
        },
    );
    packages
}

fn screenshot_config() -> Config {
    let mut config = config();
    config.sources.bun = true;
    config.sources.go = true;
    config.sources.scoop = true;
    config.sources.pnpm = true;
    config.sources.winget = false;
    config
}

fn screenshot_activity(packages: &[Package]) -> Activity {
    let mut batch = Batch::new(packages.iter().filter_map(Package::update_target).collect());
    for position in 0..5 {
        batch.finish(position, RowState::Done);
    }
    batch.finish(5, RowState::Failed);
    batch.start(6);
    Activity::Updating { batch }
}

fn screenshot_page(snapshot: &Snapshot, hash: Option<&str>) -> String {
    let stub = "<script>window.ipc = { postMessage() {} };</script></head>";
    let navigate = hash.map_or_else(String::new, |hash| {
        format!("history.replaceState(null, \"\", \"{hash}\");")
    });
    let call = format!("<script>{navigate}{}</script></body>", script(snapshot));
    UI.replacen("</head>", stub, 1)
        .replacen("</body>", &call, 1)
}

#[test]
fn a_screenshot_page_stubs_the_bridge_before_the_page_script_runs() {
    let page = screenshot_page(&snapshot(&view(&[], None), &config()), Some("#settings"));
    let stub = page.find("window.ipc = {").unwrap();
    let bridge = page.find("window.globlin = {").unwrap();
    let render = page.rfind("window.globlin.render(").unwrap();

    assert!(stub < bridge && bridge < render);
    assert!(page.contains("history.replaceState(null, \"\", \"#settings\");"));
}

#[test]
fn the_screenshot_batch_has_every_state_the_window_draws() {
    let running = screenshot_packages(true);
    let activity = screenshot_activity(&running);
    let busy = snapshot(&view(&running, Some(&activity)), &screenshot_config());
    let states: Vec<&str> = busy.batch.iter().map(|row| row.state).collect();

    assert_eq!(busy.packages.len(), 23);
    assert_eq!(states.len(), 12);
    for expected in ["done", "failed", "active", "queued"] {
        assert!(states.contains(&expected), "{states:?}");
    }
}

#[test]
#[ignore = "writes the window screenshot pages into $env:GLOBLIN_SCREENSHOT_DIR: cargo test -- --ignored --exact window::tests::dump_window_screenshots"]
fn dump_window_screenshots() {
    let directory = std::env::var_os("GLOBLIN_SCREENSHOT_DIR").map_or_else(
        || std::env::temp_dir().join("globlin-screenshots"),
        std::path::PathBuf::from,
    );
    std::fs::create_dir_all(&directory).unwrap();
    let config = screenshot_config();

    let idle = screenshot_packages(false);
    let updates = snapshot(&view(&idle, None), &config);
    std::fs::write(
        directory.join("updates.html"),
        screenshot_page(&updates, None),
    )
    .unwrap();

    let running = screenshot_packages(true);
    let activity = screenshot_activity(&running);
    let busy = snapshot(&view(&running, Some(&activity)), &config);
    std::fs::write(directory.join("batch.html"), screenshot_page(&busy, None)).unwrap();

    let release = crate::selfupdate::Release {
        version: semver::Version::new(0, 4, 0),
        asset_url: String::new(),
        sums_url: String::new(),
        sums_signature_url: String::new(),
    };
    let settings_view = View {
        self_update: SelfUpdate::Own {
            release: Some(&release),
            auto_update: true,
            log: false,
        },
        ..view(&idle, None)
    };
    let settings = snapshot(&settings_view, &config);
    std::fs::write(
        directory.join("settings.html"),
        screenshot_page(&settings, Some("#settings")),
    )
    .unwrap();

    println!("window screenshot pages written to {}", directory.display());
}

#[test]
fn the_choco_one_prompt_switch_carries_the_setting_on_windows() {
    let mut config = config();
    config.choco_one_prompt = true;

    assert_eq!(
        snapshot(&view(&[], None), &config).choco_one_prompt,
        Some(true)
    );
}

#[test]
fn a_platform_without_choco_has_no_choco_one_prompt_switch() {
    let snapshot = snapshot_on(&view(&[], None), &config(), "linux");

    assert_eq!(snapshot.choco_one_prompt, None);
}

#[test]
fn the_winget_one_prompt_switch_carries_the_setting_on_windows_only() {
    let mut config = config();
    config.winget_one_prompt = true;

    assert_eq!(
        snapshot(&view(&[], None), &config).winget_one_prompt,
        Some(true)
    );
    assert_eq!(
        snapshot_on(&view(&[], None), &config, "linux").winget_one_prompt,
        None
    );
}

#[test]
fn a_winget_row_says_one_prompt_covers_the_batch_when_that_setting_is_on() {
    let mut config = config();
    config.winget_one_prompt = true;
    let packages = vec![outdated("Git.Git", SourceKind::Winget, "2.47.1")];
    let snapshot = snapshot(&view(&packages, None), &config);

    let elevation = snapshot.packages[0].elevation.unwrap();
    assert_eq!(elevation.label, "asks for admin");
    assert!(
        elevation
            .detail
            .contains("once for all the winget updates in a batch"),
        "{}",
        elevation.detail
    );
}

#[test]
fn a_winget_row_warns_that_it_may_ask_for_an_administrator() {
    let packages = vec![outdated("Git.Git", SourceKind::Winget, "2.47.1")];
    let snapshot = snapshot(&view(&packages, None), &config());

    let elevation = snapshot.packages[0].elevation.unwrap();
    assert_eq!(elevation.label, "may ask for admin");
    assert!(
        elevation.detail.contains("whole machine"),
        "{}",
        elevation.detail
    );
}

#[test]
fn a_choco_row_says_windows_asks_once_per_package_by_default() {
    let packages = vec![outdated("git", SourceKind::Choco, "2.47.1")];
    let snapshot = snapshot(&view(&packages, None), &config());

    let elevation = snapshot.packages[0].elevation.unwrap();
    assert_eq!(elevation.label, "asks for admin");
    assert!(
        elevation.detail.contains("once for each choco update"),
        "{}",
        elevation.detail
    );
}

#[test]
fn a_choco_row_says_one_prompt_covers_the_batch_when_that_setting_is_on() {
    let mut config = config();
    config.choco_one_prompt = true;
    let packages = vec![outdated("git", SourceKind::Choco, "2.47.1")];
    let snapshot = snapshot(&view(&packages, None), &config);

    let elevation = snapshot.packages[0].elevation.unwrap();
    assert!(
        elevation
            .detail
            .contains("once for all the choco updates in a batch"),
        "{}",
        elevation.detail
    );
}

#[test]
fn a_source_that_never_needs_an_administrator_carries_no_warning() {
    let packages = vec![outdated("prettier", SourceKind::Npm, "3.0.0")];
    let snapshot = snapshot(&view(&packages, None), &config());

    assert_eq!(snapshot.packages[0].elevation, None);
    let npm = snapshot
        .sources
        .iter()
        .find(|row| row.label == "npm")
        .unwrap();
    assert_eq!(npm.elevation, None);
}

#[test]
fn the_settings_rows_for_winget_and_choco_carry_the_same_warning_as_their_packages() {
    let snapshot = snapshot(&view(&[], None), &config());
    let labels: Vec<(&str, &str)> = snapshot
        .sources
        .iter()
        .filter_map(|row| row.elevation.map(|elevation| (row.label, elevation.label)))
        .collect();

    assert_eq!(
        labels,
        vec![("winget", "may ask for admin"), ("choco", "asks for admin")]
    );
}
