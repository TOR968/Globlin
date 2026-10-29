use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{
    executable_in, find_on_path, hidden_command, not_installed, plain_name, stdout_text,
    PackageSource,
};
use crate::config::Config;
use crate::model::{Installed, SourceKind};
use crate::{platform, process, Result};

const LOCK_FILE: &str = ".skill-lock.json";
const AGENTS_DIR: &str = ".agents";
const STATE_DIR: &str = "skills";
const LOCK_VERSION: u32 = 3;
const SHORT_HASH: usize = 7;
const NPX: &str = "npx";
const CLI: &str = "skills";
const NEWER: &str = "newer";
const CHECKING: &str = "Checking skills from source: ";
const FETCH_FAILED: &str = "✗ Failed to fetch tree for ";
const CHECK_FAILED: &str = "✗ Failed to check skills from ";
const FOUND: &str = "global update(s)";
const UPDATING: &str = "Updating ";
const UPDATED: &str = "✓ Updated ";
const BULLET: &str = "•";

pub struct Skills {
    lock: PathBuf,
    npx: Option<PathBuf>,
    ignored: Vec<String>,
}

impl Skills {
    pub fn new(config: &Config) -> Result<Self> {
        let lock = lock_path(
            std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            platform::home_dir(),
        )
        .ok_or_else(|| {
            not_installed("no home directory to look for the skills.sh lock file in".into())
        })?;
        Ok(Self {
            ignored: config.ignore.clone(),
            ..Self::at(lock, npx_beside(config.npm_cmd.as_deref()))?
        })
    }

    fn at(lock: PathBuf, npx: Option<PathBuf>) -> Result<Self> {
        if !lock.is_file() {
            return Err(not_installed(
                "the skills.sh lock file was not found".into(),
            ));
        }
        Ok(Self {
            lock,
            npx,
            ignored: Vec::new(),
        })
    }

    fn cli(&self, verb: &str, name: &str) -> Option<Command> {
        let name = safe_name(name)?;
        let mut command = hidden_command(self.npx.as_ref()?);
        command.args(["--yes", CLI, verb, name, "-g", "-y"]);
        Some(command)
    }

    fn check_command(&self, names: &[String]) -> Option<Command> {
        if names.is_empty() {
            return None;
        }
        let mut command = hidden_command(self.npx.as_ref()?);
        command
            .args(["--yes", CLI, "update"])
            .args(names)
            .args(["-g", "-y"]);
        Some(command)
    }

    fn names_to_check(&self, entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .filter(|name| !self.ignored.iter().any(|ignored| ignored == name))
            .filter_map(safe_name)
            .map(str::to_string)
            .collect()
    }
}

impl PackageSource for Skills {
    fn kind(&self) -> SourceKind {
        SourceKind::Skills
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let names = self.names_to_check(&parse_lock(&fs::read_to_string(&self.lock)?)?);
        let run = self
            .check_command(&names)
            .and_then(|mut command| {
                process::output_within(&mut command, process::CHANGE_LIMIT).ok()
            })
            .map(|output| Run::read(&stdout_text(&output), names));
        let entries = parse_lock(&fs::read_to_string(&self.lock)?)?;
        Ok(resolve(&entries, run.as_ref()))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        self.cli("update", name)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        self.cli("remove", name)
    }
}

fn safe_name(name: &str) -> Option<&str> {
    plain_name(name).filter(|name| !name.starts_with('-'))
}

fn npx_beside(npm_cmd: Option<&Path>) -> Option<PathBuf> {
    npm_cmd
        .filter(|path| path.is_file())
        .and_then(Path::parent)
        .and_then(|dir| executable_in(dir, NPX))
        .or_else(|| find_on_path(NPX))
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Entry {
    #[serde(skip)]
    name: String,
    source: String,
    #[serde(rename = "skillFolderHash")]
    folder_hash: String,
}

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    skills: Option<BTreeMap<String, Entry>>,
}

#[derive(Debug, Default)]
struct Run {
    asked: BTreeSet<String>,
    checked: BTreeSet<String>,
    unchecked: BTreeSet<String>,
    outdated: BTreeSet<String>,
}

impl Run {
    fn read(output: &str, asked: Vec<String>) -> Self {
        let mut run = Self {
            asked: asked.into_iter().collect(),
            ..Self::default()
        };
        let mut updating = false;
        for line in plain_lines(output) {
            if let Some(source) = line.strip_prefix(CHECKING) {
                run.checked.insert(source.to_string());
            } else if let Some(source) = line
                .strip_prefix(FETCH_FAILED)
                .or_else(|| line.strip_prefix(CHECK_FAILED))
            {
                run.checked.remove(source);
            } else if let Some(names) = line.strip_prefix(BULLET) {
                run.unchecked.extend(bullet_names(names));
            } else if line.ends_with(FOUND) {
                updating = true;
            } else if let Some(name) = line.strip_prefix(UPDATING).filter(|_| updating) {
                run.outdated.insert(name.trim_end_matches('…').to_string());
            } else if let Some(name) = line.strip_prefix(UPDATED) {
                run.outdated.remove(name);
            }
        }
        run
    }

    fn confirms(&self, entry: &Entry) -> bool {
        self.asked.contains(&entry.name)
            && self.checked.contains(&entry.source)
            && !self.unchecked.contains(&entry.name)
    }
}

fn plain_lines(output: &str) -> Vec<String> {
    let mut text = String::with_capacity(output.len());
    let mut chars = output.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            chars.find(char::is_ascii_alphabetic);
        } else {
            text.push(character);
        }
    }
    text.split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn bullet_names(line: &str) -> impl Iterator<Item = String> + '_ {
    line.split(" (")
        .next()
        .unwrap_or_default()
        .split(", ")
        .map(|name| name.trim().to_string())
}

fn lock_path(xdg_state_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    match xdg_state_home {
        Some(state) => Some(state.join(STATE_DIR).join(LOCK_FILE)),
        None => home.map(|home| home.join(AGENTS_DIR).join(LOCK_FILE)),
    }
}

fn parse_lock(raw: &str) -> Result<Vec<Entry>> {
    let lock: Lock = serde_json::from_str(raw)?;
    if lock.version < LOCK_VERSION {
        return Ok(Vec::new());
    }
    Ok(lock
        .skills
        .unwrap_or_default()
        .into_iter()
        .map(|(name, entry)| Entry { name, ..entry })
        .collect())
}

fn resolve(entries: &[Entry], run: Option<&Run>) -> Vec<Installed> {
    entries
        .iter()
        .map(|entry| {
            let installed =
                Installed::new(&entry.name, short(&entry.folder_hash), SourceKind::Skills);
            match run {
                Some(run) if run.outdated.contains(&entry.name) => {
                    installed.with_available(Some(NEWER.to_string()))
                }
                Some(run) if run.confirms(entry) => installed,
                _ => installed.with_external(true),
            }
        })
        .collect()
}

fn short(hash: &str) -> String {
    hash.chars().take(SHORT_HASH).collect()
}

#[cfg(test)]
mod tests;
