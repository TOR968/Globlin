use std::fs;
use std::path::PathBuf;

use crate::model::{Package, Status};
use crate::platform;

const SNAPSHOT: &str = "last-check.txt";
const FAILURES: &str = "last-run.log";

pub fn log_path() -> PathBuf {
    platform::data_dir().join(FAILURES)
}

pub fn record_failures(report: &str) {
    fs::write(log_path(), report).ok();
}

pub fn record_snapshot(packages: &[Package]) {
    let body: String = packages.iter().map(line).collect();
    fs::write(platform::data_dir().join(SNAPSHOT), body).ok();
}

fn line(package: &Package) -> String {
    format!(
        "{}\t{}\t{}\t{}\n",
        package.source.label(),
        package.name,
        package.current,
        describe(&package.status)
    )
}

fn describe(status: &Status) -> String {
    match status {
        Status::Current => "current".to_string(),
        Status::Outdated { latest } => format!("outdated -> {latest}"),
        Status::Unknown => "unknown".to_string(),
        Status::Ignored => "ignored".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceKind;
    use semver::Version;

    fn package(name: &str, status: Status) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse("1.2.3").unwrap(),
            source: SourceKind::Npm,
            status,
        }
    }

    #[test]
    fn a_line_is_tab_separated_and_ends_with_a_newline() {
        let rendered = line(&package("prettier", Status::Current));
        assert_eq!(rendered, "npm\tprettier\t1.2.3\tcurrent\n");
        assert_eq!(rendered.split('\t').count(), 4);
    }

    #[test]
    fn an_outdated_line_carries_the_target_version() {
        let rendered = line(&package(
            "vercel",
            Status::Outdated {
                latest: Version::parse("2.0.0").unwrap(),
            },
        ));
        assert!(rendered.contains("outdated -> 2.0.0"), "{rendered}");
    }

    #[test]
    fn every_status_has_a_distinct_description() {
        let descriptions = [
            describe(&Status::Current),
            describe(&Status::Unknown),
            describe(&Status::Ignored),
            describe(&Status::Outdated {
                latest: Version::parse("2.0.0").unwrap(),
            }),
        ];
        let unique: std::collections::HashSet<&String> = descriptions.iter().collect();
        assert_eq!(unique.len(), descriptions.len());
    }

    #[test]
    fn the_two_diagnostic_files_do_not_collide() {
        assert_ne!(SNAPSHOT, FAILURES);
        assert!(log_path().ends_with(FAILURES));
    }
}
