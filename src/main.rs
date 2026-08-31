#![windows_subsystem = "windows"]

mod app;
mod check;
mod config;
mod diagnostics;
mod icon;
mod install;
mod model;
mod notice;
mod platform;
mod progress;
mod registry;
mod remove;
mod selfupdate;
mod source;
mod tray;
mod update;

use std::time::Duration;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;

use app::{App, Control};
use check::Report;
use model::RemoveTarget;
use update::{Outcome, Step};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub enum Message {
    Menu(MenuEvent),
    Checked(Report),
    Step(Step),
    Updated(Outcome),
    Removed { target: RemoveTarget, ok: bool },
    Replaced(Result<semver::Version>),
}

const CLAIM_ATTEMPTS: u32 = 50;
const CLAIM_PAUSE: Duration = Duration::from_millis(200);

fn main() {
    let replaced = was_replaced(std::env::args().collect::<Vec<String>>().iter());
    if !claim(replaced) {
        return;
    }
    selfupdate::clean_stale();
    if replaced {
        platform::notify(
            "Globlin — updated",
            &format!("now running {}", env!("CARGO_PKG_VERSION")),
        )
        .ok();
    }
    if let Err(error) = run() {
        platform::notify("Globlin could not start", &error.to_string()).ok();
    }
}

fn was_replaced<S: AsRef<str>>(mut args: impl Iterator<Item = S>) -> bool {
    args.any(|argument| argument.as_ref() == selfupdate::RESTART_FLAG)
}

fn claim(replaced: bool) -> bool {
    if platform::claim_single_instance() {
        return true;
    }
    if !replaced {
        return false;
    }
    for _ in 0..CLAIM_ATTEMPTS {
        std::thread::sleep(CLAIM_PAUSE);
        if platform::claim_single_instance() {
            return true;
        }
    }
    platform::notify(
        "Globlin — update installed",
        "the update is installed but the previous instance is still running; start the app manually",
    )
    .ok();
    false
}

fn run() -> Result<()> {
    let event_loop = EventLoopBuilder::<Message>::with_user_event().build();
    let mut app = App::new(event_loop.create_proxy())?;
    forward_menu_events(event_loop.create_proxy());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = match event {
            Event::NewEvents(StartCause::Init | StartCause::ResumeTimeReached { .. }) => {
                app.on_wake();
                ControlFlow::WaitUntil(app.next_wake())
            }
            Event::UserEvent(message) => match app.handle(message) {
                Control::Exit => ControlFlow::Exit,
                Control::Continue => ControlFlow::WaitUntil(app.next_wake()),
            },
            _ => ControlFlow::WaitUntil(app.next_wake()),
        };
    })
}

fn forward_menu_events(proxy: EventLoopProxy<Message>) {
    MenuEvent::set_event_handler(Some(move |event| {
        proxy.send_event(Message::Menu(event)).ok();
    }));
}

#[cfg(test)]
mod tests;
