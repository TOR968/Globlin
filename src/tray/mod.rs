use tray_icon::menu::MenuItem;
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::icon::{self, IconState};
use crate::Result;

mod menu;

pub use menu::{Action, View};

pub struct Tray {
    icon: TrayIcon,
    header: MenuItem,
}

impl Tray {
    pub fn new() -> Result<Self> {
        let view = View {
            packages: &[],
            activity: None,
            autostart: false,
            frame: 0,
        };
        let built = menu::build(&view)?;
        let icon = TrayIconBuilder::new()
            .with_tooltip(menu::headline(&view))
            .with_icon(icon::tray(IconState::Busy, 0)?)
            .with_menu(Box::new(built.menu))
            .build()?;
        Ok(Self {
            icon,
            header: built.header,
        })
    }

    pub fn render(&mut self, view: &View, state: IconState) -> Result<()> {
        let built = menu::build(view)?;
        self.icon.set_menu(Some(Box::new(built.menu)));
        self.header = built.header;
        self.icon.set_tooltip(Some(menu::headline(view)))?;
        self.icon.set_icon(Some(icon::tray(state, view.frame)?))?;
        Ok(())
    }

    pub fn animate(&mut self, view: &View) -> Result<()> {
        self.header.set_text(menu::headline(view));
        self.icon
            .set_icon(Some(icon::tray(IconState::Busy, view.frame)?))?;
        Ok(())
    }
}
