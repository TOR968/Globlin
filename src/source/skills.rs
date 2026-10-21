use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{
    executable_in, find_on_path, hidden_command, not_installed, plain_name, PackageSource,
};
use crate::config::Config;
use crate::model::{Installed, SourceKind};
use crate::{platform, registry, Result};

const LOCK_FILE: &str = ".skill-lock.json";
const AGENTS_DIR: &str = ".agents";
const STATE_DIR: &str = "skills";
const LOCK_VERSION: u32 = 3;
const CHECKABLE_TYPE: &str = "github";
const SKILL_FILE: &str = "skill.md";
const SHORT_HASH: usize = 7;
const NPX: &str = "npx";
const CLI: &str = "skills";

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
        let name = plain_name(name).filter(|name| !name.starts_with('-'))?;
        let mut command = hidden_command(self.npx.as_ref()?);
        command.args(["--yes", CLI, verb, name, "-g", "-y"]);
        Some(command)
    }
}

impl PackageSource for Skills {
    fn kind(&self) -> SourceKind {
        SourceKind::Skills
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let entries = parse_lock(&fs::read_to_string(&self.lock)?)?;
        let repos = repos_to_check(&entries, |name| {
            self.ignored.iter().any(|ignored| ignored == name)
        });
        Ok(resolve(&entries, &registry::github_trees(&repos)))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        self.cli("update", name)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        self.cli("remove", name)
    }
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
    source_type: String,
    skill_path: String,
    #[serde(rename = "skillFolderHash")]
    folder_hash: String,
    #[serde(rename = "ref")]
    git_ref: Option<String>,
}

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    skills: Option<BTreeMap<String, Entry>>,
}

#[derive(Deserialize)]
struct Tree {
    sha: String,
    tree: Vec<TreeNode>,
}

#[derive(Deserialize)]
struct TreeNode {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
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

fn repos_to_check(entries: &[Entry], ignored: impl Fn(&str) -> bool) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !ignored(&entry.name))
        .filter_map(repo_key)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn repo_key(entry: &Entry) -> Option<String> {
    let checkable = entry.source_type == CHECKABLE_TYPE
        && !entry.source.is_empty()
        && !entry.folder_hash.is_empty()
        && !entry.skill_path.is_empty();
    if !checkable {
        return None;
    }
    Some(match &entry.git_ref {
        Some(git_ref) => format!("{}#{git_ref}", entry.source),
        None => entry.source.clone(),
    })
}

fn folder_hash(tree_json: &str, skill_path: &str) -> Option<String> {
    let tree: Tree = serde_json::from_str(tree_json).ok()?;
    let folder = skill_folder(skill_path);
    if folder.is_empty() {
        return Some(tree.sha);
    }
    tree.tree
        .into_iter()
        .find(|node| node.kind == "tree" && node.path == folder)
        .map(|node| node.sha)
}

fn skill_folder(skill_path: &str) -> String {
    let path = skill_path.replace('\\', "/");
    let folder = path
        .len()
        .checked_sub(SKILL_FILE.len())
        .filter(|start| {
            path.get(*start..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(SKILL_FILE))
        })
        .map_or(path.as_str(), |start| &path[..start]);
    folder.trim_end_matches('/').to_string()
}

fn resolve(entries: &[Entry], trees: &HashMap<String, String>) -> Vec<Installed> {
    entries
        .iter()
        .map(|entry| {
            let installed =
                Installed::new(&entry.name, short(&entry.folder_hash), SourceKind::Skills);
            let upstream = repo_key(entry)
                .and_then(|key| trees.get(&key))
                .and_then(|tree| folder_hash(tree, &entry.skill_path));
            match upstream {
                None => installed.with_external(true),
                Some(hash) if hash == entry.folder_hash => installed,
                Some(hash) => installed.with_available(Some(short(&hash))),
            }
        })
        .collect()
}

fn short(hash: &str) -> String {
    hash.chars().take(SHORT_HASH).collect()
}

#[cfg(test)]
mod tests;
