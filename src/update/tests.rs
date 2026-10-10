use super::*;
use crate::config::Sources;
use crate::model::{BlockedScript, Installed, SourceKind};
use std::sync::Mutex;

fn target(name: &str) -> UpdateTarget {
    UpdateTarget {
        name: name.to_string(),
        source: SourceKind::Npm,
        from: "1.0.0".to_string(),
        to: "2.0.0".to_string(),
    }
}

struct UnrunnableNpm {
    config: Config,
}

impl UnrunnableNpm {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("globlin-fake-npm-{label}"));
        std::fs::write(&path, b"not an executable").unwrap();
        Self {
            config: Config {
                npm_cmd: Some(path),
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
                    winget: false,
                    choco: false,
                },
                ..Default::default()
            },
        }
    }
}

impl Drop for UnrunnableNpm {
    fn drop(&mut self) {
        if let Some(path) = &self.config.npm_cmd {
            std::fs::remove_file(path).ok();
        }
    }
}

#[test]
fn an_empty_target_list_does_nothing_and_announces_nothing() {
    let seen = Mutex::new(Vec::new());
    let outcome = run(&Config::default(), &[], &[], |step| {
        seen.lock().unwrap().push(step);
    });

    assert_eq!(outcome, Outcome::default());
    assert!(seen.lock().unwrap().is_empty());
}

fn started(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .filter(|step| matches!(step, Step::Started { .. }))
        .collect()
}

#[test]
fn every_target_announces_a_start_and_a_finish() {
    let fake = UnrunnableNpm::new("indexes");
    let targets = vec![target("alpha"), target("beta"), target("gamma")];
    let seen = Mutex::new(Vec::new());

    run(&fake.config, &targets, &[], |step| {
        seen.lock().unwrap().push(step);
    });

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 6);
    assert!(matches!(seen[0], Step::Started { index: 0, .. }));
    assert!(matches!(seen[1], Step::Finished { index: 0, .. }));
    assert_eq!(started(&seen).len(), 3);
    assert!(
        matches!(seen[2], Step::Started { index: 1, ref target, total: 3 } if target.name == "beta")
    );
}

#[test]
fn a_start_carries_both_versions_so_the_menu_can_show_them() {
    let fake = UnrunnableNpm::new("versions");
    let seen = Mutex::new(Vec::new());

    run(&fake.config, &[target("alpha")], &[], |step| {
        seen.lock().unwrap().push(step);
    });

    let seen = seen.lock().unwrap();
    let Step::Started { target, .. } = &seen[0] else {
        panic!("the first step should be a start: {:?}", seen[0]);
    };
    assert_eq!(target.from, "1.0.0");
    assert_eq!(target.to, "2.0.0");
}

#[test]
fn a_target_that_cannot_be_started_finishes_with_ok_false() {
    let fake = UnrunnableNpm::new("finishes-false");
    let seen = Mutex::new(Vec::new());

    run(&fake.config, &[target("alpha")], &[], |step| {
        seen.lock().unwrap().push(step);
    });

    let seen = seen.lock().unwrap();
    assert!(matches!(
        seen[1],
        Step::Finished {
            index: 0,
            ok: false
        }
    ));
}

#[test]
fn a_target_that_cannot_be_started_is_reported_as_failed() {
    let fake = UnrunnableNpm::new("unstartable");

    let outcome = run(&fake.config, &[target("alpha")], &[], |_| {});

    assert_eq!(outcome.failed, vec!["alpha".to_string()]);
    assert!(outcome.updated.is_empty());
}

#[test]
fn a_failing_target_does_not_stop_the_ones_behind_it() {
    let fake = UnrunnableNpm::new("continues");
    let targets = vec![target("alpha"), target("beta")];

    let outcome = run(&fake.config, &targets, &[], |_| {});

    assert_eq!(
        outcome.failed,
        vec!["alpha".to_string(), "beta".to_string()]
    );
}

#[test]
fn a_target_whose_source_is_disabled_is_reported_rather_than_skipped_silently() {
    let config = Config {
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
            winget: false,
            choco: false,
        },
        ..Default::default()
    };
    let bun_target = UpdateTarget {
        source: SourceKind::Bun,
        ..target("opencode-ai")
    };

    let outcome = run(&config, &[bun_target], &[], |_| {});

    assert_eq!(outcome.failed, vec!["opencode-ai".to_string()]);
}

#[test]
#[ignore = "installs for real: cargo test -- --ignored --exact update::tests::updates_a_package_for_real"]
fn updates_a_package_for_real() {
    let name = std::env::var("UPDATE_TARGET")
        .expect("set UPDATE_TARGET to the package to install at @latest");
    let targets = vec![target(&name)];

    let outcome = run(&Config::default(), &targets, &[], |_| {});

    assert!(outcome.failed.is_empty(), "failed: {:?}", outcome.failed);
    assert_eq!(outcome.updated, vec![name]);
}

