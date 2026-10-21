use super::*;
use crate::config::Sources;
use crate::model::{BlockedScript, Installed, SourceKind};
use std::sync::Mutex;
use std::time::Duration;

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
        let dir = std::env::temp_dir().join(format!("globlin-fake-npm-{label}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("npm");
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
                    brew: false,
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
fn a_target_that_cannot_be_started_finishes_as_failed() {
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
            state: RowState::Failed
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
            brew: false,
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
    assert_eq!(outcome.updated, targets);
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
        let dir = std::env::temp_dir().join(format!("globlin-fake-npm-run-{label}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("npm.cmd");
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
                    brew: false,
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
        updated: vec![blocked("alpha", SourceKind::Npm).target],
        ..Outcome::default()
    };

    settle(&mut approvals, &outcome);

    assert!(approvals.is_empty());
}

#[test]
fn updating_a_namesake_from_another_source_keeps_the_approval() {
    let mut approvals = vec![blocked("alpha", SourceKind::Npm)];
    let outcome = Outcome {
        updated: vec![blocked("alpha", SourceKind::Pnpm).target],
        ..Outcome::default()
    };

    settle(&mut approvals, &outcome);

    assert_eq!(approvals, vec![blocked("alpha", SourceKind::Npm)]);
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

fn outdated(name: &str, source: SourceKind) -> Package {
    Package {
        name: name.to_string(),
        current: "1.0.0".to_string(),
        source,
        status: Status::Outdated {
            latest: "2.0.0".to_string(),
        },
    }
}

#[test]
fn an_updated_package_reads_current_at_its_new_version_before_the_recheck_returns() {
    let mut packages = vec![outdated("alpha", SourceKind::Npm)];
    let outcome = Outcome {
        updated: vec![target("alpha")],
        ..Outcome::default()
    };

    mark_updated(&mut packages, &outcome);

    assert_eq!(packages[0].current, "2.0.0");
    assert_eq!(packages[0].status, Status::Current);
}

#[test]
fn a_failed_or_blocked_package_stays_outdated_until_the_recheck() {
    let mut packages = vec![
        outdated("alpha", SourceKind::Npm),
        outdated("beta", SourceKind::Npm),
    ];
    let outcome = Outcome {
        failed: vec!["alpha".to_string()],
        blocked: vec![blocked("beta", SourceKind::Npm)],
        ..Outcome::default()
    };

    mark_updated(&mut packages, &outcome);

    assert_eq!(
        packages,
        vec![
            outdated("alpha", SourceKind::Npm),
            outdated("beta", SourceKind::Npm)
        ]
    );
}

#[test]
fn updating_a_package_leaves_its_namesake_from_another_source_outdated() {
    let mut packages = vec![
        outdated("alpha", SourceKind::Npm),
        outdated("alpha", SourceKind::Bun),
    ];
    let outcome = Outcome {
        updated: vec![target("alpha")],
        ..Outcome::default()
    };

    mark_updated(&mut packages, &outcome);

    assert_eq!(packages[0].status, Status::Current);
    assert_eq!(packages[1], outdated("alpha", SourceKind::Bun));
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
        updated: vec![blocked("alpha", SourceKind::Npm).target],
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

fn sourced(name: &str, source: SourceKind) -> UpdateTarget {
    UpdateTarget {
        source,
        ..target(name)
    }
}

#[test]
fn with_one_prompt_every_choco_target_joins_the_first_choco_job() {
    let targets = vec![
        sourced("git", SourceKind::Choco),
        target("prettier"),
        sourced("nodejs", SourceKind::Choco),
        sourced("7zip", SourceKind::Choco),
    ];

    assert_eq!(
        jobs(&targets, &[SourceKind::Choco]),
        vec![vec![0, 2, 3], vec![1]]
    );
}

#[test]
fn without_one_prompt_every_target_is_its_own_job() {
    let targets = vec![
        sourced("git", SourceKind::Choco),
        target("prettier"),
        sourced("nodejs", SourceKind::Choco),
    ];

    assert_eq!(jobs(&targets, &[]), vec![vec![0], vec![1], vec![2]]);
}

#[test]
fn one_prompt_never_groups_targets_from_other_sources() {
    let targets = vec![target("prettier"), target("vite")];

    assert_eq!(
        jobs(&targets, &[SourceKind::Choco, SourceKind::Winget]),
        vec![vec![0], vec![1]]
    );
}

#[test]
fn with_both_one_prompt_switches_winget_and_choco_each_get_their_own_shared_job() {
    let targets = vec![
        sourced("Git.Git", SourceKind::Winget),
        sourced("git", SourceKind::Choco),
        target("prettier"),
        sourced("GitHub.cli", SourceKind::Winget),
        sourced("nodejs", SourceKind::Choco),
    ];

    assert_eq!(
        jobs(&targets, &[SourceKind::Choco, SourceKind::Winget]),
        vec![vec![0, 3], vec![1, 4], vec![2]]
    );
}

fn refused(refusal: Refusal) -> Result<(), Failure> {
    Err(Failure {
        details: format!("{refusal:?}\n"),
        blocked: None,
        refusal: Some(refusal),
    })
}

fn driven(
    targets: &[UpdateTarget],
    attempts: Vec<Result<(), Failure>>,
) -> (Outcome, Vec<Step>, usize) {
    let attempts = Mutex::new(attempts.into_iter());
    let calls = Mutex::new(0);
    let seen = Mutex::new(Vec::new());
    let jobs = (0..targets.len()).map(|index| vec![index]).collect();
    let outcome = drive(
        targets,
        jobs,
        |_| {
            *calls.lock().unwrap() += 1;
            vec![attempts.lock().unwrap().next().unwrap_or(Ok(()))]
        },
        &|step| seen.lock().unwrap().push(step),
        Duration::ZERO,
    );
    let calls = *calls.lock().unwrap();
    (outcome, seen.into_inner().unwrap(), calls)
}

#[test]
fn an_update_refused_because_another_installer_was_running_is_tried_again_at_the_end() {
    let targets = vec![target("Epic"), target("Gh")];

    let (outcome, steps, calls) = driven(&targets, vec![refused(Refusal::Busy), Ok(()), Ok(())]);

    assert_eq!(calls, 3);
    assert_eq!(outcome.updated, vec![target("Gh"), target("Epic")]);
    assert!(outcome.failed.is_empty());
    assert!(matches!(
        steps.last(),
        Some(Step::Finished {
            index: 0,
            state: RowState::Done
        })
    ));
}

#[test]
fn a_target_still_busy_on_its_second_try_fails_instead_of_retrying_forever() {
    let (outcome, _, calls) = driven(
        &[target("Epic")],
        vec![refused(Refusal::Busy), refused(Refusal::Busy)],
    );

    assert_eq!(calls, 2);
    assert_eq!(outcome.failed, vec!["Epic".to_string()]);
}

#[test]
fn a_busy_target_is_left_queued_until_its_retry_rather_than_shown_as_failed() {
    let (_, steps, _) = driven(&[target("Epic")], vec![refused(Refusal::Busy), Ok(())]);

    let finishes: Vec<&Step> = steps
        .iter()
        .filter(|step| matches!(step, Step::Finished { .. }))
        .collect();
    assert_eq!(finishes.len(), 1);
}

#[test]
fn a_portable_package_whose_program_is_running_is_skipped_not_failed() {
    let (outcome, steps, calls) = driven(
        &[target("ClaudeCodeUsageMonitor")],
        vec![refused(Refusal::InUse)],
    );

    assert_eq!(calls, 1);
    assert!(outcome.failed.is_empty());
    assert_eq!(
        outcome.skipped,
        vec![Skipped {
            name: "ClaudeCodeUsageMonitor".to_string(),
            reason: Refusal::InUse
        }]
    );
    assert!(matches!(
        steps.last(),
        Some(Step::Finished {
            index: 0,
            state: RowState::Skipped
        })
    ));
}

#[test]
fn an_update_that_does_not_apply_or_whose_prompt_was_cancelled_is_skipped_not_failed() {
    let targets = vec![target("Docker"), target("LocalSend")];

    let (outcome, steps, _) = driven(
        &targets,
        vec![refused(Refusal::NotApplicable), refused(Refusal::Cancelled)],
    );

    assert!(outcome.failed.is_empty());
    assert_eq!(
        outcome.skipped,
        vec![
            Skipped {
                name: "Docker".to_string(),
                reason: Refusal::NotApplicable
            },
            Skipped {
                name: "LocalSend".to_string(),
                reason: Refusal::Cancelled
            },
        ]
    );
    assert!(steps.iter().any(|step| matches!(
        step,
        Step::Finished {
            index: 1,
            state: RowState::Skipped
        }
    )));
}

#[test]
fn a_batch_that_only_skipped_targets_changed_nothing_worth_rechecking() {
    let outcome = Outcome {
        skipped: vec![Skipped {
            name: "Docker".to_string(),
            reason: Refusal::NotApplicable,
        }],
        ..Outcome::default()
    };

    assert!(!outcome.changed_packages());
}

#[test]
fn a_failure_outranks_everything_else_in_the_notification() {
    let outcome = Outcome {
        updated: vec![target("vite")],
        failed: vec!["Obsidian.Obsidian".to_string()],
        ..Outcome::default()
    };

    let (title, body) = toast(&outcome).unwrap();
    assert_eq!(title, "Globlin — update failed");
    assert_eq!(body, "Obsidian.Obsidian (see Open last log)");
}

#[test]
fn skipped_targets_are_named_with_their_reason_beside_the_updated_ones() {
    let outcome = Outcome {
        updated: vec![sourced("GitHub.cli", SourceKind::Winget)],
        skipped: vec![Skipped {
            name: "Docker.DockerDesktop".to_string(),
            reason: Refusal::NotApplicable,
        }],
        ..Outcome::default()
    };

    let (title, body) = toast(&outcome).unwrap();
    assert_eq!(title, "Globlin — updated");
    assert_eq!(
        body,
        "GitHub.cli (winget)
Docker.DockerDesktop — no update applies to this system"
    );
}

#[test]
fn a_batch_that_only_skipped_says_nothing_was_updated() {
    let outcome = Outcome {
        skipped: vec![Skipped {
            name: "LocalSend.LocalSend".to_string(),
            reason: Refusal::Cancelled,
        }],
        ..Outcome::default()
    };

    assert_eq!(
        toast(&outcome).map(|(title, _)| title),
        Some("Globlin — nothing updated")
    );
}

#[test]
fn a_batch_with_nothing_to_report_raises_no_notification() {
    assert_eq!(toast(&Outcome::default()), None);
}

#[test]
fn after_a_shared_run_a_package_counts_as_updated_only_if_its_version_moved() {
    let group = vec![
        sourced("git", SourceKind::Choco),
        sourced("nodejs", SourceKind::Choco),
        sourced("7zip", SourceKind::Choco),
    ];
    let listing = vec![
        Installed::new("git", "2.0.0", SourceKind::Choco),
        Installed::new("nodejs", "1.0.0", SourceKind::Choco),
    ];

    assert_eq!(
        changed_versions(&group, Some(&listing), false),
        vec![true, false, false]
    );
}

#[test]
fn when_the_listing_cannot_be_read_the_exit_code_decides_for_the_whole_group() {
    let group = vec![
        sourced("git", SourceKind::Choco),
        sourced("nodejs", SourceKind::Choco),
    ];

    assert_eq!(changed_versions(&group, None, true), vec![true, true]);
    assert_eq!(changed_versions(&group, None, false), vec![false, false]);
}

#[test]
fn a_group_that_could_not_run_logs_its_reason_once_and_fails_every_member() {
    let group = vec![
        sourced("git", SourceKind::Choco),
        sourced("nodejs", SourceKind::Choco),
    ];

    let results = fail_group(&group, "choco: no shell\n".to_string());
    let details: Vec<String> = results
        .into_iter()
        .map(|result| result.err().unwrap().details)
        .collect();

    assert_eq!(
        details,
        vec!["choco: no shell\n".to_string(), String::new()]
    );
}

#[test]
fn in_a_shared_run_every_unchanged_member_carries_the_refusal_the_whole_run_ended_with() {
    let results = group_results(
        vec![false, true, false],
        "winget: not applicable\n".to_string(),
        Some(Refusal::NotApplicable),
    );

    let refusals: Vec<Option<Option<Refusal>>> = results
        .iter()
        .map(|result| result.as_ref().err().map(|failure| failure.refusal))
        .collect();
    assert_eq!(
        refusals,
        vec![
            Some(Some(Refusal::NotApplicable)),
            None,
            Some(Some(Refusal::NotApplicable))
        ]
    );
    assert_eq!(
        results[0].as_ref().err().unwrap().details,
        "winget: not applicable\n"
    );
    assert_eq!(results[2].as_ref().err().unwrap().details, "");
}

#[test]
fn busy_members_of_a_shared_run_are_retried_together_under_one_more_prompt() {
    let targets = vec![
        sourced("Epic", SourceKind::Winget),
        sourced("Gh", SourceKind::Winget),
        sourced("Vortex", SourceKind::Winget),
    ];
    let runs = Mutex::new(Vec::new());

    let outcome = drive(
        &targets,
        vec![vec![0, 1, 2]],
        |job| {
            runs.lock().unwrap().push(job.to_vec());
            if job.len() == 3 {
                vec![refused(Refusal::Busy), Ok(()), refused(Refusal::Busy)]
            } else {
                job.iter().map(|_| Ok(())).collect()
            }
        },
        &|_| {},
        Duration::ZERO,
    );

    assert_eq!(runs.into_inner().unwrap(), vec![vec![0, 1, 2], vec![0, 2]]);
    assert_eq!(outcome.updated.len(), 3);
}
