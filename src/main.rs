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
mod window;

use std::time::Duration;

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

use app::{App, Control};
use check::Report;
use model::RemoveTarget;
use update::{Outcome, Step};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub enum Message {
    Menu(MenuEvent),
    Tray(TrayIconEvent),
    NotificationClicked,
    Ipc(String),
    Checked(Report),
    Step(Step),
    Updated(Outcome),
    Removed { target: RemoveTarget, ok: bool },
    Replaced(Result<semver::Version>),
}

const CLAIM_ATTEMPTS: u32 = 50;
const CLAIM_PAUSE: Duration = Duration::from_millis(200);

fn main() {
    platform::prepare_environment();
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
    run();
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

fn run() -> ! {
    let mut event_loop = EventLoopBuilder::<Message>::with_user_event().build();
    hide_from_dock(&mut event_loop);
    let proxy = event_loop.create_proxy();
    forward_menu_events(event_loop.create_proxy());
    forward_tray_events(event_loop.create_proxy());
    forward_notification_clicks(event_loop.create_proxy());
    let mut app: Option<App> = None;

    event_loop.run(move |event, target, control_flow| {
        if matches!(event, Event::NewEvents(StartCause::Init)) {
            match App::new(proxy.clone()) {
                Ok(started) => app = Some(started),
                Err(error) => {
                    platform::notify("Globlin could not start", &error.to_string()).ok();
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
        }
        let Some(app) = app.as_mut() else {
            return;
        };
        *control_flow = match event {
            Event::NewEvents(StartCause::Init | StartCause::ResumeTimeReached { .. }) => {
                app.on_wake();
                ControlFlow::WaitUntil(app.next_wake())
            }
            Event::UserEvent(message) => match app.handle(message, target) {
                Control::Exit => ControlFlow::Exit,
                Control::Continue => ControlFlow::WaitUntil(app.next_wake()),
            },
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                app.close_window(window_id);
                ControlFlow::WaitUntil(app.next_wake())
            }
            _ => ControlFlow::WaitUntil(app.next_wake()),
        };
    })
}

#[cfg(target_os = "macos")]
fn hide_from_dock(event_loop: &mut EventLoop<Message>) {
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};

    event_loop.set_activation_policy(ActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
const fn hide_from_dock(_event_loop: &mut EventLoop<Message>) {}

fn forward_menu_events(proxy: EventLoopProxy<Message>) {
    MenuEvent::set_event_handler(Some(move |event| {
        proxy.send_event(Message::Menu(event)).ok();
    }));
}

fn forward_tray_events(proxy: EventLoopProxy<Message>) {
    TrayIconEvent::set_event_handler(Some(move |event| {
        proxy.send_event(Message::Tray(event)).ok();
    }));
}

fn forward_notification_clicks(proxy: EventLoopProxy<Message>) {
    platform::on_notification_click(move || {
        proxy.send_event(Message::NotificationClicked).ok();
    });
}

#[cfg(test)]
mod tests;
