use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

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
    truncate(&log_path(), report);
}

pub fn record_note(note: &str) {
    append_once(&log_path(), note);
}

pub fn command_report(action: &str, output: &Output) -> String {
    format!(
        "{action} exited with {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n\n",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn append_once(path: &Path, note: &str) {
    if fs::read_to_string(path).is_ok_and(|log| log.contains(note)) {
        return;
    }
    append(path, note);
}

fn truncate(path: &Path, text: &str) {
    fs::write(path, text).ok();
}

fn append(path: &Path, text: &str) {
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        file.write_all(text.as_bytes()).ok();
    }
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
