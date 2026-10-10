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
    truncate(&log_path(), &redacted(report));
}

const SECRET_KEYS: [&str; 4] = ["_authToken=", "_auth=", "_password=", "Authorization:"];
const SECRET_PREFIXES: [&str; 3] = ["npm_", "ghp_", "github_pat_"];
const SECRET_MIN_LENGTH: usize = 30;
const REDACTED: &str = "<redacted>";

fn redacted(report: &str) -> String {
    let mut cleaned = String::with_capacity(report.len());
    for line in report.split_inclusive('\n') {
        match line.strip_suffix('\n') {
            Some(text) => {
                cleaned.push_str(&redact_line(text));
                cleaned.push('\n');
            }
            None => cleaned.push_str(&redact_line(line)),
        }
    }
    cleaned
}

fn redact_line(line: &str) -> String {
    if let Some((key, at)) = SECRET_KEYS
        .iter()
        .filter_map(|key| line.find(key).map(|at| (key, at)))
        .min_by_key(|(_, at)| *at)
    {
        return format!("{}{REDACTED}", &line[..at + key.len()]);
    }
    line.split(' ')
        .map(|word| {
            let token =
                word.trim_matches(|glyph: char| !glyph.is_ascii_alphanumeric() && glyph != '_');
            let secret = token.len() >= SECRET_MIN_LENGTH
                && SECRET_PREFIXES
                    .iter()
                    .any(|prefix| token.starts_with(prefix))
                && token
                    .chars()
                    .all(|glyph| glyph.is_ascii_alphanumeric() || glyph == '_');
            if secret {
                word.replace(token, REDACTED)
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
