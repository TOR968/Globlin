use std::cmp::Reverse;

use serde::Serialize;

use crate::config::Config;
use crate::model::{Activity, Batch, Blocked, Package, RowState, SourceKind, Status, KINDS};
use crate::tray::{self, SelfUpdate, View};

mod shell;

pub use shell::{show_in_dock, Window};

pub const TITLE: &str = "Globlin";
pub const WIDTH: f64 = 960.0;
pub const HEIGHT: f64 = 640.0;

const UI: &str = include_str!("window/ui.html");
const OWN_PAGE_BLANK: &str = "about:blank";
const OWN_PAGE_DATA: &str = "data:text/html";

#[derive(Debug, Serialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Snapshot {
    pub headline: String,
    pub busy: bool,
    pub version: &'static str,
    pub autostart: bool,
    pub auto_update: bool,
    pub auto_approve: bool,
    pub choco_one_prompt: Option<bool>,
    pub winget_one_prompt: Option<bool>,
    pub monochrome_icon: bool,
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
    pub elevation: Option<Elevation>,
    pub update_id: String,
    pub ignore_id: String,
    pub remove_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Elevation {
    pub label: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SourceRow {
    pub label: &'static str,
    pub id: String,
    pub enabled: bool,
    pub elevation: Option<Elevation>,
    pub total: usize,
    pub outdated: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BatchRow {
    pub name: String,
    pub source: &'static str,
    pub from: String,
    pub to: String,
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
    snapshot_on(view, config, std::env::consts::OS)
}

fn snapshot_on(view: &View, config: &Config, os: &str) -> Snapshot {
    let (self_update, auto_update, managed_by) = match view.self_update {
        SelfUpdate::Managed(manager) => (None, false, Some(manager.label())),
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
    let sources = source_rows(view.packages, config, os);
    Snapshot {
        headline: tray::headline(view),
        busy: view.activity.is_some(),
        version: env!("CARGO_PKG_VERSION"),
        autostart: view.autostart,
        auto_update,
        auto_approve: config.auto_approve_scripts,
        choco_one_prompt: SourceKind::Choco
            .available_on(os)
            .then_some(config.choco_one_prompt),
        winget_one_prompt: SourceKind::Winget
            .available_on(os)
            .then_some(config.winget_one_prompt),
        monochrome_icon: config.monochrome_icon,
        self_update,
        managed_by,
        packages: package_rows(view.packages, &sources, config),
        sources,
        batch: batch_rows(view),
        approvals: view.approvals.iter().map(approval_row).collect(),
    }
}

fn package_rows(packages: &[Package], sources: &[SourceRow], config: &Config) -> Vec<Row> {
    let mut ordered: Vec<&Package> = packages.iter().collect();
    ordered.sort_by_key(|package| {
        (
            sources
                .iter()
                .position(|source| source.label == package.source.label()),
            !matches!(package.status, Status::Outdated { .. }),
        )
    });
    ordered
        .into_iter()
        .map(|package| row(package, config))
        .collect()
}

fn row(package: &Package, config: &Config) -> Row {
    Row {
        key: tray::key_of(package),
        name: package.name.clone(),
        source: package.source.label(),
        current: package.current.clone(),
        latest: package.latest().map(ToString::to_string),
        status: package.status.label(),
        elevation: elevation(package.source, config),
        update_id: tray::update_id(package),
        ignore_id: tray::ignore_id(package),
        remove_id: tray::remove_id(package),
    }
}

fn source_rows(packages: &[Package], config: &Config, os: &str) -> Vec<SourceRow> {
    let mut rows: Vec<SourceRow> = KINDS
        .into_iter()
        .filter(|kind| kind.available_on(os))
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
        elevation: elevation(kind, config),
        total: mine.len(),
        outdated: mine
            .iter()
            .filter(|package| matches!(package.status, Status::Outdated { .. }))
            .count(),
    }
}

fn elevation(kind: SourceKind, config: &Config) -> Option<Elevation> {
    match kind {
        SourceKind::Winget if config.winget_one_prompt => Some(Elevation {
            label: "asks for admin",
            detail: "Windows asks once for all the winget updates in a batch, and each one then runs as administrator. An uninstall still runs as you.",
        }),
        SourceKind::Winget => Some(Elevation {
            label: "may ask for admin",
            detail: "winget runs as you. A package installed for you alone updates without asking; one installed for the whole machine shows its own Windows administrator prompt.",
        }),
        SourceKind::Choco if config.choco_one_prompt => Some(Elevation {
            label: "asks for admin",
            detail: "Chocolatey needs an administrator for every change. Windows asks once for all the choco updates in a batch, and once for each uninstall.",
        }),
        SourceKind::Choco => Some(Elevation {
            label: "asks for admin",
            detail: "Chocolatey needs an administrator for every change, so Windows asks once for each choco update or uninstall.",
        }),
        _ => None,
    }
}

fn batch_rows(view: &View) -> Vec<BatchRow> {
    let Some(Activity::Updating { batch }) = view.activity else {
        return Vec::new();
    };
    batch
        .targets
        .iter()
        .enumerate()
        .map(|(position, target)| BatchRow {
            name: target.name.clone(),
            source: target.source.label(),
            from: target.from.clone(),
            to: target.to.clone(),
            state: state_label(batch, position),
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
        RowState::Skipped => "skipped",
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

pub fn is_own_page(uri: &str) -> bool {
    uri == OWN_PAGE_BLANK || uri.starts_with(OWN_PAGE_DATA)
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
