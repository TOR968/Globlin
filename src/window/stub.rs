use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::WindowId;

use super::{Snapshot, Tick};
use crate::{Message, Result};

pub struct Window;

impl Window {
    pub fn new(
        _target: &EventLoopWindowTarget<Message>,
        _proxy: EventLoopProxy<Message>,
    ) -> Result<Self> {
        Err("the Globlin window is only available on Windows".into())
    }

    pub fn id(&self) -> WindowId {
        unreachable!()
    }

    pub const fn show(&self) {}

    pub const fn hide(&self) {}

    pub const fn visible(&self) -> bool {
        false
    }

    pub const fn render(&self, _snapshot: &Snapshot) {}

    pub const fn tick(&self, _tick: &Tick) {}
}
