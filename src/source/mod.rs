use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
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

pub fn enabled(config: &Config) -> Result<Vec<Box<dyn PackageSource>>> {
    let mut sources: Vec<Box<dyn PackageSource>> = Vec::new();
    let mut failures = Vec::new();

    if config.sources.npm {
        match Npm::new(config.npm_cmd.as_deref()) {
            Ok(source) => sources.push(Box::new(source)),
            Err(error) => failures.push(error.to_string()),
        }
    }
    if config.sources.bun {
        match Bun::new() {
            Ok(source) => sources.push(Box::new(source)),
            Err(error) => failures.push(error.to_string()),
        }
    }

    if sources.is_empty() {
        return Err(if failures.is_empty() {
            "no package sources are enabled in the config".into()
        } else {
            failures.join("; ").into()
        });
    }
    Ok(sources)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Sources;

    fn with_sources(npm: bool, bun: bool) -> Config {
        Config {
            sources: Sources { npm, bun },
            ..Default::default()
        }
    }

    #[test]
    fn disabling_every_source_is_an_error_rather_than_an_empty_result() {
        let error = enabled(&with_sources(false, false))
            .err()
            .expect("disabling every source should fail")
            .to_string();
        assert!(error.contains("no package sources are enabled"), "{error}");
    }

    #[test]
    fn npm_alone_is_enough_to_run() {
        let sources = enabled(&with_sources(true, false)).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].kind(), SourceKind::Npm);
    }

    #[test]
    fn a_missing_optional_source_does_not_break_the_working_one() {
        let sources = enabled(&with_sources(true, true)).unwrap();
        assert!(sources
            .iter()
            .any(|source| source.kind() == SourceKind::Npm));
    }

    #[test]
    fn a_hidden_command_targets_the_program_it_was_given() {
        let command = hidden_command(Path::new("npm.cmd"));
        assert_eq!(command.get_program(), "npm.cmd");
    }

    #[test]
    fn a_program_that_is_not_on_path_is_not_found() {
        assert_eq!(find_on_path("npm-globals-tray-absent-4b1e.exe"), None);
    }
}
