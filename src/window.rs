use std::cmp::Reverse;

use serde::Serialize;

use crate::config::Config;
use crate::model::{Activity, Batch, Blocked, Package, RowState, SourceKind, Status, KINDS};
use crate::tray::{self, SelfUpdate, View};

#[cfg(windows)]
mod shell;
#[cfg(not(windows))]
mod stub;

#[cfg(windows)]
pub use shell::Window;
#[cfg(not(windows))]
pub use stub::Window;

#[cfg_attr(not(windows), allow(dead_code))]
pub const TITLE: &str = "Globlin";
#[cfg_attr(not(windows), allow(dead_code))]
pub const WIDTH: f64 = 960.0;
#[cfg_attr(not(windows), allow(dead_code))]
pub const HEIGHT: f64 = 640.0;

#[cfg_attr(not(windows), allow(dead_code))]
const UI: &str = include_str!("window/ui.html");

#[derive(Debug, Serialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Snapshot {
    pub headline: String,
    pub busy: bool,
    pub version: &'static str,
    pub autostart: bool,
    pub auto_update: bool,
    pub auto_approve: bool,
    pub self_update: Option<String>,
    pub managed_by: Option<&'static str>,
    pub sources: Vec<SourceRow>,
    pub packages: Vec<Row>,
    pub batch: Vec<BatchRow>,
    pub approvals: Vec<ApprovalRow>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Row {
    pub key: String,
    pub name: String,
    pub source: &'static str,
    pub current: String,
    pub latest: Option<String>,
    pub status: &'static str,
    pub read_only: bool,
    pub removable: bool,
    pub update_id: String,
    pub ignore_id: String,
    pub remove_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SourceRow {
    pub label: &'static str,
    pub id: String,
    pub enabled: bool,
    pub read_only: bool,
    pub total: usize,
    pub outdated: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BatchRow {
    pub text: String,
    pub state: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ApprovalRow {
    pub name: String,
    pub source: &'static str,
    pub from: String,
    pub to: String,
    pub scripts: Vec<ScriptRow>,
    pub approve_id: String,
    pub dismiss_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ScriptRow {
    pub package: String,
    pub scripts: String,
}

pub fn snapshot(view: &View, config: &Config) -> Snapshot {
    let (self_update, auto_update, managed_by) = match view.self_update {
        SelfUpdate::Winget => (None, false, Some("winget")),
        SelfUpdate::Own {
            release,
            auto_update,
            ..
        } => (
            release.map(|release| release.version.to_string()),
            auto_update,
            None,
        ),
    };
    let sources = source_rows(view.packages, config);
    Snapshot {
        headline: tray::headline(view),
        busy: view.activity.is_some(),
        version: env!("CARGO_PKG_VERSION"),
        autostart: view.autostart,
        auto_update,
        auto_approve: config.auto_approve_scripts,
        self_update,
        managed_by,
        packages: package_rows(view.packages, &sources),
        sources,
        batch: batch_rows(view),
        approvals: view.approvals.iter().map(approval_row).collect(),
    }
}

fn package_rows(packages: &[Package], sources: &[SourceRow]) -> Vec<Row> {
    let mut ordered: Vec<&Package> = packages.iter().collect();
    ordered.sort_by_key(|package| {
        (
            sources
                .iter()
                .position(|source| source.label == package.source.label()),
            !matches!(package.status, Status::Outdated { .. }),
        )
    });
    ordered.into_iter().map(row).collect()
}

fn row(package: &Package) -> Row {
    Row {
        key: tray::key_of(package),
        name: package.name.clone(),
        source: package.source.label(),
        current: package.current.clone(),
        latest: package.latest().map(ToString::to_string),
        status: package.status.label(),
        read_only: package.source.read_only(),
        removable: package.source.removable(),
        update_id: tray::update_id(package),
        ignore_id: tray::ignore_id(package),
        remove_id: tray::remove_id(package),
    }
}

fn source_rows(packages: &[Package], config: &Config) -> Vec<SourceRow> {
    let mut rows: Vec<SourceRow> = KINDS
        .into_iter()
        .map(|kind| source_row(kind, packages, config))
        .collect();
    rows.sort_by_key(|row| {
        (
            !row.enabled,
            row.total == 0,
            Reverse(row.outdated),
            Reverse(row.total),
        )
    });
    rows
}

fn source_row(kind: SourceKind, packages: &[Package], config: &Config) -> SourceRow {
    let mine: Vec<&Package> = packages
        .iter()
        .filter(|package| package.source == kind)
        .collect();
    SourceRow {
        label: kind.label(),
        id: tray::source_id(kind),
        enabled: config.source_enabled(kind),
        read_only: kind.read_only(),
        total: mine.len(),
        outdated: mine
            .iter()
            .filter(|package| matches!(package.status, Status::Outdated { .. }))
            .count(),
    }
}

fn batch_rows(view: &View) -> Vec<BatchRow> {
    let Some(Activity::Updating { batch }) = view.activity else {
        return Vec::new();
    };
    (0..batch.total())
        .filter_map(|position| {
            Some(BatchRow {
                text: tray::batch_row_text(view, position)?,
                state: state_label(batch, position),
            })
        })
        .collect()
}

fn approval_row(blocked: &Blocked) -> ApprovalRow {
    ApprovalRow {
        name: blocked.target.name.clone(),
        source: blocked.target.source.label(),
        from: blocked.target.from.clone(),
        to: blocked.target.to.clone(),
        scripts: blocked
            .scripts
            .iter()
            .map(|script| ScriptRow {
                package: script.package(),
                scripts: script.scripts.clone(),
            })
            .collect(),
        approve_id: tray::approve_id(&blocked.target),
        dismiss_id: tray::dismiss_id(&blocked.target),
    }
}

fn state_label(batch: &Batch, position: usize) -> &'static str {
    match batch.state_of(position) {
        RowState::Done => "done",
        RowState::Failed => "failed",
        RowState::Active => "active",
        RowState::Queued => "queued",
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Tick {
    pub headline: String,
    pub busy: bool,
    pub batch: Vec<BatchRow>,
}

pub fn tick(view: &View) -> Tick {
    Tick {
        headline: tray::headline(view),
        busy: view.activity.is_some(),
        batch: batch_rows(view),
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn payload(snapshot: &Snapshot) -> String {
    serde_json::to_string(snapshot).unwrap_or_else(|_| "null".to_string())
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn script(snapshot: &Snapshot) -> String {
    format!("window.globlin.render({})", payload(snapshot))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn tick_script(tick: &Tick) -> String {
    let body = serde_json::to_string(tick).unwrap_or_else(|_| "null".to_string());
    format!("window.globlin.tick({body})")
}

#[cfg(test)]
mod tests;
