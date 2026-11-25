use tao::dpi::{LogicalSize, PhysicalPosition};
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Icon, Theme, Window as HostWindow, WindowBuilder, WindowId};
use wry::{NewWindowResponse, WebView, WebViewBuilder};

use super::{
    is_own_page, on_a_screen, script, tick_script, Screen, Snapshot, Tick, HEIGHT, TITLE, UI, WIDTH,
};
use crate::icon::{self, IconState};
use crate::{Message, Result};

const DARK_BACKGROUND: (u8, u8, u8, u8) = (15, 18, 21, 255);
const LIGHT_BACKGROUND: (u8, u8, u8, u8) = (244, 245, 247, 255);
const MIN_WIDTH: f64 = 760.0;
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
        size: Option<(u32, u32)>,
        position: Option<(i32, i32)>,
    ) -> Result<Self> {
        let (width, height) = size.map_or((WIDTH, HEIGHT), |(width, height)| {
            (f64::from(width), f64::from(height))
        });
        let screens: Vec<Screen> = target
            .available_monitors()
            .map(|monitor| {
                let origin = monitor.position();
                let extent = monitor.size();
                ((origin.x, origin.y), (extent.width, extent.height))
            })
            .collect();
        let mut builder = WindowBuilder::new()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(MIN_WIDTH, MIN_HEIGHT))
            .with_window_icon(app_icon())
            .with_visible(false);
        if let Some((x, y)) = position.filter(|&point| on_a_screen(point, &screens)) {
            builder = builder.with_position(PhysicalPosition::new(x, y));
        }
        let host = with_taskbar_icon(builder).build(target)?;
        let builder = WebViewBuilder::new()
            .with_html(UI)
            .with_background_color(if host.theme() == Theme::Dark {
                DARK_BACKGROUND
            } else {
                LIGHT_BACKGROUND
            })
            .with_navigation_handler(|uri| is_own_page(&uri))
            .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
            .with_drag_drop_handler(|_| true)
            .with_ipc_handler(move |request| {
                if is_own_page(&request.uri().to_string()) {
                    proxy.send_event(Message::Ipc(request.into_body())).ok();
                }
            });
        let webview = attach(builder, &host)?;
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

    pub fn size(&self) -> (u32, u32) {
        let logical = self
            .host
            .inner_size()
            .to_logical::<u32>(self.host.scale_factor());
        (logical.width, logical.height)
    }

    pub fn position(&self) -> Option<(i32, i32)> {
        self.host
            .outer_position()
            .ok()
            .map(|position| (position.x, position.y))
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

#[cfg(target_os = "macos")]
pub fn show_in_dock(target: &EventLoopWindowTarget<Message>, visible: bool) {
    use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};

    target.set_activation_policy_at_runtime(if visible {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    });
}

#[cfg(not(target_os = "macos"))]
pub const fn show_in_dock(_target: &EventLoopWindowTarget<Message>, _visible: bool) {}

#[cfg(not(target_os = "linux"))]
fn attach<'a>(builder: WebViewBuilder<'a>, host: &'a HostWindow) -> Result<WebView> {
    Ok(builder.build(host)?)
}

#[cfg(target_os = "linux")]
fn attach<'a>(builder: WebViewBuilder<'a>, host: &'a HostWindow) -> Result<WebView> {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;

    let container = host
        .default_vbox()
        .ok_or("the window has no GTK container to hold the page")?;
    Ok(builder.build_gtk(container)?)
}

#[cfg(windows)]
fn with_taskbar_icon(builder: WindowBuilder) -> WindowBuilder {
    use tao::platform::windows::WindowBuilderExtWindows;

    builder.with_taskbar_icon(app_icon())
}

#[cfg(not(windows))]
fn with_taskbar_icon(builder: WindowBuilder) -> WindowBuilder {
    builder
}

fn app_icon() -> Option<Icon> {
    Icon::from_rgba(
        icon::image(IconState::Idle, ICON_SIZE),
        ICON_SIZE,
        ICON_SIZE,
    )
    .ok()
}
