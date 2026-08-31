use std::path::Path;
use std::time::{Duration, Instant};

use semver::Version;
use tao::event_loop::EventLoopProxy;
use tray_icon::menu::MenuEvent;

use crate::check::{self, Report};
use crate::config::Config;
use crate::icon::{self, IconState, BUSY_FRAMES};
use crate::install;
use crate::model::{self, Activity, Batch, Package, RemoveTarget, Status, UpdateTarget};
use crate::remove;
use crate::selfupdate::{self, Release};
use crate::tray::{Action, SelfUpdate, Tray, View};
use crate::update::{self, Outcome, Step};
use crate::{diagnostics, notice, platform, progress, Message, Result};

const FRAME_INTERVAL: Duration = Duration::from_millis(120);

pub enum Control {
    Continue,
    Exit,
}

pub struct App {
    config: Config,
    tray: Tray,
    packages: Vec<Package>,
    available_release: Option<Release>,
    blocked_self: Option<Version>,
    pending_restart: Option<Version>,
    activity: Option<Activity>,
    failed: bool,
    frame: u32,
    next_check: Instant,
    step_started: Instant,
    proxy: EventLoopProxy<Message>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<Message>) -> Result<Self> {
        let (config, warning) = Config::load();
        let next_check = Instant::now() + config.interval();
        let app = Self {
            config,
            tray: Tray::new()?,
            packages: Vec::new(),
            available_release: None,
            blocked_self: None,
            pending_restart: None,
            activity: None,
            failed: false,
            frame: 0,
            next_check,
            step_started: Instant::now(),
            proxy,
        };
        if let Some(warning) = warning {
            platform::notify("Globlin", &warning).ok();
        }
        Ok(app)
    }

    pub fn next_wake(&self) -> Instant {
        match self.activity {
            Some(_) => Instant::now() + FRAME_INTERVAL,
            None => self.next_check,
        }
    }

    pub fn on_wake(&mut self) {
        match self.activity {
            Some(_) => self.advance_animation(),
            None => self.start_check(),
        }
    }

    pub fn handle(&mut self, message: Message) -> Control {
        match message {
            Message::Menu(event) => return self.on_menu(&event),
            Message::Checked(report) => self.on_checked(report),
            Message::Step(step) => self.on_step(&step),
            Message::Updated(outcome) => self.on_updated(&outcome),
            Message::Removed { target, ok } => self.on_removed(&target, ok),
            Message::Replaced(result) => return self.on_replaced(result),
        }
        Control::Continue
    }

    fn on_menu(&mut self, event: &MenuEvent) -> Control {
        let Some(action) = Action::from_id(&event.id) else {
            return Control::Continue;
        };
        match action {
            Action::Quit => return Control::Exit,
            Action::CheckNow => self.start_check(),
            Action::UpdateAll => {
                let targets = self.every_outdated_target();
                self.start_update(targets);
            }
            Action::Update { name, source } => {
                let target = self
                    .packages
                    .iter()
                    .find(|package| package.name == name && package.source == source);
                if let Some(target) = target.and_then(Package::update_target) {
                    self.start_update(vec![target]);
                }
            }
            Action::ToggleIgnore { name } => self.toggle_ignore(&name),
            Action::Remove { name, source } => {
                let known = self
                    .packages
                    .iter()
                    .any(|package| package.name == name && package.source == source);
                if known {
                    self.start_remove(RemoveTarget { name, source });
                }
            }
            Action::ToggleAutostart => self.toggle_autostart(),
            Action::UpdateSelf => {
                self.config.last_self_notice = None;
                self.start_self_update();
            }
            Action::ToggleAutoUpdate => self.toggle_auto_update(),
            Action::OpenLog => open_log(),
            Action::OpenSelfLog => open_self_log(),
        }
        Control::Continue
    }

    fn every_outdated_target(&self) -> Vec<UpdateTarget> {
        model::outdated(&self.packages)
            .iter()
            .filter_map(|package| package.update_target())
            .collect()
    }

    fn toggle_ignore(&mut self, name: &str) {
        let ignoring = !self.config.is_ignored(name);
        self.config.set_ignored(name, ignoring);
        if let Err(error) = self.config.save() {
            platform::notify("Globlin", &format!("Could not save the setting: {error}")).ok();
        }
        for package in &mut self.packages {
            if package.name == name {
                package.status = if ignoring {
                    Status::Ignored
                } else {
                    Status::Unknown
                };
            }
        }
        self.render();
        if !ignoring {
            self.start_check();
        }
    }

    fn start_check(&mut self) {
        if self.activity.is_some() {
            return;
        }
        self.next_check = Instant::now() + self.config.interval();
        self.begin(Activity::Checking);

        let config = self.config.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            proxy.send_event(Message::Checked(check::run(&config))).ok();
        });
    }

    fn start_update(&mut self, targets: Vec<UpdateTarget>) {
        if targets.is_empty() || self.activity.is_some() {
            return;
        }
        self.begin(Activity::Updating {
            batch: Batch::new(targets.clone()),
        });

        let config = self.config.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let announce = |step| {
                proxy.send_event(Message::Step(step)).ok();
            };
            let outcome = update::run(&config, &targets, announce);
            proxy.send_event(Message::Updated(outcome)).ok();
        });
    }

    fn start_remove(&mut self, target: RemoveTarget) {
        if self.activity.is_some() {
            return;
        }
        self.begin(Activity::Removing {
            target: target.clone(),
        });

        let config = self.config.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let ok = remove::run(&config, &target);
            proxy.send_event(Message::Removed { target, ok }).ok();
        });
    }

    fn start_self_update(&mut self) {
        if self.activity.is_some() || self.pending_restart.is_some() {
            return;
        }
        let Some(release) = self.available_release.clone() else {
            return;
        };
        self.begin(Activity::SelfUpdate);

        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            proxy
                .send_event(Message::Replaced(selfupdate::apply(&release)))
                .ok();
        });
    }

    fn toggle_auto_update(&mut self) {
        self.config.auto_update = !self.config.auto_update;
        if let Err(error) = self.config.save() {
            platform::notify("Globlin", &format!("Could not save the setting: {error}")).ok();
        }
        self.render();
    }

    fn on_replaced(&mut self, result: Result<Version>) -> Control {
        self.activity = None;
        match result {
            Ok(version) => {
                self.available_release = None;
                if selfupdate::relaunch().is_ok() {
                    return Control::Exit;
                }
                self.pending_restart = Some(version);
                platform::notify(
                    "Globlin",
                    "The new build is installed; restart the app to run it.",
                )
                .ok();
            }
            Err(error) => self.report_self_failure(&error.to_string()),
        }
        self.render();
        Control::Continue
    }

    fn report_self_failure(&mut self, error: &str) {
        let Some(release) = self.available_release.as_ref() else {
            return;
        };
        let version = release.version.clone();
        self.blocked_self = Some(version.clone());
        let version = version.to_string();
        diagnostics::record_self_update_failure(&format!(
            "self-update failed: {version}: {error}\n"
        ));
        if !notice::self_failure(&version, self.config.last_self_notice.as_deref()) {
            return;
        }
        platform::notify(
            "Globlin — self-update failed",
            &format!("{version}: {error}"),
        )
        .ok();
        self.config.last_self_notice = Some(version);
        self.config.save().ok();
    }

    fn begin(&mut self, activity: Activity) {
        self.activity = Some(activity);
        self.frame = 0;
        self.step_started = Instant::now();
        self.render();
    }

    fn advance_animation(&mut self) {
        self.frame = (self.frame + 1) % (BUSY_FRAMES * 4);
        let level = self.water_level();
        let view = view_of(
            &self.packages,
            self.activity.as_ref(),
            self.available_release.as_ref(),
            self.pending_restart.as_ref(),
            self.config.auto_update,
            self.frame,
            self.elapsed(),
        );
        self.tray.animate(&view, level).ok();
    }

    fn on_step(&mut self, step: &Step) {
        if let Some(Activity::Updating { batch }) = self.activity.as_mut() {
            match step {
                Step::Started { index, .. } => batch.start(*index),
                Step::Finished { index, ok } => batch.finish(*index, *ok),
            }
        }
        self.step_started = Instant::now();
        self.render();
    }

    fn on_checked(&mut self, report: Report) {
        self.activity = None;
        self.available_release = report.release.filter(|release| {
            selfupdate::supersedes(&release.version, self.pending_restart.as_ref())
        });
        match report.packages {
            Ok(packages) => {
                self.packages = packages;
                self.failed = false;
                self.announce_new_updates();
            }
            Err(error) => {
                self.failed = true;
                platform::notify("Globlin — check failed", &error.to_string()).ok();
            }
        }
        if self.wants_auto_update() {
            self.start_self_update();
        }
        self.render();
    }

    fn wants_auto_update(&self) -> bool {
        self.available_release.as_ref().is_some_and(|release| {
            selfupdate::should_auto_apply(
                &release.version,
                self.config.auto_update,
                self.blocked_self.as_ref(),
            )
        })
    }

    fn announce_new_updates(&mut self) {
        let stamps = match notice::decide(&self.packages, &self.config.last_notified) {
            notice::Decision::Nothing => return,
            notice::Decision::Remember { stamps } => stamps,
            notice::Decision::Announce {
                title,
                body,
                stamps,
            } => {
                platform::notify(&title, &body).ok();
                stamps
            }
        };
        self.config.last_notified = stamps;
        self.config.save().ok();
    }

    fn on_updated(&mut self, outcome: &Outcome) {
        self.activity = None;
        if !outcome.failed.is_empty() {
            platform::notify(
                "Globlin — update failed",
                &format!("{} (see Open last log)", outcome.failed.join(", ")),
            )
            .ok();
        } else if !outcome.updated.is_empty() {
            platform::notify("Globlin — updated", &outcome.updated.join("\n")).ok();
        }
        self.start_check();
    }

    fn on_removed(&mut self, target: &RemoveTarget, ok: bool) {
        self.activity = None;
        let name = format!("{}{}", target.name, target.source.suffix());
        if ok {
            platform::notify("Globlin — removed", &name).ok();
        } else {
            platform::notify(
                "Globlin — uninstall failed",
                &format!("{name} (see Open last log)"),
            )
            .ok();
        }
        self.start_check();
    }

    fn toggle_autostart(&mut self) {
        let desired = !platform::autostart_enabled();
        if let Err(error) = platform::set_autostart(desired) {
            platform::notify(
                "Globlin",
                &format!("Could not change the startup setting: {error}"),
            )
            .ok();
        }
        self.render();
    }

    fn render(&mut self) {
        let state = self.icon_state();
        let level = self.water_level();
        let view = view_of(
            &self.packages,
            self.activity.as_ref(),
            self.available_release.as_ref(),
            self.pending_restart.as_ref(),
            self.config.auto_update,
            self.frame,
            self.elapsed(),
        );
        self.tray.render(&view, state, level).ok();
    }

    fn icon_state(&self) -> IconState {
        icon::state_for(
            self.activity.is_some(),
            self.failed,
            model::outdated(&self.packages).len(),
        )
    }

    fn water_level(&self) -> f32 {
        match &self.activity {
            Some(Activity::Updating { batch }) => {
                progress::level(batch.done(), batch.total(), self.elapsed())
            }
            Some(Activity::Checking | Activity::SelfUpdate | Activity::Removing { .. }) => {
                progress::creep(self.elapsed())
            }
            None => 0.0,
        }
    }

    fn elapsed(&self) -> Duration {
        self.step_started.elapsed()
    }
}

fn open_log() {
    open_diagnostic(
        &diagnostics::log_path(),
        "No failures have been logged yet.",
    );
}

fn open_self_log() {
    open_diagnostic(
        &diagnostics::self_update_log_path(),
        "No self-update failures have been logged yet.",
    );
}

fn open_diagnostic(path: &Path, empty: &str) {
    if path.is_file() {
        platform::open_in_shell(path).ok();
    } else {
        platform::notify("Globlin", empty).ok();
    }
}

fn view_of<'a>(
    packages: &'a [Package],
    activity: Option<&'a Activity>,
    release: Option<&'a Release>,
    pending_restart: Option<&'a Version>,
    auto_update: bool,
    frame: u32,
    elapsed: Duration,
) -> View<'a> {
    View {
        packages,
        activity,
        self_update: if install::winget_managed() {
            SelfUpdate::Winget
        } else {
            SelfUpdate::Own {
                release,
                auto_update,
                log: diagnostics::self_update_log_path().is_file(),
            }
        },
        pending_restart,
        autostart: platform::autostart_enabled(),
        frame,
        elapsed,
    }
}
