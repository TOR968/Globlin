use crate::model::{self, Package};

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Announce {
        title: String,
        body: String,
        stamps: Vec<String>,
    },
    Remember {
        stamps: Vec<String>,
    },
    Nothing,
}

pub fn decide(packages: &[Package], last_notified: &[String]) -> Decision {
    let stamps = model::stamps(packages);
    if stamps == last_notified {
        return Decision::Nothing;
    }
    if stamps.is_empty() {
        return Decision::Remember { stamps };
    }
    Decision::Announce {
        title: title(packages),
        body: body(packages),
        stamps,
    }
}

fn title(packages: &[Package]) -> String {
    match model::outdated(packages).len() {
        1 => "1 npm global update".to_string(),
        count => format!("{count} npm global updates"),
    }
}

fn body(packages: &[Package]) -> String {
    model::outdated(packages)
        .iter()
        .map(|package| {
            format!(
                "{}{}  {} → {}",
                package.name,
                package.source.suffix(),
                package.current,
                package
                    .latest()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SourceKind, Status};
    use semver::Version;

    fn behind(name: &str, current: &str, latest: &str, source: SourceKind) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse(current).unwrap(),
            source,
            status: Status::Outdated {
                latest: Version::parse(latest).unwrap(),
            },
        }
    }

    fn current(name: &str) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse("1.0.0").unwrap(),
            source: SourceKind::Npm,
            status: Status::Current,
        }
    }

    #[test]
    fn an_unchanged_set_of_updates_is_not_announced_again() {
        let packages = vec![behind("prettier", "3.9.6", "3.10.0", SourceKind::Npm)];
        let already = vec!["npm:prettier@3.10.0".to_string()];

        assert_eq!(decide(&packages, &already), Decision::Nothing);
    }

    #[test]
    fn a_first_sighting_is_announced() {
        let packages = vec![behind("prettier", "3.9.6", "3.10.0", SourceKind::Npm)];

        match decide(&packages, &[]) {
            Decision::Announce {
                title,
                body,
                stamps,
            } => {
                assert_eq!(title, "1 npm global update");
                assert_eq!(body, "prettier  3.9.6 → 3.10.0");
                assert_eq!(stamps, vec!["npm:prettier@3.10.0".to_string()]);
            }
            other => panic!("expected an announcement, got {other:?}"),
        }
    }

    #[test]
    fn a_newer_target_version_counts_as_a_change_worth_announcing() {
        let packages = vec![behind("prettier", "3.9.6", "3.11.0", SourceKind::Npm)];
        let already = vec!["npm:prettier@3.10.0".to_string()];

        assert!(matches!(
            decide(&packages, &already),
            Decision::Announce { .. }
        ));
    }

    #[test]
    fn everything_becoming_current_is_remembered_but_not_announced() {
        let packages = vec![current("prettier")];
        let already = vec!["npm:prettier@3.10.0".to_string()];

        assert_eq!(
            decide(&packages, &already),
            Decision::Remember { stamps: Vec::new() }
        );
    }

    #[test]
    fn staying_fully_up_to_date_produces_nothing_at_all() {
        assert_eq!(decide(&[current("prettier")], &[]), Decision::Nothing);
    }

    #[test]
    fn the_title_switches_to_plural_beyond_one_update() {
        let packages = vec![
            behind("a", "1.0.0", "2.0.0", SourceKind::Npm),
            behind("b", "1.0.0", "2.0.0", SourceKind::Npm),
        ];

        match decide(&packages, &[]) {
            Decision::Announce { title, .. } => assert_eq!(title, "2 npm global updates"),
            other => panic!("expected an announcement, got {other:?}"),
        }
    }

    #[test]
    fn the_body_lists_one_package_per_line_with_both_versions() {
        let packages = vec![
            behind("prettier", "3.9.6", "3.10.0", SourceKind::Npm),
            behind("vercel", "58.4.4", "59.0.1", SourceKind::Npm),
        ];

        let body = body(&packages);
        let lines: Vec<&str> = body.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "prettier  3.9.6 → 3.10.0");
        assert_eq!(lines[1], "vercel  58.4.4 → 59.0.1");
    }

    #[test]
    fn a_bun_package_is_labelled_so_a_shared_name_is_unambiguous() {
        let packages = vec![behind("typescript", "5.9.3", "5.10.0", SourceKind::Bun)];

        assert_eq!(body(&packages), "typescript (bun)  5.9.3 → 5.10.0");
    }

    #[test]
    fn current_and_ignored_packages_never_reach_the_notification() {
        let packages = vec![
            behind("prettier", "3.9.6", "3.10.0", SourceKind::Npm),
            current("typescript"),
            Package {
                status: Status::Ignored,
                ..current("npm")
            },
        ];

        assert_eq!(body(&packages), "prettier  3.9.6 → 3.10.0");
    }
}
