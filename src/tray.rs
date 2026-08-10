use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::model::{self, Package, SourceKind, Status};
use crate::Result;

const ID_UPDATE_ALL: &str = "update-all";
const ID_CHECK_NOW: &str = "check-now";
const ID_AUTOSTART: &str = "autostart";
const ID_OPEN_LOG: &str = "open-log";
const ID_QUIT: &str = "quit";
const UPDATE_PREFIX: &str = "update:";

const SIZE: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Update { name: String, source: SourceKind },
    UpdateAll,
    CheckNow,
    ToggleAutostart,
    OpenLog,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Updates,
    Busy,
    Error,
}

pub struct Tray {
    icon: TrayIcon,
}

impl Tray {
    pub fn new() -> Result<Self> {
        let icon = TrayIconBuilder::new()
            .with_tooltip(headline(&[], IconState::Busy))
            .with_icon(render_icon(IconState::Busy)?)
            .with_menu(Box::new(build_menu(&[], IconState::Busy, false)?))
            .build()?;
        Ok(Self { icon })
    }

    pub fn render(
        &mut self,
        packages: &[Package],
        state: IconState,
        autostart: bool,
    ) -> Result<()> {
        self.icon
            .set_menu(Some(Box::new(build_menu(packages, state, autostart)?)));
        self.icon.set_tooltip(Some(headline(packages, state)))?;
        self.icon.set_icon(Some(render_icon(state)?))?;
        Ok(())
    }

    pub fn set_state(&mut self, state: IconState) -> Result<()> {
        self.icon.set_icon(Some(render_icon(state)?))?;
        Ok(())
    }
}

impl Action {
    pub fn from_id(id: &MenuId) -> Option<Self> {
        Self::from_key(id.as_ref())
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            ID_UPDATE_ALL => Some(Action::UpdateAll),
            ID_CHECK_NOW => Some(Action::CheckNow),
            ID_AUTOSTART => Some(Action::ToggleAutostart),
            ID_OPEN_LOG => Some(Action::OpenLog),
            ID_QUIT => Some(Action::Quit),
            other => Self::parse_update(other),
        }
    }

    fn parse_update(key: &str) -> Option<Self> {
        let (label, name) = key.strip_prefix(UPDATE_PREFIX)?.split_once(':')?;
        Some(Action::Update {
            name: name.to_string(),
            source: SourceKind::from_label(label)?,
        })
    }
}

fn build_menu(packages: &[Package], state: IconState, autostart: bool) -> Result<Menu> {
    let menu = Menu::new();
    let outdated = model::outdated(packages);

    menu.append(&MenuItem::new(headline(packages, state), false, None))?;
    menu.append(&PredefinedMenuItem::separator())?;

    for package in &outdated {
        menu.append(&MenuItem::with_id(
            update_id(package),
            outdated_label(package),
            true,
            None,
        ))?;
    }

    let quiet: Vec<&Package> = packages
        .iter()
        .filter(|package| package.latest().is_none())
        .collect();
    if !outdated.is_empty() && !quiet.is_empty() {
        menu.append(&PredefinedMenuItem::separator())?;
    }
    for package in &quiet {
        menu.append(&MenuItem::new(quiet_label(package), false, None))?;
    }

    menu.append(&PredefinedMenuItem::separator())?;
    if !outdated.is_empty() {
        menu.append(&MenuItem::with_id(
            ID_UPDATE_ALL,
            format!("Update all ({})", outdated.len()),
            true,
            None,
        ))?;
    }
    menu.append(&MenuItem::with_id(ID_CHECK_NOW, "Check now", true, None))?;
    menu.append(&CheckMenuItem::with_id(
        ID_AUTOSTART,
        "Run at startup",
        true,
        autostart,
        None,
    ))?;
    menu.append(&MenuItem::with_id(ID_OPEN_LOG, "Open last log", true, None))?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id(ID_QUIT, "Quit", true, None))?;

    Ok(menu)
}

fn headline(packages: &[Package], state: IconState) -> String {
    match state {
        IconState::Busy => "npm globals — working…".to_string(),
        IconState::Error => "npm globals — check failed".to_string(),
        _ => match model::outdated(packages).len() {
            0 if packages.is_empty() => "npm globals — nothing found".to_string(),
            0 => "npm globals — up to date".to_string(),
            1 => "npm globals — 1 update".to_string(),
            count => format!("npm globals — {count} updates"),
        },
    }
}

fn update_id(package: &Package) -> String {
    format!(
        "{UPDATE_PREFIX}{}:{}",
        package.source.label(),
        package.name
    )
}

fn outdated_label(package: &Package) -> String {
    let latest = package
        .latest()
        .map(ToString::to_string)
        .unwrap_or_default();
    format!(
        "↑  {}{}   {} → {}",
        package.name,
        source_suffix(package),
        package.current,
        latest
    )
}

fn quiet_label(package: &Package) -> String {
    let (marker, note) = match package.status {
        Status::Unknown => ("?", "   (not checked)"),
        Status::Ignored => ("·", "   (ignored)"),
        _ => ("✓", ""),
    };
    format!(
        "{marker}  {}{}   {}{note}",
        package.name,
        source_suffix(package),
        package.current
    )
}

