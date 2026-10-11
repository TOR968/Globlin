use std::path::{Path, PathBuf};
use std::process::Command;

use super::{find_on_path, hidden_command, listed, run_listing, stdout_text, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "go";

const DEVEL: &str = "(devel)";

pub struct Go {
    command: PathBuf,
}

struct Binary {
    file: PathBuf,
    package: String,
    module: String,
    version: String,
}

impl Go {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("go was not found on PATH")?;
        Ok(Self { command })
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = hidden_command(&self.command);
        command
            .args(arguments)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never");
        command
    }

    fn run(&self, arguments: &[&str]) -> String {
        run_listing(&mut self.command(arguments))
            .map(|output| stdout_text(&output))
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

    fn remove_binary(&self, name: &str) -> Result<()> {
        let bin = self
            .bin_dir()
            .ok_or("the go bin directory could not be found")?;
        let output = run_listing(&mut self.command(&["version", "-m", &bin.to_string_lossy()]))?;
        let file = locate(&parse_listing(&stdout_text(&output)), &bin, name)?;
        std::fs::remove_file(file)?;
        Ok(())
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
        let output = run_listing(&mut self.command(&["version", "-m", &bin.to_string_lossy()]))?;
        let installed = parse_listing(&stdout_text(&output))
            .into_iter()
            .map(|binary| {
                let available = self.latest(&binary.module);
                Installed::new(binary.package, binary.version, SourceKind::Go)
                    .with_available(available)
            })
            .collect();
        listed(installed, "go version -m", &output)
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        Some(self.command(&["install", &format!("{name}@latest")]))
    }

    fn uninstall_command(&self, _name: &str) -> Option<Command> {
        None
    }

    fn remove_directly(&self, name: &str) -> Option<Result<()>> {
        Some(self.remove_binary(name))
    }
}

fn locate(binaries: &[Binary], bin: &Path, name: &str) -> Result<PathBuf> {
    let binary = binaries
        .iter()
        .find(|binary| binary.package == name)
        .ok_or_else(|| format!("{name} was not found in {}", bin.display()))?;
    if !is_directly_inside(&binary.file, bin) {
        return Err(format!(
            "{} is not inside {}, so it was left alone",
            binary.file.display(),
            bin.display()
        )
        .into());
    }
    Ok(binary.file.clone())
}

fn is_directly_inside(file: &Path, directory: &Path) -> bool {
    let Some(parent) = file.parent() else {
        return false;
    };
    let resolve = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    resolve(parent) == resolve(directory)
}

fn header_file(line: &str) -> Option<PathBuf> {
    let (file, _) = line.rsplit_once(": go")?;
    (!file.is_empty()).then(|| PathBuf::from(file))
}

fn parse_listing(stdout: &str) -> Vec<Binary> {
    let mut binaries = Vec::new();
    let mut file = None;
    let mut package = None;
    for line in stdout.lines() {
        let Some(fields) = tagged(line) else {
            file = header_file(line);
            package = None;
            continue;
        };
        match fields.as_slice() {
            ["path", path] => package = Some((*path).to_string()),
            ["mod", module, version, ..] => {
                if let (Some(path), Some(file)) = (package.take(), file.clone()) {
                    if *version != DEVEL {
                        binaries.push(Binary {
                            file,
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
