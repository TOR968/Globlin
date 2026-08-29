use std::time::Duration;

use semver::Version;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

use crate::model::{self, Activity, Batch, Package, RowState, SourceKind, Status, UpdateTarget};
use crate::progress;
use crate::selfupdate::Release;
use crate::Result;

const ID_UPDATE_ALL: &str = "update-all";
const ID_CHECK_NOW: &str = "check-now";
const ID_AUTOSTART: &str = "autostart";
const ID_UPDATE_SELF: &str = "update-self";
const ID_AUTO_UPDATE: &str = "auto-update";
const ID_OPEN_LOG: &str = "open-log";
const ID_OPEN_SELF_LOG: &str = "open-self-log";
const ID_QUIT: &str = "quit";
const UPDATE_PREFIX: &str = "update:";
const IGNORE_PREFIX: &str = "ignore:";
const REMOVE_PREFIX: &str = "remove:";

const DOT_CYCLE: u32 = 4;
const FRAMES_PER_DOT: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Update { name: String, source: SourceKind },
    ToggleIgnore { name: String },
    Remove { name: String, source: SourceKind },
    UpdateAll,
    CheckNow,
    ToggleAutostart,
    UpdateSelf,
    ToggleAutoUpdate,
    OpenLog,
    OpenSelfLog,
    Quit,
}

pub struct Built {
    pub menu: Menu,
    pub header: MenuItem,
    pub rows: Vec<MenuItem>,
}

pub struct View<'a> {
    pub packages: &'a [Package],
    pub activity: Option<&'a Activity>,
    pub autostart: bool,
    pub auto_update: bool,
    pub release: Option<&'a Release>,
    pub pending_restart: Option<&'a Version>,
    pub self_log: bool,
    pub frame: u32,
    pub elapsed: Duration,
}

impl Action {
    pub fn from_id(id: &MenuId) -> Option<Self> {
        Self::from_key(id.as_ref())
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            ID_UPDATE_ALL => Some(Self::UpdateAll),
            ID_CHECK_NOW => Some(Self::CheckNow),
            ID_AUTOSTART => Some(Self::ToggleAutostart),
            ID_UPDATE_SELF => Some(Self::UpdateSelf),
            ID_AUTO_UPDATE => Some(Self::ToggleAutoUpdate),
            ID_OPEN_LOG => Some(Self::OpenLog),
            ID_OPEN_SELF_LOG => Some(Self::OpenSelfLog),
            ID_QUIT => Some(Self::Quit),
            other => Self::parse_update(other)
                .or_else(|| Self::parse_ignore(other))
                .or_else(|| Self::parse_remove(other)),
        }
    }

    fn parse_update(key: &str) -> Option<Self> {
        let (label, name) = key.strip_prefix(UPDATE_PREFIX)?.split_once(':')?;
        Some(Self::Update {
            name: name.to_string(),
            source: SourceKind::from_label(label)?,
        })
    }

    fn parse_ignore(key: &str) -> Option<Self> {
        let (label, name) = key.strip_prefix(IGNORE_PREFIX)?.split_once(':')?;
        SourceKind::from_label(label)?;
        Some(Self::ToggleIgnore {
            name: name.to_string(),
        })
    }

    fn parse_remove(key: &str) -> Option<Self> {
        let (label, name) = key.strip_prefix(REMOVE_PREFIX)?.split_once(':')?;
        Some(Self::Remove {
            name: name.to_string(),
            source: SourceKind::from_label(label)?,
        })
    }
}

struct Sections {
    menu: Menu,
    pending: bool,
}

impl Sections {
    fn new() -> Self {
        Self {
            menu: Menu::new(),
            pending: false,
        }
    }

    fn item(&mut self, item: &dyn IsMenuItem) -> Result<()> {
        self.menu.append(item)?;
        self.pending = true;
        Ok(())
    }

    fn split(&mut self) -> Result<()> {
        if self.pending {
            self.menu.append(&PredefinedMenuItem::separator())?;
            self.pending = false;
        }
        Ok(())
    }
}

