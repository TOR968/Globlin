use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "winget.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "winget";

const NAME: usize = 0;
const ID: usize = 1;
const VERSION: usize = 2;
const AVAILABLE: usize = 3;
const WITH_AVAILABLE: usize = 5;
const SEPARATOR_WIDTH: usize = 10;

pub struct Winget {
    command: PathBuf,
}

impl Winget {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("winget was not found on PATH")?;
        Ok(Self { command })
    }
}

impl PackageSource for Winget {
    fn kind(&self) -> SourceKind {
        SourceKind::Winget
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = hidden_command(&self.command)
            .args([
                "list",
                "--disable-interactivity",
                "--accept-source-agreements",
            ])
            .output()?;
        Ok(parse_table(&String::from_utf8_lossy(&output.stdout)))
    }

    fn update_command(&self, _name: &str) -> Option<Command> {
        None
    }

    fn uninstall_command(&self, _name: &str) -> Option<Command> {
        None
    }
}

fn parse_table(stdout: &str) -> Vec<Installed> {
    let lines: Vec<&str> = stdout.trim_start_matches('\u{feff}').lines().collect();
    let Some(separator) = lines.iter().position(|line| is_separator(line)) else {
        return Vec::new();
    };
    let Some(starts) = separator.checked_sub(1).map(|index| columns(lines[index])) else {
        return Vec::new();
    };
    if starts.len() < 4 {
        return Vec::new();
    }
    lines[separator + 1..]
        .iter()
        .filter_map(|line| to_installed(&split_row(line, &starts), starts.len()))
        .collect()
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= SEPARATOR_WIDTH && trimmed.chars().all(|glyph| glyph == '-')
}

fn columns(header: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut previous_blank = true;
    for (index, glyph) in header.chars().enumerate() {
        if glyph == ' ' {
            previous_blank = true;
        } else {
            if previous_blank {
                starts.push(index);
            }
            previous_blank = false;
        }
    }
    starts
}

fn split_row(line: &str, starts: &[usize]) -> Vec<String> {
    let glyphs: Vec<char> = line.chars().collect();
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(glyphs.len());
            slice(&glyphs, *start, end)
        })
        .collect()
}

fn slice(glyphs: &[char], start: usize, end: usize) -> String {
    let start = start.min(glyphs.len());
    let end = end.min(glyphs.len());
    glyphs[start..end].iter().collect::<String>().trim().into()
}

fn to_installed(row: &[String], width: usize) -> Option<Installed> {
    let id = row.get(ID)?;
    let version = row.get(VERSION)?;
    if id.is_empty() || version.is_empty() || row.get(NAME)?.is_empty() {
        return None;
    }
    let available = if width >= WITH_AVAILABLE {
        row.get(AVAILABLE).filter(|cell| !cell.is_empty()).cloned()
    } else {
        None
    };
    Some(Installed::new(id, version, SourceKind::Winget).with_available(available))
}

#[cfg(test)]
mod tests;