fn source_suffix(package: &Package) -> &'static str {
    match package.source {
        SourceKind::Npm => "",
        SourceKind::Bun => " (bun)",
    }
}

fn render_icon(state: IconState) -> Result<Icon> {
    Ok(Icon::from_rgba(pixels(state), SIZE, SIZE).map_err(|error| error.to_string())?)
}

fn pixels(state: IconState) -> Vec<u8> {
    let mut buffer = vec![0u8; (SIZE * SIZE * 4) as usize];
    let center = (SIZE as f32 - 1.0) / 2.0;
    let radius = center - 0.5;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= radius * radius {
                put(&mut buffer, x, y, color(state));
            }
        }
    }
    if state == IconState::Updates {
        draw_arrow(&mut buffer);
    }
    buffer
}

fn color(state: IconState) -> [u8; 4] {
    match state {
        IconState::Idle => [0x6b, 0x72, 0x80, 0xff],
        IconState::Updates => [0xf5, 0x9e, 0x0b, 0xff],
        IconState::Busy => [0x38, 0xbd, 0xf8, 0xff],
        IconState::Error => [0xef, 0x44, 0x44, 0xff],
    }
}

fn draw_arrow(buffer: &mut [u8]) {
    const WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

    for y in 8..17 {
        let half = y - 8;
        for x in (16 - half)..=(16 + half) {
            put(buffer, x, y, WHITE);
        }
    }
    for y in 17..25 {
        for x in 13..19 {
            put(buffer, x, y, WHITE);
        }
    }
}

fn put(buffer: &mut [u8], x: u32, y: u32, color: [u8; 4]) {
    let offset = ((y * SIZE + x) * 4) as usize;
    buffer[offset..offset + 4].copy_from_slice(&color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    fn package(name: &str, source: SourceKind, status: Status) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse("1.2.3").unwrap(),
            source,
            status,
        }
    }

    #[test]
    fn fixed_menu_ids_map_to_their_actions() {
        assert_eq!(Action::from_key("quit"), Some(Action::Quit));
        assert_eq!(Action::from_key("check-now"), Some(Action::CheckNow));
        assert_eq!(Action::from_key("update-all"), Some(Action::UpdateAll));
        assert_eq!(Action::from_key("autostart"), Some(Action::ToggleAutostart));
        assert_eq!(Action::from_key("open-log"), Some(Action::OpenLog));
    }

    #[test]
    fn an_update_id_round_trips_through_the_menu() {
        let target = package("prettier", SourceKind::Npm, Status::Current);
        let id = update_id(&target);

        assert_eq!(
            Action::from_key(&id),
            Some(Action::Update {
                name: "prettier".to_string(),
                source: SourceKind::Npm
            })
        );
    }

    #[test]
    fn scoped_names_survive_the_id_round_trip() {
        let target = package("@salesforce/cli", SourceKind::Npm, Status::Current);

        assert_eq!(
            Action::from_key(&update_id(&target)),
            Some(Action::Update {
                name: "@salesforce/cli".to_string(),
                source: SourceKind::Npm
            })
        );
    }

    #[test]
    fn the_same_name_from_two_sources_gets_two_distinct_ids() {
        let from_npm = package("typescript", SourceKind::Npm, Status::Current);
        let from_bun = package("typescript", SourceKind::Bun, Status::Current);

        assert_ne!(update_id(&from_npm), update_id(&from_bun));
        assert_eq!(
            Action::from_key(&update_id(&from_bun)),
            Some(Action::Update {
                name: "typescript".to_string(),
                source: SourceKind::Bun
            })
        );
    }

    #[test]
    fn unknown_ids_are_ignored() {
        assert_eq!(Action::from_key("something-else"), None);
        assert_eq!(Action::from_key("update:pnpm:prettier"), None);
        assert_eq!(Action::from_key("update:npm"), None);
    }

    #[test]
    fn the_headline_counts_only_outdated_packages() {
        let packages = vec![
            package(
                "a",
                SourceKind::Npm,
                Status::Outdated {
                    latest: Version::parse("2.0.0").unwrap(),
                },
            ),
            package("b", SourceKind::Npm, Status::Current),
            package("c", SourceKind::Npm, Status::Ignored),
        ];

        assert_eq!(headline(&packages, IconState::Updates), "npm globals — 1 update");
    }

    #[test]
    fn bun_packages_are_labelled_so_duplicates_are_distinguishable() {
        let from_bun = package("typescript", SourceKind::Bun, Status::Current);
        assert!(quiet_label(&from_bun).contains("(bun)"));

        let from_npm = package("typescript", SourceKind::Npm, Status::Current);
        assert!(!quiet_label(&from_npm).contains("(bun)"));
    }

    #[test]
    fn every_icon_state_produces_a_full_rgba_buffer() {
        for state in [
            IconState::Idle,
            IconState::Updates,
            IconState::Busy,
            IconState::Error,
        ] {
            assert_eq!(pixels(state).len(), (SIZE * SIZE * 4) as usize);
        }
    }
}
