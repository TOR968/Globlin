#![windows_subsystem = "windows"]

mod app;
mod check;
mod config;
mod diagnostics;
mod icon;
mod model;
mod notice;
mod platform;
mod registry;
mod source;
mod tray;
mod update;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;

use app::{App, Control};
use model::Package;
use update::{Outcome, Progress};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub enum Message {
    Menu(MenuEvent),
    Checked(Result<Vec<Package>>),
    Progress(Progress),
    Updated(Outcome),
}

fn main() {
    if !platform::claim_single_instance() {
        return;
    }
    if let Err(error) = run() {
        platform::notify("npm globals could not start", &error.to_string()).ok();
    }
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
