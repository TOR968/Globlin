use serde::Serialize;

use crate::config::Config;
use crate::model::{Activity, Batch, Package, RowState, SourceKind, Status, KINDS};
use crate::tray::{self, SelfUpdate, View};

#[cfg(windows)]
mod shell;
#[cfg(not(windows))]
mod stub;

#[cfg(windows)]
pub use shell::Window;
#[cfg(not(windows))]
pub use stub::Window;

pub const TITLE: &str = "Globlin";
pub const WIDTH: f64 = 960.0;
pub const HEIGHT: f64 = 640.0;

const UI: &str = include_str!("window/ui.html");

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Snapshot {
    pub headline: String,
    pub busy: bool,
    pub version: &'static str,
    pub autostart: bool,
    pub auto_update: bool,
    pub self_update: Option<String>,
    pub managed_by: Option<&'static str>,
    pub sources: Vec<SourceRow>,
    pub packages: Vec<Row>,
    pub batch: Vec<BatchRow>,
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
    Snapshot {
        headline: tray::headline(view),
        busy: view.activity.is_some(),
        version: env!("CARGO_PKG_VERSION"),
        autostart: view.autostart,
        auto_update,
        self_update,
        managed_by,
        sources: source_rows(view.packages, config),
        packages: view.packages.iter().map(row).collect(),
        batch: batch_rows(view),
    }
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
        update_id: tray::update_id(package),
        ignore_id: tray::ignore_id(package),
        remove_id: tray::remove_id(package),
    }
}

fn source_rows(packages: &[Package], config: &Config) -> Vec<SourceRow> {
    KINDS
        .into_iter()
        .map(|kind| source_row(kind, packages, config))
        .collect()
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

pub fn payload(snapshot: &Snapshot) -> String {
    serde_json::to_string(snapshot).unwrap_or_else(|_| "null".to_string())
}

pub fn script(snapshot: &Snapshot) -> String {
    format!("window.globlin.render({})", payload(snapshot))
}

pub fn tick_script(tick: &Tick) -> String {
    let body = serde_json::to_string(tick).unwrap_or_else(|_| "null".to_string());
    format!("window.globlin.tick({body})")
}

#[cfg(test)]
mod tests;
