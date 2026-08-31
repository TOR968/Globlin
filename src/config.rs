use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::install;
use crate::platform;
use crate::Result;

const FILE_NAME: &str = "globlin.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub check_interval_hours: u64,
    pub sources: Sources,
    pub ignore: Vec<String>,
    pub last_notified: Vec<String>,
    pub npm_cmd: Option<PathBuf>,
    pub auto_update: bool,
    pub last_self_notice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Sources {
    pub npm: bool,
    pub bun: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            check_interval_hours: 6,
            sources: Sources::default(),
            ignore: vec!["npm".to_string(), "@anthropic-ai/claude-code".to_string()],
            last_notified: Vec::new(),
            npm_cmd: None,
            auto_update: false,
            last_self_notice: None,
        }
    }
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            npm: true,
            bun: true,
        }
    }
}

impl Config {
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = read_path() else {
            return (Self::default(), None);
        };
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                return (
                    Self::default(),
                    Some(format!("Could not read {}: {error}", path.display())),
                )
            }
        };
        match serde_json::from_str(&raw) {
            Ok(config) => (config, None),
            Err(error) => (Self::default(), Some(quarantine(&path, &error.to_string()))),
        }
    }

    pub fn save(&self) -> Result<()> {
        let body = serde_json::to_string_pretty(self)?;
        let mut last_error = None;
        for path in write_paths() {
            match fs::write(&path, &body) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
        }
        Err(match last_error {
            Some(error) => Box::new(error) as crate::Error,
            None => "no writable location for the config file".into(),
        })
    }

    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.check_interval_hours.max(1) * 3600)
    }

    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignore.iter().any(|ignored| ignored == name)
    }

    pub fn set_ignored(&mut self, name: &str, ignored: bool) {
        if ignored {
            if !self.is_ignored(name) {
                self.ignore.push(name.to_string());
            }
        } else {
            self.ignore.retain(|entry| entry != name);
        }
    }
}

fn read_path() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|path| path.is_file())
}

fn write_paths() -> Vec<PathBuf> {
    candidate_paths()
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dir) = exe_dir() {
        paths.push(dir.join(FILE_NAME));
    }
    paths.push(platform::data_dir().join(FILE_NAME));
    paths
}

fn exe_dir() -> Option<PathBuf> {
    if install::winget_managed() {
        return None;
    }
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

fn quarantine(path: &PathBuf, error: &str) -> String {
    let backup = path.with_extension("json.invalid");
    match fs::rename(path, &backup) {
        Ok(()) => format!(
            "{} was not valid JSON ({error}); it was moved to {} and defaults are in use",
            path.display(),
            backup.display()
        ),
        Err(_) => format!(
            "{} was not valid JSON ({error}); defaults are in use",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests;
