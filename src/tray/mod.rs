use tray_icon::menu::MenuItem;
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::icon::{self, IconState};
use crate::Result;

mod menu;

pub use menu::{Action, View};

pub struct Tray {
    icon: TrayIcon,
    header: MenuItem,
    rows: Vec<MenuItem>,
}

impl Tray {
    pub fn new() -> Result<Self> {
        let view = View {
            packages: &[],
            activity: None,
            autostart: false,
            auto_update: false,
            release: None,
            frame: 0,
            elapsed: std::time::Duration::ZERO,
        };
        let built = menu::build(&view)?;
        let icon = TrayIconBuilder::new()
            .with_tooltip(menu::headline(&view))
            .with_icon(icon::tray(IconState::Busy, 0, 0.0)?)
            .with_menu(Box::new(built.menu))
            .build()?;
        Ok(Self {
            icon,
            header: built.header,
            rows: built.rows,
        })
    }

    pub fn render(&mut self, view: &View, state: IconState, level: f32) -> Result<()> {
        let built = menu::build(view)?;
        self.icon.set_menu(Some(Box::new(built.menu)));
        self.header = built.header;
        self.rows = built.rows;
        self.icon.set_tooltip(Some(menu::headline(view)))?;
        self.icon
            .set_icon(Some(icon::tray(state, view.frame, level)?))?;
        Ok(())
    }

    pub fn animate(&mut self, view: &View, level: f32) -> Result<()> {
        self.header.set_text(menu::headline(view));
        for (position, row) in self.rows.iter().enumerate() {
            if let Some(text) = menu::batch_row_text(view, position) {
                row.set_text(text);
            }
        }
        self.icon
            .set_icon(Some(icon::tray(IconState::Busy, view.frame, level)?))?;
        Ok(())
    }
}