pub fn build(view: &View) -> Result<Built> {
    let header = MenuItem::new(headline(view), false, None);
    let busy = view.activity.is_some();
    let outdated = model::outdated(view.packages);
    let batch = active_batch(view.activity);

    let mut built = Sections::new();
    built.item(&header)?;
    built.split()?;

    let mut rows = Vec::new();
    if let Some(batch) = batch {
        for position in 0..batch.total() {
            let text = batch_row_text(view, position).unwrap_or_default();
            let item = MenuItem::new(text, false, None);
            built.item(&item)?;
            rows.push(item);
        }
    }
    built.split()?;

    for package in &outdated {
        if in_batch(batch, package) {
            continue;
        }
        built.item(&package_entry(package, busy)?)?;
    }
    built.split()?;

    let settled: Vec<&Package> = view
        .packages
        .iter()
        .filter(|package| package.latest().is_none())
        .collect();
    for package in &settled {
        built.item(&package_entry(package, busy)?)?;
    }
    built.split()?;

    if !outdated.is_empty() {
        built.item(&MenuItem::with_id(
            ID_UPDATE_ALL,
            format!("Update all ({})", outdated.len()),
            !busy,
            None,
        ))?;
    }
    built.item(&MenuItem::with_id(ID_CHECK_NOW, "Check now", !busy, None))?;
    built.item(&CheckMenuItem::with_id(
        ID_AUTOSTART,
        "Run at startup",
        true,
        view.autostart,
        None,
    ))?;
    built.item(&MenuItem::with_id(ID_OPEN_LOG, "Open last log", true, None))?;
    built.split()?;

    built.item(&self_block(view, busy)?)?;
    built.split()?;
    built.item(&MenuItem::with_id(ID_QUIT, "Quit", true, None))?;

    Ok(Built {
        menu: built.menu,
        header,
        rows,
    })
}

fn in_batch(batch: Option<&Batch>, package: &Package) -> bool {
    batch.is_some_and(|batch| {
        batch
            .targets
            .iter()
            .any(|target| target.name == package.name && target.source == package.source)
    })
}

pub fn batch_row_text(view: &View, position: usize) -> Option<String> {
    let batch = active_batch(view.activity)?;
    let target = batch.targets.get(position)?;
    Some(match batch.state_of(position) {
        RowState::Done => format!("{}   done", target_label(target, '✓')),
        RowState::Failed => format!("{}   failed", target_label(target, '✗')),
        RowState::Queued => format!("{}   queued", target_label(target, '·')),
        RowState::Active => format!(
            "{}   {}",
            target_label(target, spinner_tick(view.frame)),
            progress::bar(progress::working(view.elapsed))
        ),
    })
}

fn active_batch(activity: Option<&Activity>) -> Option<&Batch> {
    match activity {
        Some(Activity::Updating { batch }) => Some(batch),
        _ => None,
    }
}

fn target_label(target: &UpdateTarget, marker: char) -> String {
    format!(
        "{marker}  {}{}   {} → {}",
        target.name,
        target.source.suffix(),
        target.from,
        target.to
    )
}

fn self_block(view: &View, busy: bool) -> Result<Submenu> {
    let entry = Submenu::new(format!("Globlin v{}", env!("CARGO_PKG_VERSION")), true);
    if let Some(release) = view.release {
        entry.append(&MenuItem::with_id(
            ID_UPDATE_SELF,
            self_update_text(release),
            !busy,
            None,
        ))?;
    }
    if let Some(version) = view.pending_restart {
        entry.append(&MenuItem::new(
            format!("Restart to finish the update to {version}"),
            false,
            None,
        ))?;
    }
    entry.append(&PredefinedMenuItem::separator())?;
    entry.append(&CheckMenuItem::with_id(
        ID_AUTO_UPDATE,
        "Auto-update Globlin",
        true,
        view.auto_update,
        None,
    ))?;
    if view.self_log {
        entry.append(&MenuItem::with_id(
            ID_OPEN_SELF_LOG,
            "Open Globlin-update log",
            true,
            None,
        ))?;
    }
    Ok(entry)
}

