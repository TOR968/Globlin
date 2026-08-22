use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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
mod tests {
    use super::*;

    #[test]
    fn defaults_ignore_the_self_updating_packages() {
        let config = Config::default();
        assert!(config.is_ignored("npm"));
        assert!(config.is_ignored("@anthropic-ai/claude-code"));
        assert!(!config.is_ignored("prettier"));
    }

    #[test]
    fn round_trips_through_json() {
        let config = Config {
            check_interval_hours: 12,
            last_notified: vec!["npm:prettier@3.10.0".to_string()],
            sources: Sources {
                npm: true,
                bun: false,
            },
            ..Default::default()
        };

        let body = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&body).unwrap();

        assert_eq!(restored.check_interval_hours, 12);
        assert_eq!(restored.last_notified, config.last_notified);
        assert!(!restored.sources.bun);
        assert!(restored.sources.npm);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let restored: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(restored.check_interval_hours, 6);
        assert!(restored.sources.npm);
        assert!(restored.sources.bun);
    }

    #[test]
    fn an_explicit_empty_ignore_list_is_respected() {
        let restored: Config = serde_json::from_str(r#"{"ignore":[]}"#).unwrap();
        assert!(!restored.is_ignored("npm"));
    }

    #[test]
    fn interval_never_collapses_to_zero() {
        let config = Config {
            check_interval_hours: 0,
            ..Default::default()
        };
        assert_eq!(config.interval(), Duration::from_hours(1));
    }

    #[test]
    fn ignoring_the_same_package_twice_does_not_duplicate_the_entry() {
        let mut config = Config::default();
        config.set_ignored("prettier", true);
        config.set_ignored("prettier", true);

        assert_eq!(
            config
                .ignore
                .iter()
                .filter(|name| *name == "prettier")
                .count(),
            1
        );
        assert!(config.is_ignored("prettier"));
    }

    #[test]
    fn un_ignoring_a_package_that_was_never_ignored_changes_nothing() {
        let mut config = Config::default();
        let before = config.ignore.clone();
        config.set_ignored("prettier", false);

        assert_eq!(config.ignore, before);
    }

    #[test]
    fn un_ignoring_removes_only_the_named_package() {
        let mut config = Config::default();
        config.set_ignored("npm", false);

        assert!(!config.is_ignored("npm"));
        assert!(config.is_ignored("@anthropic-ai/claude-code"));
    }

    #[test]
    fn self_updating_is_off_until_it_is_asked_for() {
        let config = Config::default();
        assert!(!config.auto_update);
        assert_eq!(config.last_self_notice, None);
    }

    #[test]
    fn an_older_config_file_without_the_new_fields_still_loads() {
        let raw = r#"{"check_interval_hours":3,"ignore":["npm"]}"#;
        let config: Config = serde_json::from_str(raw).unwrap();
        assert_eq!(config.check_interval_hours, 3);
        assert!(!config.auto_update);
    }
}
