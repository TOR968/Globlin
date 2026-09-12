use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "go.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "go";

const DEVEL: &str = "(devel)";

pub struct Go {
    command: PathBuf,
}

struct Binary {
    package: String,
    module: String,
    version: String,
}

impl Go {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("go was not found on PATH")?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> String {
        hidden_command(&self.command)
            .args(arguments)
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default()
    }

    fn bin_dir(&self) -> Option<PathBuf> {
        for variable in ["GOBIN", "GOPATH"] {
            let reported = self.run(&["env", variable]);
            let path = reported.trim();
            if !path.is_empty() {
                let candidate = PathBuf::from(path);
                let candidate = if variable == "GOPATH" {
                    candidate.join("bin")
                } else {
                    candidate
                };
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn latest(&self, module: &str) -> Option<String> {
        let query = format!("{module}@latest");
        let reported = self.run(&["list", "-m", "-f", "{{.Version}}", &query]);
        let version = reported.trim().trim_start_matches('v');
        (!version.is_empty() && version != DEVEL).then(|| version.to_string())
    }
}

impl PackageSource for Go {
    fn kind(&self) -> SourceKind {
        SourceKind::Go
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let Some(bin) = self.bin_dir() else {
            return Ok(Vec::new());
        };
        let reported = self.run(&["version", "-m", &bin.to_string_lossy()]);
        Ok(parse_listing(&reported)
            .into_iter()
            .map(|binary| {
                let available = self.latest(&binary.module);
                Installed::new(binary.package, binary.version, SourceKind::Go)
                    .with_available(available)
            })
            .collect())
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["install", &format!("{name}@latest")]);
        Some(command)
    }

    fn uninstall_command(&self, _name: &str) -> Option<Command> {
        None
    }
}

fn parse_listing(stdout: &str) -> Vec<Binary> {
    let mut binaries = Vec::new();
    let mut package = None;
    for line in stdout.lines() {
        let Some(fields) = tagged(line) else {
            package = None;
            continue;
        };
        match fields.as_slice() {
            ["path", path] => package = Some((*path).to_string()),
            ["mod", module, version, ..] => {
                if let Some(path) = package.take() {
                    if *version != DEVEL {
                        binaries.push(Binary {
                            package: path,
                            module: (*module).to_string(),
                            version: version.trim_start_matches('v').to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    binaries
}

fn tagged(line: &str) -> Option<Vec<&str>> {
    let fields: Vec<&str> = line.strip_prefix('\t')?.split('\t').collect();
    (fields.len() >= 2).then_some(fields)
}

#[cfg(test)]
mod tests;
