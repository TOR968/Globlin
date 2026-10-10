use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::WindowId;

use super::{Snapshot, Tick};
use crate::{Message, Result};

pub enum Window {}

impl Window {
    pub fn new(
        _target: &EventLoopWindowTarget<Message>,
        _proxy: EventLoopProxy<Message>,
    ) -> Result<Self> {
        Err("the Globlin window is only available on Windows".into())
    }

    pub fn id(&self) -> WindowId {
        match *self {}
    }

    pub fn show(&self) {
        match *self {}
    }

    pub fn hide(&self) {
        match *self {}
    }

    pub fn visible(&self) -> bool {
        match *self {}
    }

    pub fn render(&self, _snapshot: &Snapshot) {
        match *self {}
    }

    pub fn tick(&self, _tick: &Tick) {
        match *self {}
    }
}
