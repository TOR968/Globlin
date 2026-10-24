use tray_icon::menu::MenuItem;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::icon::{self, IconState, Look};
use crate::platform;
use crate::Result;

mod menu;

pub use menu::{
    approve_id, close_update_id, dismiss_id, headline, ignore_id, key_of, remove_id, source_id,
    update_id, Action, SelfUpdate, View,
};

pub struct Tray {
    icon: TrayIcon,
    header: MenuItem,
    rows: Vec<(usize, MenuItem)>,
    look: Look,
}

impl Tray {
    pub fn new(monochrome: bool) -> Result<Self> {
        let view = View {
            packages: &[],
            activity: None,
            approvals: &[],
            in_use: &[],
            autostart: false,
            self_update: SelfUpdate::Own {
                release: None,
                auto_update: false,
                log: false,
            },
            pending_restart: None,
            frame: 0,
            elapsed: std::time::Duration::ZERO,
        };
        let built = menu::build(&view)?;
        let look = look_for(monochrome);
        let icon = TrayIconBuilder::new()
            .with_tooltip(menu::headline(&view))
            .with_icon(icon::tray(IconState::Busy, 0, 0.0, look)?)
            .with_icon_as_template(monochrome)
            .with_menu(Box::new(built.menu))
            .with_menu_on_left_click(false)
            .build()?;
        Ok(Self {
            icon,
            header: built.header,
            rows: built.rows,
            look,
        })
    }

    pub fn render(
        &mut self,
        view: &View,
        state: IconState,
        level: f32,
        monochrome: bool,
    ) -> Result<()> {
        let built = menu::build(view)?;
        self.icon.set_menu(Some(Box::new(built.menu)));
        self.header = built.header;
        self.rows = built.rows;
        self.icon.set_tooltip(Some(menu::headline(view)))?;
        self.look = look_for(monochrome);
        self.show(icon::tray(state, view.frame, level, self.look)?)
    }

    pub fn animate(&mut self, view: &View, level: f32) -> Result<()> {
        self.header.set_text(menu::headline(view));
        for (position, row) in &self.rows {
            if let Some(text) = menu::batch_row_text(view, *position) {
                row.set_text(text);
            }
        }
        self.show(icon::tray(IconState::Busy, view.frame, level, self.look)?)
    }

    fn show(&self, icon: Icon) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.icon
            .set_icon_with_as_template(Some(icon), matches!(self.look, Look::Mono(_)))?;
        #[cfg(not(target_os = "macos"))]
        self.icon.set_icon(Some(icon))?;
        Ok(())
    }
}

fn look_for(monochrome: bool) -> Look {
    icon::look(monochrome, monochrome && platform::tray_is_light())
}
