use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};

use crate::model::{self, Activity, Package, SourceKind, Status};
use crate::Result;

const ID_UPDATE_ALL: &str = "update-all";
const ID_CHECK_NOW: &str = "check-now";
const ID_AUTOSTART: &str = "autostart";
const ID_OPEN_LOG: &str = "open-log";
const ID_QUIT: &str = "quit";
const UPDATE_PREFIX: &str = "update:";

const DOT_CYCLE: u32 = 4;
const FRAMES_PER_DOT: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Update { name: String, source: SourceKind },
    UpdateAll,
    CheckNow,
    ToggleAutostart,
    OpenLog,
    Quit,
}

pub struct Built {
    pub menu: Menu,
    pub header: MenuItem,
}

pub struct View<'a> {
    pub packages: &'a [Package],
    pub activity: Option<&'a Activity>,
    pub autostart: bool,
    pub frame: u32,
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
            ID_OPEN_LOG => Some(Self::OpenLog),
            ID_QUIT => Some(Self::Quit),
            other => Self::parse_update(other),
        }
    }

    fn parse_update(key: &str) -> Option<Self> {
        let (label, name) = key.strip_prefix(UPDATE_PREFIX)?.split_once(':')?;
        Some(Self::Update {
            name: name.to_string(),
            source: SourceKind::from_label(label)?,
        })
    }
}

pub fn build(view: &View) -> Result<Built> {
    let menu = Menu::new();
    let header = MenuItem::new(headline(view), false, None);
    let busy = view.activity.is_some();
    let outdated = model::outdated(view.packages);

    menu.append(&header)?;
    menu.append(&PredefinedMenuItem::separator())?;

    for package in &outdated {
        menu.append(&MenuItem::with_id(
            update_id(package),
            row(package, view.activity, view.frame),
            !busy,
            None,
        ))?;
    }

    let settled: Vec<&Package> = view
        .packages
        .iter()
        .filter(|package| package.latest().is_none())
        .collect();
    if !outdated.is_empty() && !settled.is_empty() {
        menu.append(&PredefinedMenuItem::separator())?;
    }
    for package in &settled {
        menu.append(&MenuItem::new(
            row(package, view.activity, view.frame),
            false,
            None,
        ))?;
    }

    menu.append(&PredefinedMenuItem::separator())?;
    if !outdated.is_empty() {
        menu.append(&MenuItem::with_id(
            ID_UPDATE_ALL,
            format!("Update all ({})", outdated.len()),
            !busy,
            None,
        ))?;
    }
    menu.append(&MenuItem::with_id(ID_CHECK_NOW, "Check now", !busy, None))?;
    menu.append(&CheckMenuItem::with_id(
        ID_AUTOSTART,
        "Run at startup",
        true,
        view.autostart,
        None,
    ))?;
    menu.append(&MenuItem::with_id(ID_OPEN_LOG, "Open last log", true, None))?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id(ID_QUIT, "Quit", true, None))?;

    Ok(Built { menu, header })
}

