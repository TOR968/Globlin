use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Icon, Window as HostWindow, WindowBuilder, WindowId};
use wry::{WebView, WebViewBuilder};

use super::{script, tick_script, Snapshot, Tick, HEIGHT, TITLE, UI, WIDTH};
use crate::icon::{self, IconState};
use crate::{Message, Result};

const MIN_WIDTH: f64 = 680.0;
const MIN_HEIGHT: f64 = 440.0;
const ICON_SIZE: u32 = 64;

pub struct Window {
    host: HostWindow,
    webview: WebView,
}

impl Window {
    pub fn new(
        target: &EventLoopWindowTarget<Message>,
        proxy: EventLoopProxy<Message>,
    ) -> Result<Self> {
        let host = WindowBuilder::new()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .with_min_inner_size(LogicalSize::new(MIN_WIDTH, MIN_HEIGHT))
            .with_window_icon(app_icon())
            .with_visible(false)
            .build(target)?;
        let webview = WebViewBuilder::new()
            .with_html(UI)
            .with_ipc_handler(move |request| {
                proxy.send_event(Message::Ipc(request.into_body())).ok();
            })
            .build(&host)?;
        Ok(Self { host, webview })
    }

    pub fn id(&self) -> WindowId {
        self.host.id()
    }

    pub fn show(&self) {
        self.host.set_visible(true);
        self.host.set_focus();
    }

    pub fn hide(&self) {
        self.host.set_visible(false);
    }

    pub fn visible(&self) -> bool {
        self.host.is_visible()
    }

    pub fn render(&self, snapshot: &Snapshot) {
        self.webview.evaluate_script(&script(snapshot)).ok();
    }

    pub fn tick(&self, tick: &Tick) {
        self.webview.evaluate_script(&tick_script(tick)).ok();
    }
}

fn app_icon() -> Option<Icon> {
    Icon::from_rgba(
        icon::image(IconState::Idle, ICON_SIZE),
        ICON_SIZE,
        ICON_SIZE,
    )
    .ok()
}
