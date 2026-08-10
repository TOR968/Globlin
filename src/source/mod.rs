use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::{Installed, SourceKind};
use crate::Result;

mod bun;
mod npm;

pub use bun::Bun;
pub use npm::Npm;

pub trait PackageSource {
    fn kind(&self) -> SourceKind;

    fn installed(&self) -> Result<Vec<Installed>>;

    fn update_command(&self, name: &str) -> Command;
}

pub(crate) fn find_on_path(file_name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(file_name))
        .find(|candidate| candidate.is_file())
}

#[cfg(windows)]
pub(crate) fn hidden_command(program: &Path) -> Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(windows))]
pub(crate) fn hidden_command(program: &Path) -> Command {
    Command::new(program)
}