fn self_update_text(release: &Release) -> String {
    format!(
        "Update Globlin {} → {}",
        env!("CARGO_PKG_VERSION"),
        release.version
    )
}

pub fn headline(view: &View) -> String {
    match view.activity {
        Some(Activity::Checking) => format!("Checking for updates{}", dots(view.frame)),
        Some(Activity::Updating { batch }) => match batch.current() {
            Some(target) => {
                let progress = if batch.total() > 1 {
                    format!("  [{}/{}]", batch.index + 1, batch.total())
                } else {
                    String::new()
                };
                format!(
                    "Updating {} {} → {}{progress}{}",
                    target.name,
                    target.from,
                    target.to,
                    dots(view.frame)
                )
            }
            None => format!("Updating packages{}", dots(view.frame)),
        },
        Some(Activity::Removing { target }) => format!(
            "Removing {}{}{}",
            target.name,
            target.source.suffix(),
            dots(view.frame)
        ),
        Some(Activity::SelfUpdate) => {
            format!("Updating Globlin{}", dots(view.frame))
        }
        None => summary(view.packages),
    }
}

fn summary(packages: &[Package]) -> String {
    if packages.is_empty() {
        return "Globlin — nothing found".to_string();
    }
    match model::outdated(packages).len() {
        0 => format!("Globlin — {} packages, up to date", packages.len()),
        1 => "Globlin — 1 update available".to_string(),
        count => format!("Globlin — {count} updates available"),
    }
}

fn dots(frame: u32) -> String {
    ".".repeat((frame / FRAMES_PER_DOT % DOT_CYCLE) as usize)
}

fn update_id(package: &Package) -> String {
    format!("{UPDATE_PREFIX}{}:{}", package.source.label(), package.name)
}

fn ignore_id(package: &Package) -> String {
    format!("{IGNORE_PREFIX}{}:{}", package.source.label(), package.name)
}

fn remove_id(package: &Package) -> String {
    format!("{REMOVE_PREFIX}{}:{}", package.source.label(), package.name)
}

fn offers_update(package: &Package) -> bool {
    package.latest().is_some()
}

fn package_entry(package: &Package, busy: bool) -> Result<Submenu> {
    let entry = Submenu::new(row(package), !busy);
    if offers_update(package) {
        entry.append(&MenuItem::with_id(
            update_id(package),
            "Update",
            !busy,
            None,
        ))?;
    }
    entry.append(&CheckMenuItem::with_id(
        ignore_id(package),
        "Ignore",
        !busy,
        package.status == Status::Ignored,
        None,
    ))?;
    let uninstall = Submenu::new("Uninstall", !busy);
    uninstall.append(&MenuItem::with_id(
        remove_id(package),
        format!("Confirm — remove {}", package.name),
        !busy,
        None,
    ))?;
    entry.append(&uninstall)?;
    Ok(entry)
}

fn row(package: &Package) -> String {
    match &package.status {
        Status::Outdated { latest } => format!(
            "↑  {}{}   {} → {latest}",
            package.name,
            package.source.suffix(),
            package.current
        ),
        Status::Current => format!(
            "✓  {}{}   {}",
            package.name,
            package.source.suffix(),
            package.current
        ),
        Status::Unknown => format!(
            "?  {}{}   {}   (not checked)",
            package.name,
            package.source.suffix(),
            package.current
        ),
        Status::Ignored => format!(
            "·  {}{}   {}   (ignored)",
            package.name,
            package.source.suffix(),
            package.current
        ),
    }
}

const fn spinner_tick(frame: u32) -> char {
    const TICKS: [char; 4] = ['◐', '◓', '◑', '◒'];
    TICKS[frame as usize % TICKS.len()]
}

#[cfg(test)]
mod tests;