pub fn headline(view: &View) -> String {
    match view.activity {
        Some(Activity::Checking) => format!("Checking npm globals{}", dots(view.frame)),
        Some(Activity::Updating {
            target,
            index,
            total,
        }) => {
            let progress = if *total > 1 {
                format!("  [{}/{}]", index + 1, total)
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
        None => summary(view.packages),
    }
}

fn summary(packages: &[Package]) -> String {
    if packages.is_empty() {
        return "npm globals — nothing found".to_string();
    }
    match model::outdated(packages).len() {
        0 => format!("npm globals — {} packages, up to date", packages.len()),
        1 => "npm globals — 1 update available".to_string(),
        count => format!("npm globals — {count} updates available"),
    }
}

fn dots(frame: u32) -> String {
    ".".repeat((frame / FRAMES_PER_DOT % DOT_CYCLE) as usize)
}

fn update_id(package: &Package) -> String {
    format!("{UPDATE_PREFIX}{}:{}", package.source.label(), package.name)
}

fn row(package: &Package, activity: Option<&Activity>, frame: u32) -> String {
    if is_in_progress(package, activity) {
        return format!(
            "{}  {}{}   {} → {}{}",
            spinner_tick(frame),
            package.name,
            package.source.suffix(),
            package.current,
            package
                .latest()
                .map(ToString::to_string)
                .unwrap_or_default(),
            dots(frame)
        );
    }
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

fn is_in_progress(package: &Package, activity: Option<&Activity>) -> bool {
    matches!(
        activity,
        Some(Activity::Updating { target, .. })
            if target.name == package.name && target.source == package.source
    )
}

const fn spinner_tick(frame: u32) -> char {
    const TICKS: [char; 4] = ['◐', '◓', '◑', '◒'];
    TICKS[frame as usize % TICKS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::UpdateTarget;
    use semver::Version;

    fn package(name: &str, source: SourceKind, status: Status) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse("1.2.3").unwrap(),
            source,
            status,
        }
    }

    fn behind(name: &str, latest: &str) -> Package {
        package(
            name,
            SourceKind::Npm,
            Status::Outdated {
                latest: Version::parse(latest).unwrap(),
            },
        )
    }

    fn view<'a>(packages: &'a [Package], activity: Option<&'a Activity>, frame: u32) -> View<'a> {
        View {
            packages,
            activity,
            autostart: false,
            frame,
        }
    }

    fn updating(name: &str, index: usize, total: usize) -> Activity {
        Activity::Updating {
            target: UpdateTarget {
                name: name.to_string(),
                source: SourceKind::Npm,
                from: Version::parse("1.2.3").unwrap(),
                to: Version::parse("2.0.0").unwrap(),
            },
            index,
            total,
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
    fn an_update_id_round_trips_including_scoped_names() {
        for name in ["prettier", "@salesforce/cli"] {
            let target = behind(name, "2.0.0");
            assert_eq!(
                Action::from_key(&update_id(&target)),
                Some(Action::Update {
                    name: name.to_string(),
                    source: SourceKind::Npm
                })
            );
        }
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
    fn the_idle_headline_counts_only_outdated_packages() {
        let packages = vec![
            behind("a", "2.0.0"),
            package("b", SourceKind::Npm, Status::Current),
            package("c", SourceKind::Npm, Status::Ignored),
        ];

        assert_eq!(
            headline(&view(&packages, None, 0)),
            "npm globals — 1 update available"
        );
    }

    #[test]
    fn an_up_to_date_headline_reports_how_many_are_watched() {
        let packages = vec![
            package("a", SourceKind::Npm, Status::Current),
            package("b", SourceKind::Npm, Status::Current),
        ];

        assert_eq!(
            headline(&view(&packages, None, 0)),
            "npm globals — 2 packages, up to date"
        );
    }

    #[test]
    fn the_update_headline_names_the_package_and_both_versions() {
        let activity = updating("prettier", 0, 1);
        let text = headline(&view(&[], Some(&activity), 0));

        assert!(
            text.starts_with("Updating prettier 1.2.3 → 2.0.0"),
            "{text}"
        );
        assert!(
            !text.contains('['),
            "a single target needs no counter: {text}"
        );
    }

    #[test]
    fn a_batch_update_headline_shows_a_one_based_counter() {
        let activity = updating("vercel", 1, 3);
        assert!(
            headline(&view(&[], Some(&activity), 0)).contains("[2/3]"),
            "expected a 2/3 counter"
        );
    }

    #[test]
    fn the_dots_grow_then_reset_so_the_header_animates() {
        let activity = Activity::Checking;
        let rendered: Vec<String> = (0..8)
            .map(|frame| headline(&view(&[], Some(&activity), frame)))
            .collect();

        assert_eq!(rendered[0], "Checking npm globals");
        assert_eq!(rendered[2], "Checking npm globals.");
        assert_eq!(rendered[4], "Checking npm globals..");
        assert_eq!(rendered[6], "Checking npm globals...");
    }

    #[test]
    fn the_dot_animation_repeats_after_a_full_cycle() {
        assert_eq!(dots(0), dots(DOT_CYCLE * FRAMES_PER_DOT));
        assert_eq!(dots(3), dots(3 + DOT_CYCLE * FRAMES_PER_DOT));
    }

    #[test]
    fn each_status_gets_its_own_marker() {
        let outdated = row(&behind("a", "2.0.0"), None, 0);
        let current = row(&package("b", SourceKind::Npm, Status::Current), None, 0);
        let unknown = row(&package("c", SourceKind::Npm, Status::Unknown), None, 0);
        let ignored = row(&package("d", SourceKind::Npm, Status::Ignored), None, 0);

        assert!(outdated.starts_with('↑'), "{outdated}");
        assert!(current.starts_with('✓'), "{current}");
        assert!(
            unknown.starts_with('?') && unknown.contains("not checked"),
            "{unknown}"
        );
        assert!(
            ignored.starts_with('·') && ignored.contains("ignored"),
            "{ignored}"
        );
    }

    #[test]
    fn an_outdated_row_shows_installed_and_target_versions() {
        assert!(row(&behind("prettier", "2.0.0"), None, 0).contains("1.2.3 → 2.0.0"));
    }

    #[test]
    fn the_row_being_updated_gets_a_spinner_instead_of_the_arrow() {
        let target = behind("prettier", "2.0.0");
        let activity = updating("prettier", 0, 1);
        let text = row(&target, Some(&activity), 0);

        assert!(!text.starts_with('↑'), "{text}");
        assert!(text.contains("1.2.3 → 2.0.0"), "{text}");
    }

    #[test]
    fn only_the_matching_source_row_shows_as_in_progress() {
        let activity = updating("typescript", 0, 1);
        let from_npm = package("typescript", SourceKind::Npm, Status::Current);
        let from_bun = package("typescript", SourceKind::Bun, Status::Current);

        assert!(is_in_progress(&from_npm, Some(&activity)));
        assert!(!is_in_progress(&from_bun, Some(&activity)));
        assert!(!is_in_progress(&from_npm, None));
    }

    #[test]
    fn the_row_spinner_advances_every_frame_and_repeats() {
        let ticks: Vec<char> = (0..5).map(spinner_tick).collect();

        assert_eq!(ticks[0], ticks[4]);
        assert_eq!(
            ticks[..4]
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            4
        );
    }

    #[test]
    fn bun_packages_are_labelled_so_duplicates_are_distinguishable() {
        let from_bun = package("typescript", SourceKind::Bun, Status::Current);
        let from_npm = package("typescript", SourceKind::Npm, Status::Current);

        assert!(row(&from_bun, None, 0).contains("(bun)"));
        assert!(!row(&from_npm, None, 0).contains("(bun)"));
    }
}
