use std::fs;
use std::path::PathBuf;

use crate::model::{Package, Status};
use crate::platform;

const SNAPSHOT: &str = "last-check.txt";
const FAILURES: &str = "last-run.log";
const SELF_UPDATE_FAILURES: &str = "self-update.log";

pub fn log_path() -> PathBuf {
    platform::data_dir().join(FAILURES)
}

pub fn self_update_log_path() -> PathBuf {
    platform::data_dir().join(SELF_UPDATE_FAILURES)
}

pub fn record_failures(report: &str) {
    fs::write(log_path(), report).ok();
}

pub fn record_self_update_failure(report: &str) {
    fs::write(self_update_log_path(), report).ok();
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
mod tests;
