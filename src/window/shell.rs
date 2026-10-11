use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Icon, Window as HostWindow, WindowBuilder, WindowId};
use wry::{NewWindowResponse, WebView, WebViewBuilder};

use super::{is_own_page, script, tick_script, Snapshot, Tick, HEIGHT, TITLE, UI, WIDTH};
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
        let builder = WebViewBuilder::new()
            .with_html(UI)
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

fn app_icon() -> Option<Icon> {
    Icon::from_rgba(
        icon::image(IconState::Idle, ICON_SIZE),
        ICON_SIZE,
        ICON_SIZE,
    )
    .ok()
}
