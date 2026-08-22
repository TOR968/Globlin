use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

const NOT_IMPLEMENTED: &str = "this build only implements the Windows platform arm; see README.md";

pub fn claim_single_instance() -> bool {
    true
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("globlin");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn notify(_title: &str, _body: &str) -> Result<()> {
    Err(NOT_IMPLEMENTED.into())
}

pub fn autostart_enabled() -> bool {
    false
}

pub fn set_autostart(_enabled: bool) -> Result<()> {
    Err(NOT_IMPLEMENTED.into())
}

pub fn open_in_shell(_path: &Path) -> Result<()> {
    Err(NOT_IMPLEMENTED.into())
}
