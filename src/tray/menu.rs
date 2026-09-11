use std::time::Duration;

use semver::Version;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

use crate::model::{
    self, Activity, Batch, Package, PackageRef, RowState, SourceKind, UpdateTarget,
};
use crate::progress;
use crate::selfupdate::Release;
use crate::Result;

const ID_UPDATE_ALL: &str = "update-all";
const ID_CHECK_NOW: &str = "check-now";
const ID_OPEN_WINDOW: &str = "open-window";
const ID_WINDOW_READY: &str = "window-ready";
const ID_AUTOSTART: &str = "autostart";
const ID_UPDATE_SELF: &str = "update-self";
const ID_AUTO_UPDATE: &str = "auto-update";
const ID_OPEN_LOG: &str = "open-log";
const ID_OPEN_SELF_LOG: &str = "open-self-log";
const ID_QUIT: &str = "quit";
const UPDATE_PREFIX: &str = "update:";
const UPDATE_MANY_PREFIX: &str = "update-many:";
const IGNORE_PREFIX: &str = "ignore:";
const REMOVE_PREFIX: &str = "remove:";
const SOURCE_PREFIX: &str = "source:";
const REF_SEPARATOR: char = '|';

const DOT_CYCLE: u32 = 4;
const FRAMES_PER_DOT: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Update { name: String, source: SourceKind },
    UpdateMany { refs: Vec<PackageRef> },
    ToggleIgnore { name: String },
    Remove { name: String, source: SourceKind },
    ToggleSource { kind: SourceKind },
    UpdateAll,
    CheckNow,
    OpenWindow,
    WindowReady,
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

#[derive(Clone, Copy)]
pub enum SelfUpdate<'a> {
    Winget,
    Own {
        release: Option<&'a Release>,
        auto_update: bool,
        log: bool,
    },
}

pub struct View<'a> {
    pub packages: &'a [Package],
    pub activity: Option<&'a Activity>,
    pub autostart: bool,
    pub self_update: SelfUpdate<'a>,
    pub pending_restart: Option<&'a Version>,
    pub frame: u32,
    pub elapsed: Duration,
}

impl Action {
    pub fn from_id(id: &MenuId) -> Option<Self> {
        Self::from_key(id.as_ref())
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            ID_UPDATE_ALL => Some(Self::UpdateAll),
            ID_CHECK_NOW => Some(Self::CheckNow),
            ID_OPEN_WINDOW => Some(Self::OpenWindow),
            ID_WINDOW_READY => Some(Self::WindowReady),
            ID_AUTOSTART => Some(Self::ToggleAutostart),
            ID_UPDATE_SELF => Some(Self::UpdateSelf),
            ID_AUTO_UPDATE => Some(Self::ToggleAutoUpdate),
            ID_OPEN_LOG => Some(Self::OpenLog),
            ID_OPEN_SELF_LOG => Some(Self::OpenSelfLog),
            ID_QUIT => Some(Self::Quit),
            other => Self::parse_update(other)
                .or_else(|| Self::parse_update_many(other))
                .or_else(|| Self::parse_ignore(other))
                .or_else(|| Self::parse_remove(other))
                .or_else(|| Self::parse_source(other)),
        }
    }

    fn parse_update(key: &str) -> Option<Self> {
        let reference = package_ref(key.strip_prefix(UPDATE_PREFIX)?)?;
        Some(Self::Update {
            name: reference.name,
            source: reference.source,
        })
    }

    fn parse_update_many(key: &str) -> Option<Self> {
        let refs: Vec<PackageRef> = key
            .strip_prefix(UPDATE_MANY_PREFIX)?
            .split(REF_SEPARATOR)
            .filter_map(package_ref)
            .collect();
        (!refs.is_empty()).then_some(Self::UpdateMany { refs })
    }

    fn parse_ignore(key: &str) -> Option<Self> {
        let reference = package_ref(key.strip_prefix(IGNORE_PREFIX)?)?;
        Some(Self::ToggleIgnore {
            name: reference.name,
        })
    }

    fn parse_remove(key: &str) -> Option<Self> {
        let reference = package_ref(key.strip_prefix(REMOVE_PREFIX)?)?;
        Some(Self::Remove {
            name: reference.name,
            source: reference.source,
        })
    }

    fn parse_source(key: &str) -> Option<Self> {
        Some(Self::ToggleSource {
            kind: SourceKind::from_label(key.strip_prefix(SOURCE_PREFIX)?)?,
        })
    }
}

fn package_ref(key: &str) -> Option<PackageRef> {
    let (label, name) = key.split_once(':')?;
    if name.is_empty() {
        return None;
    }
    Some(PackageRef {
        name: name.to_string(),
        source: SourceKind::from_label(label)?,
    })
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
    let updatable = model::updatable(view.packages).len();

    let mut built = Sections::new();
    built.item(&header)?;
    built.split()?;

    let mut rows = Vec::new();
    if let Some(batch) = active_batch(view.activity) {
        for position in 0..batch.total() {
            let text = batch_row_text(view, position).unwrap_or_default();
            let item = MenuItem::new(text, false, None);
            built.item(&item)?;
            rows.push(item);
        }
    }
    built.split()?;

    built.item(&MenuItem::with_id(
        ID_OPEN_WINDOW,
        "Open Globlin",
        true,
        None,
    ))?;
    if updatable > 0 {
        built.item(&MenuItem::with_id(
            ID_UPDATE_ALL,
            format!("Update all ({updatable})"),
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
    let SelfUpdate::Own {
        release,
        auto_update,
        log,
    } = view.self_update
    else {
        entry.append(&MenuItem::new(
            "Installed with winget — run winget upgrade",
            false,
            None,
        ))?;
        return Ok(entry);
    };
    if let Some(release) = release {
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
        auto_update,
        None,
    ))?;
    if log {
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

pub fn update_id(package: &Package) -> String {
    format!("{UPDATE_PREFIX}{}", key_of(package))
}

pub fn ignore_id(package: &Package) -> String {
    format!("{IGNORE_PREFIX}{}", key_of(package))
}

pub fn remove_id(package: &Package) -> String {
    format!("{REMOVE_PREFIX}{}", key_of(package))
}

pub fn source_id(kind: SourceKind) -> String {
    format!("{SOURCE_PREFIX}{}", kind.label())
}

pub fn key_of(package: &Package) -> String {
    format!("{}:{}", package.source.label(), package.name)
}

const fn spinner_tick(frame: u32) -> char {
    const TICKS: [char; 4] = ['◐', '◓', '◑', '◒'];
    TICKS[frame as usize % TICKS.len()]
}

#[cfg(test)]
mod tests;