#[test]
#[ignore = "spawns npm for real: cargo test -- --ignored --exact update::tests::a_real_npm_failure_lands_in_the_log"]
fn a_real_npm_failure_lands_in_the_log() {
    let name = "globlin-no-such-package-9d3f".to_string();

    let outcome = run(&Config::default(), &[target(&name)], &[], |_| {});

    assert_eq!(outcome.failed, vec![name.clone()]);
    assert!(outcome.updated.is_empty());
    let log =
        std::fs::read_to_string(diagnostics::log_path()).expect("the failure log should exist");
    assert!(
        log.contains(&name),
        "log did not mention the package: {log}"
    );
}

fn blocked(name: &str, source: SourceKind) -> Blocked {
    Blocked {
        target: UpdateTarget {
            source,
            ..target(name)
        },
        scripts: vec![BlockedScript {
            name: "koffi".to_string(),
            version: "3.3.2".to_string(),
            scripts: "install: node ./cnoke.cjs".to_string(),
        }],
    }
}

#[cfg(windows)]
struct FakeNpmRun {
    config: Config,
}

#[cfg(windows)]
impl FakeNpmRun {
    fn new(label: &str, script: &str) -> Self {
        let path = std::env::temp_dir().join(format!("globlin-fake-npm-run-{label}.cmd"));
        std::fs::write(&path, script).unwrap();
        Self {
            config: Config {
                npm_cmd: Some(path),
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
                    winget: false,
                    choco: false,
                },
                ..Default::default()
            },
        }
    }
}

#[cfg(windows)]
impl Drop for FakeNpmRun {
    fn drop(&mut self) {
        if let Some(path) = &self.config.npm_cmd {
            std::fs::remove_file(path).ok();
        }
    }
}

#[cfg(windows)]
const STRICT_REFUSAL_SCRIPT: &str = "@echo off\r\n\
if \"%1\"==\"config\" goto config\r\n\
if \"%1\"==\"install\" goto install\r\n\
exit /b 1\r\n\
:config\r\n\
echo.\r\n\
exit /b 0\r\n\
:install\r\n\
echo npm error code ESTRICTALLOWSCRIPTS 1>&2\r\n\
echo npm error --strict-allow-scripts: 1 package(s) have install scripts not covered by allowScripts: 1>&2\r\n\
echo npm error   koffi@3.3.2 (install: node ./cnoke.cjs) 1>&2\r\n\
exit /b 1\r\n";

#[cfg(windows)]
#[test]
fn a_strict_refusal_lands_the_target_in_blocked_not_failed() {
    let fake = FakeNpmRun::new("blocked", STRICT_REFUSAL_SCRIPT);

    let outcome = run(&fake.config, &[target("koffi")], &[], |_| {});

    assert!(outcome.failed.is_empty(), "failed: {:?}", outcome.failed);
    assert_eq!(outcome.blocked.len(), 1);
    assert_eq!(outcome.blocked[0].target.name, "koffi");
    assert_eq!(outcome.blocked[0].scripts[0].name, "koffi");
    assert_eq!(outcome.blocked[0].scripts[0].version, "3.3.2");
}

#[cfg(windows)]
const MIXED_BLOCKED_AND_FAILED_SCRIPT: &str = "@echo off\r\n\
if \"%1\"==\"config\" goto config\r\n\
if \"%1\"==\"install\" goto install\r\n\
exit /b 1\r\n\
:config\r\n\
echo.\r\n\
exit /b 0\r\n\
:install\r\n\
echo %3 | findstr /I \"koffi\" >nul\r\n\
if %errorlevel%==0 goto blocked\r\n\
echo npm error code E404 1>&2\r\n\
echo npm error 404 Not Found 1>&2\r\n\
exit /b 1\r\n\
:blocked\r\n\
echo npm error code ESTRICTALLOWSCRIPTS 1>&2\r\n\
echo npm error --strict-allow-scripts: 1 package(s) have install scripts not covered by allowScripts: 1>&2\r\n\
echo npm error   koffi@3.3.2 (install: node ./cnoke.cjs) 1>&2\r\n\
exit /b 1\r\n";

#[cfg(windows)]
#[test]
fn a_batch_can_mix_a_blocked_target_and_a_failed_one() {
    let fake = FakeNpmRun::new("mixed", MIXED_BLOCKED_AND_FAILED_SCRIPT);
    let targets = vec![target("koffi"), target("beta")];

    let outcome = run(&fake.config, &targets, &[], |_| {});

    assert_eq!(outcome.failed, vec!["beta".to_string()]);
    assert_eq!(outcome.blocked.len(), 1);
    assert_eq!(outcome.blocked[0].target.name, "koffi");
}

