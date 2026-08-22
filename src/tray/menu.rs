use std::time::Duration;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use crate::model::{self, Activity, Batch, Package, RowState, SourceKind, Status, UpdateTarget};
use crate::progress;
use crate::Result;

const ID_UPDATE_ALL: &str = "update-all";
const ID_CHECK_NOW: &str = "check-now";
const ID_AUTOSTART: &str = "autostart";
const ID_OPEN_LOG: &str = "open-log";
const ID_QUIT: &str = "quit";
const UPDATE_PREFIX: &str = "update:";
const IGNORE_PREFIX: &str = "ignore:";

const DOT_CYCLE: u32 = 4;
const FRAMES_PER_DOT: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Update { name: String, source: SourceKind },
    ToggleIgnore { name: String },
    UpdateAll,
    CheckNow,
    ToggleAutostart,
    OpenLog,
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
            ID_OPEN_LOG => Some(Self::OpenLog),
            ID_QUIT => Some(Self::Quit),
            other => Self::parse_update(other).or_else(|| Self::parse_ignore(other)),
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
}

pub fn build(view: &View) -> Result<Built> {
    let menu = Menu::new();
    let header = MenuItem::new(headline(view), false, None);
    let busy = view.activity.is_some();
    let outdated = model::outdated(view.packages);
    let batch = active_batch(view.activity);

    menu.append(&header)?;
    menu.append(&PredefinedMenuItem::separator())?;

    let mut rows = Vec::new();
    if let Some(batch) = batch {
        for position in 0..batch.total() {
            let text = batch_row_text(view, position).unwrap_or_default();
            let item = MenuItem::new(text, false, None);
            menu.append(&item)?;
            rows.push(item);
        }
        if !batch.targets.is_empty() {
            menu.append(&PredefinedMenuItem::separator())?;
        }
    }

    for package in &outdated {
        if in_batch(batch, package) {
            continue;
        }
        menu.append(&package_entry(package, busy)?)?;
    }

    let settled: Vec<&Package> = view
        .packages
        .iter()
        .filter(|package| package.latest().is_none())
        .collect();
    if separates_settled(&outdated, batch, &settled) {
        menu.append(&PredefinedMenuItem::separator())?;
    }
    for package in &settled {
        menu.append(&package_entry(package, busy)?)?;
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

    Ok(Built { menu, header, rows })
}

fn in_batch(batch: Option<&Batch>, package: &Package) -> bool {
    batch.is_some_and(|batch| {
        batch
            .targets
            .iter()
            .any(|target| target.name == package.name && target.source == package.source)
    })
}

fn separates_settled(outdated: &[&Package], batch: Option<&Batch>, settled: &[&Package]) -> bool {
    !settled.is_empty() && outdated.iter().any(|package| !in_batch(batch, package))
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

pub fn headline(view: &View) -> String {
    match view.activity {
        Some(Activity::Checking) => format!("Checking npm globals{}", dots(view.frame)),
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
            None => format!("Updating npm globals{}", dots(view.frame)),
        },
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

fn ignore_id(package: &Package) -> String {
    format!("{IGNORE_PREFIX}{}:{}", package.source.label(), package.name)
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
            elapsed: Duration::ZERO,
        }
    }

    fn updating_view(activity: &Activity, elapsed: Duration) -> View<'_> {
        View {
            packages: &[],
            activity: Some(activity),
            autostart: false,
            frame: 0,
            elapsed,
        }
    }

    fn updating(name: &str, index: usize, total: usize) -> Activity {
        let targets = (0..total)
            .map(|position| UpdateTarget {
                name: if position == index {
                    name.to_string()
                } else {
                    format!("filler-{position}")
                },
                source: SourceKind::Npm,
                from: Version::parse("1.2.3").unwrap(),
                to: Version::parse("2.0.0").unwrap(),
            })
            .collect();
        let mut batch = crate::model::Batch::new(targets);
        batch.start(index);
        Activity::Updating { batch }
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
        let outdated = row(&behind("a", "2.0.0"));
        let current = row(&package("b", SourceKind::Npm, Status::Current));
        let unknown = row(&package("c", SourceKind::Npm, Status::Unknown));
        let ignored = row(&package("d", SourceKind::Npm, Status::Ignored));

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
        assert!(row(&behind("prettier", "2.0.0")).contains("1.2.3 → 2.0.0"));
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

        assert!(row(&from_bun).contains("(bun)"));
        assert!(!row(&from_npm).contains("(bun)"));
    }

    #[test]
    fn only_the_active_row_carries_a_bar() {
        let activity = updating("beta", 1, 3);
        let view = updating_view(&activity, Duration::from_secs(2));

        let first = batch_row_text(&view, 0).unwrap();
        let active = batch_row_text(&view, 1).unwrap();
        let queued = batch_row_text(&view, 2).unwrap();

        assert!(active.contains('█') || active.contains('░'), "{active}");
        assert!(!first.contains('░'), "{first}");
        assert!(!queued.contains('░'), "{queued}");
        assert!(queued.contains("queued"), "{queued}");
    }

    #[test]
    fn a_failed_row_never_renders_as_done() {
        let Activity::Updating { mut batch } = updating("alpha", 0, 2) else {
            panic!("expected an update activity");
        };
        batch.finish(0, false);
        batch.start(1);
        let activity = Activity::Updating { batch };
        let view = updating_view(&activity, Duration::ZERO);

        let failed = batch_row_text(&view, 0).unwrap();
        assert!(failed.contains("failed"), "{failed}");
        assert!(!failed.contains("done"), "{failed}");
        assert!(failed.starts_with('✗'), "{failed}");
    }

    #[test]
    fn a_landed_row_reports_done_with_a_tick() {
        let Activity::Updating { mut batch } = updating("alpha", 0, 2) else {
            panic!("expected an update activity");
        };
        batch.finish(0, true);
        batch.start(1);
        let activity = Activity::Updating { batch };
        let view = updating_view(&activity, Duration::ZERO);

        let done = batch_row_text(&view, 0).unwrap();
        assert!(done.starts_with('✓') && done.contains("done"), "{done}");
    }

    #[test]
    fn a_row_beyond_the_batch_has_no_text() {
        let activity = updating("alpha", 0, 1);
        let view = updating_view(&activity, Duration::ZERO);

        assert_eq!(batch_row_text(&view, 7), None);
    }

    #[test]
    fn a_batch_covering_every_outdated_package_needs_no_second_separator() {
        let targets = [behind("alpha", "2.0.0"), behind("beta", "2.0.0")];
        let outdated: Vec<&Package> = targets.iter().collect();
        let settled = [package("prettier", SourceKind::Npm, Status::Current)];
        let settled_refs: Vec<&Package> = settled.iter().collect();
        let batch = Batch::new(targets.iter().filter_map(Package::update_target).collect());

        assert!(!separates_settled(&outdated, Some(&batch), &settled_refs));
    }

    #[test]
    fn an_outdated_package_outside_the_batch_still_gets_its_separator() {
        let targets = [behind("alpha", "2.0.0"), behind("beta", "2.0.0")];
        let outdated: Vec<&Package> = targets.iter().collect();
        let settled = [package("prettier", SourceKind::Npm, Status::Current)];
        let settled_refs: Vec<&Package> = settled.iter().collect();
        let batch = Batch::new(vec![targets[0].update_target().unwrap()]);

        assert!(separates_settled(&outdated, Some(&batch), &settled_refs));
    }

    #[test]
    fn an_idle_menu_separates_outdated_from_settled() {
        let targets = [behind("alpha", "2.0.0")];
        let outdated: Vec<&Package> = targets.iter().collect();
        let settled = [package("prettier", SourceKind::Npm, Status::Current)];
        let settled_refs: Vec<&Package> = settled.iter().collect();

        assert!(separates_settled(&outdated, None, &settled_refs));
        assert!(!separates_settled(&outdated, None, &[]));
    }

    #[test]
    fn an_ignore_id_round_trips_including_scoped_names() {
        for name in ["prettier", "@salesforce/cli"] {
            let package = package(name, SourceKind::Npm, Status::Current);
            assert_eq!(
                Action::from_key(&ignore_id(&package)),
                Some(Action::ToggleIgnore {
                    name: name.to_string()
                })
            );
        }
    }

    #[test]
    fn the_same_name_from_two_sources_gets_two_distinct_ignore_ids() {
        let from_npm = package("typescript", SourceKind::Npm, Status::Current);
        let from_bun = package("typescript", SourceKind::Bun, Status::Current);

        assert_ne!(ignore_id(&from_npm), ignore_id(&from_bun));
    }

    #[test]
    fn an_ignore_id_with_an_unknown_source_is_rejected() {
        assert_eq!(Action::from_key("ignore:pnpm:prettier"), None);
        assert_eq!(Action::from_key("ignore:prettier"), None);
    }

    #[test]
    fn only_an_outdated_package_offers_an_update_item() {
        assert!(offers_update(&behind("prettier", "2.0.0")));
        assert!(!offers_update(&package(
            "npm",
            SourceKind::Npm,
            Status::Ignored
        )));
        assert!(!offers_update(&package(
            "typescript",
            SourceKind::Npm,
            Status::Current
        )));
    }
}
