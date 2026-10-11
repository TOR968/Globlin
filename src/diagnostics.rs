use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{SystemTime, UNIX_EPOCH};

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
    truncate(
        &log_path(),
        &format!("{}\n{}", heading(SystemTime::now()), redacted(report)),
    );
}

const SECONDS_PER_DAY: u64 = 86_400;

fn heading(now: SystemTime) -> String {
    format!(
        "== Globlin {} · {} ==\n",
        env!("CARGO_PKG_VERSION"),
        stamp(now)
    )
}

fn stamp(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (year, month, day) = civil_date(seconds / SECONDS_PER_DAY);
    let time = seconds % SECONDS_PER_DAY;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

fn civil_date(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
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
    append(path, &format!("{}{note}", heading(SystemTime::now())));
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
    fs::write(
        self_update_log_path(),
        format!("{}\n{report}", heading(SystemTime::now())),
    )
    .ok();
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