struct NoScriptsSource;

impl PackageSource for NoScriptsSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Cargo
    }

    fn installed(&self) -> crate::Result<Vec<Installed>> {
        Ok(Vec::new())
    }

    fn update_command(&self, _name: &str) -> Option<std::process::Command> {
        None
    }

    fn uninstall_command(&self, _name: &str) -> Option<std::process::Command> {
        None
    }
}

#[test]
fn a_source_without_its_own_policy_reports_no_blocked_scripts_by_default() {
    assert_eq!(NoScriptsSource.blocked_scripts(""), None);
}

#[test]
fn an_approval_that_cannot_be_saved_fails_every_target_without_running_it() {
    let fake = UnrunnableNpm::new("approval");
    let seen = Mutex::new(Vec::new());

    let outcome = run(
        &fake.config,
        &[target("alpha")],
        &[blocked("alpha", SourceKind::Npm)],
        |step| seen.lock().unwrap().push(step),
    );

    assert_eq!(outcome.failed, vec!["alpha".to_string()]);
    assert!(outcome.blocked.is_empty());
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn a_package_blocked_again_replaces_its_older_approval_entry() {
    let mut approvals = vec![
        blocked("alpha", SourceKind::Npm),
        blocked("beta", SourceKind::Npm),
    ];
    let mut fresh = blocked("alpha", SourceKind::Npm);
    fresh.scripts[0].version = "3.4.0".to_string();
    let outcome = Outcome {
        blocked: vec![fresh.clone()],
        ..Outcome::default()
    };

    settle(&mut approvals, &outcome);

    assert_eq!(approvals, vec![blocked("beta", SourceKind::Npm), fresh]);
}

#[test]
fn a_package_that_updated_leaves_the_approvals() {
    let mut approvals = vec![blocked("alpha", SourceKind::Npm)];
    let outcome = Outcome {
        updated: vec!["alpha".to_string()],
        ..Outcome::default()
    };

    settle(&mut approvals, &outcome);

    assert!(approvals.is_empty());
}

#[test]
fn a_package_that_failed_for_another_reason_keeps_its_approval_entry() {
    let mut approvals = vec![blocked("alpha", SourceKind::Npm)];
    let outcome = Outcome {
        failed: vec!["alpha".to_string()],
        ..Outcome::default()
    };

    settle(&mut approvals, &outcome);

    assert_eq!(approvals, vec![blocked("alpha", SourceKind::Npm)]);
}

#[test]
fn a_failure_report_names_both_versions_and_the_source() {
    let output = std::process::Output {
        status: std::process::ExitStatus::default(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    let report = describe_failure(&target("prettier"), &output);

    assert!(
        report.contains("prettier 1.0.0 → 2.0.0 via npm"),
        "{report}"
    );
    assert!(report.contains("--- stdout ---"), "{report}");
    assert!(report.contains("--- stderr ---"), "{report}");
}

#[test]
fn a_batch_where_every_target_was_blocked_changed_nothing_worth_rechecking() {
    let outcome = Outcome {
        blocked: vec![blocked("alpha", SourceKind::Npm)],
        ..Outcome::default()
    };

    assert!(!outcome.changed_packages());
}

#[test]
fn a_batch_with_an_updated_or_failed_target_changed_packages() {
    let updated = Outcome {
        updated: vec!["alpha".to_string()],
        blocked: vec![blocked("beta", SourceKind::Npm)],
        ..Outcome::default()
    };
    let failed = Outcome {
        failed: vec!["alpha".to_string()],
        ..Outcome::default()
    };

    assert!(updated.changed_packages());
    assert!(failed.changed_packages());
}

#[test]
fn auto_approval_retries_every_blocked_target_when_the_setting_is_on() {
    let outcome = Outcome {
        blocked: vec![
            blocked("alpha", SourceKind::Npm),
            blocked("beta", SourceKind::Npm),
        ],
        ..Outcome::default()
    };

    assert_eq!(auto_approvals(&outcome, true), outcome.blocked);
}

#[test]
fn auto_approval_leaves_blocked_targets_for_the_dialog_when_the_setting_is_off() {
    let outcome = Outcome {
        blocked: vec![blocked("alpha", SourceKind::Npm)],
        ..Outcome::default()
    };

    assert!(auto_approvals(&outcome, false).is_empty());
}

#[test]
fn a_retry_that_is_blocked_again_goes_to_the_dialog_instead_of_looping() {
    let outcome = Outcome {
        blocked: vec![blocked("alpha", SourceKind::Npm)],
        retried: true,
        ..Outcome::default()
    };

    assert!(auto_approvals(&outcome, true).is_empty());
}
