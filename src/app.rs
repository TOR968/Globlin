use std::time::Instant;

use tao::event_loop::EventLoopProxy;
use tray_icon::menu::MenuEvent;

use crate::check::{self, Outcome};
use crate::config::Config;
use crate::model::{self, Package, SourceKind};
use crate::platform;
use crate::tray::{Action, IconState, Tray};
use crate::{Message, Result};

pub enum Control {
    Continue,
    Exit,
}

pub struct App {
    config: Config,
    tray: Tray,
    packages: Vec<Package>,
    state: IconState,
    busy: bool,
    next_check: Instant,
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
            state: IconState::Busy,
            busy: false,
            next_check,
            proxy,
        };
        if let Some(warning) = warning {
            platform::notify("npm globals", &warning).ok();
        }
        Ok(app)
    }

    pub fn next_check(&self) -> Instant {
        self.next_check
    }

    pub fn start_check(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.next_check = Instant::now() + self.config.interval();
        self.show_state(IconState::Busy);

        let config = self.config.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            proxy.send_event(Message::Checked(check::check(&config))).ok();
        });
    }

    pub fn handle(&mut self, message: Message) -> Control {
        match message {
            Message::Menu(event) => return self.on_menu(event),
            Message::Checked(result) => self.on_checked(result),
            Message::Updated(outcome) => self.on_updated(outcome),
        }
        Control::Continue
    }

    fn on_menu(&mut self, event: MenuEvent) -> Control {
        let Some(action) = Action::from_id(&event.id) else {
            return Control::Continue;
        };
        match action {
            Action::Quit => return Control::Exit,
            Action::CheckNow => self.start_check(),
            Action::UpdateAll => {
                let targets = self.outdated_targets();
                self.start_update(targets);
            }
            Action::Update { name, source } => self.start_update(vec![(name, source)]),
            Action::ToggleAutostart => self.toggle_autostart(),
            Action::OpenLog => self.open_log(),
        }
        Control::Continue
    }

    fn outdated_targets(&self) -> Vec<(String, SourceKind)> {
        model::outdated(&self.packages)
            .iter()
            .map(|package| (package.name.clone(), package.source))
            .collect()
    }

    fn start_update(&mut self, targets: Vec<(String, SourceKind)>) {
        if self.busy || targets.is_empty() {
            return;
        }
        self.busy = true;
        self.show_state(IconState::Busy);

        let config = self.config.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let outcome = check::update(&config, &targets);
            proxy.send_event(Message::Updated(outcome)).ok();
        });
    }

    fn toggle_autostart(&mut self) {
        let desired = !platform::autostart_enabled();
        if let Err(error) = platform::set_autostart(desired) {
            platform::notify(
                "npm globals",
                &format!("Could not change the startup setting: {error}"),
            )
            .ok();
        }
        self.render();
    }

    fn open_log(&self) {
        let path = check::log_path();
        if path.is_file() {
            platform::open_in_shell(&path).ok();
        } else {
            platform::notify("npm globals", "No failures have been logged yet.").ok();
        }
    }

    fn on_checked(&mut self, result: Result<Vec<Package>>) {
        self.busy = false;
        match result {
            Ok(packages) => {
                self.packages = packages;
                self.state = if model::outdated(&self.packages).is_empty() {
                    IconState::Idle
                } else {
                    IconState::Updates
                };
                self.announce_new_updates();
            }
            Err(error) => {
                self.state = IconState::Error;
                platform::notify("npm globals — check failed", &error.to_string()).ok();
            }
        }
        self.render();
    }

    fn announce_new_updates(&mut self) {
        let stamps = model::stamps(&self.packages);
        if stamps == self.config.last_notified {
            return;
        }
        if !stamps.is_empty() {
            let names: Vec<&str> = model::outdated(&self.packages)
                .iter()
                .map(|package| package.name.as_str())
                .collect();
            platform::notify(
                &format!("{} npm global update(s)", names.len()),
                &names.join(", "),
            )
            .ok();
        }
        self.config.last_notified = stamps;
        self.config.save().ok();
    }

    fn on_updated(&mut self, outcome: Outcome) {
        self.busy = false;
        if !outcome.failed.is_empty() {
            platform::notify(
                "npm globals — update failed",
                &format!("{} (see Open last log)", outcome.failed.join(", ")),
            )
            .ok();
        } else if !outcome.updated.is_empty() {
            platform::notify("npm globals — updated", &outcome.updated.join(", ")).ok();
        }
        self.start_check();
    }

    fn show_state(&mut self, state: IconState) {
        self.state = state;
        self.tray.set_state(state).ok();
    }

    fn render(&mut self) {
        self.tray
            .render(&self.packages, self.state, platform::autostart_enabled())
            .ok();
    }
}
