use crate::config::Config;
use crate::diagnostics;
use crate::model::UpdateTarget;
use crate::source::{self, PackageSource};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Started {
        target: UpdateTarget,
        index: usize,
        total: usize,
    },
    Finished {
        index: usize,
        ok: bool,
    },
}

pub fn run(config: &Config, targets: &[UpdateTarget], announce: impl Fn(Step)) -> Outcome {
    let sources = match source::enabled(config) {
        Ok(sources) => sources,
        Err(error) => {
            let report = format!("no package source is available: {error}\n");
            diagnostics::record_failures(&report);
            return Outcome {
                updated: Vec::new(),
                failed: targets.iter().map(|target| target.name.clone()).collect(),
            };
        }
    };

    let mut outcome = Outcome::default();
    let mut report = String::new();

    for (index, target) in targets.iter().enumerate() {
        announce(Step::Started {
            target: target.clone(),
            index,
            total: targets.len(),
        });

        match apply(&sources, target) {
            Ok(()) => {
                outcome.updated.push(target.name.clone());
                announce(Step::Finished { index, ok: true });
            }
            Err(details) => {
                outcome.failed.push(target.name.clone());
                report.push_str(&details);
                announce(Step::Finished { index, ok: false });
            }
        }
    }

    if !report.is_empty() {
        diagnostics::record_failures(&report);
    }
    outcome
}

fn apply(
    sources: &[Box<dyn PackageSource>],
    target: &UpdateTarget,
) -> std::result::Result<(), String> {
    let source = sources
        .iter()
        .find(|source| source.kind() == target.source)
        .ok_or_else(|| {
            format!(
                "{}: the {} source is not available\n",
                target.name,
                target.source.label()
            )
        })?;

    let output = source
        .update_command(&target.name)
        .output()
        .map_err(|error| {
            format!(
                "{}: could not start {}: {error}\n",
                target.name,
                target.source.label()
            )
        })?;

    if output.status.success() {
        return Ok(());
    }
    Err(describe_failure(target, &output))
}

fn describe_failure(target: &UpdateTarget, output: &std::process::Output) -> String {
    format!(
        "{} {} → {} via {} exited with {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n\n",
        target.name,
        target.from,
        target.to,
        target.source.label(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Sources;
    use crate::model::SourceKind;
    use semver::Version;
    use std::sync::Mutex;

    fn target(name: &str) -> UpdateTarget {
        UpdateTarget {
            name: name.to_string(),
            source: SourceKind::Npm,
            from: Version::parse("1.0.0").unwrap(),
            to: Version::parse("2.0.0").unwrap(),
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
        let outcome = run(&Config::default(), &[], |step| {
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

        run(&fake.config, &targets, |step| {
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

        run(&fake.config, &[target("alpha")], |step| {
            seen.lock().unwrap().push(step);
        });

        let seen = seen.lock().unwrap();
        let Step::Started { target, .. } = &seen[0] else {
            panic!("the first step should be a start: {:?}", seen[0]);
        };
        assert_eq!(target.from, Version::parse("1.0.0").unwrap());
        assert_eq!(target.to, Version::parse("2.0.0").unwrap());
    }

    #[test]
    fn a_target_that_cannot_be_started_finishes_with_ok_false() {
        let fake = UnrunnableNpm::new("finishes-false");
        let seen = Mutex::new(Vec::new());

        run(&fake.config, &[target("alpha")], |step| {
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

        let outcome = run(&fake.config, &[target("alpha")], |_| {});

        assert_eq!(outcome.failed, vec!["alpha".to_string()]);
        assert!(outcome.updated.is_empty());
    }

    #[test]
    fn a_failing_target_does_not_stop_the_ones_behind_it() {
        let fake = UnrunnableNpm::new("continues");
        let targets = vec![target("alpha"), target("beta")];

        let outcome = run(&fake.config, &targets, |_| {});

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
            },
            ..Default::default()
        };
        let bun_target = UpdateTarget {
            source: SourceKind::Bun,
            ..target("opencode-ai")
        };

        let outcome = run(&config, &[bun_target], |_| {});

        assert_eq!(outcome.failed, vec!["opencode-ai".to_string()]);
    }

    #[test]
    #[ignore = "installs for real: cargo test -- --ignored --exact update::tests::updates_a_package_for_real"]
    fn updates_a_package_for_real() {
        let name = std::env::var("UPDATE_TARGET")
            .expect("set UPDATE_TARGET to the package to install at @latest");
        let targets = vec![target(&name)];

        let outcome = run(&Config::default(), &targets, |_| {});

        assert!(outcome.failed.is_empty(), "failed: {:?}", outcome.failed);
        assert_eq!(outcome.updated, vec![name]);
    }

    #[test]
    #[ignore = "spawns npm for real: cargo test -- --ignored --exact update::tests::a_real_npm_failure_lands_in_the_log"]
    fn a_real_npm_failure_lands_in_the_log() {
        let name = "globlin-no-such-package-9d3f".to_string();

        let outcome = run(&Config::default(), &[target(&name)], |_| {});

        assert_eq!(outcome.failed, vec![name.clone()]);
        assert!(outcome.updated.is_empty());
        let log =
            std::fs::read_to_string(diagnostics::log_path()).expect("the failure log should exist");
        assert!(
            log.contains(&name),
            "log did not mention the package: {log}"
        );
    }

    #[test]
    fn a_failure_report_names_both_versions_and_the_source() {
        let output = std::process::Command::new("cmd")
            .args(["/C", "exit 3"])
            .output()
            .unwrap();
        let report = describe_failure(&target("prettier"), &output);

        assert!(
            report.contains("prettier 1.0.0 → 2.0.0 via npm"),
            "{report}"
        );
        assert!(report.contains("--- stdout ---"), "{report}");
        assert!(report.contains("--- stderr ---"), "{report}");
    }
}
