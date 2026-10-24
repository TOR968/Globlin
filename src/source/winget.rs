use std::path::PathBuf;
use std::process::{Command, Output};

use super::{
    find_on_path, hidden_command, listed, not_on_path, plain_name, quoted, run_listing,
    stdout_text, PackageSource,
};
use crate::model::{Installed, Refusal, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "winget";
const OWN_NAME: &str = "Globlin";
const UNKNOWN_VERSION: &str = "Unknown";
const REGISTRY_ONLY: &str = "ARP\\";

const SHELLS: [&str; 2] = ["powershell", "pwsh"];
const BROAD_DIRS: [&str; 9] = [
    "SystemDrive",
    "SystemRoot",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "ProgramData",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
];
const PROCESSES: &str = "Get-Process | Where-Object { $_.Path -and \
    $_.Path.StartsWith($root, [StringComparison]::OrdinalIgnoreCase) }";
const SERVICES: &str = "Get-CimInstance Win32_Service | Where-Object { $_.State -eq 'Running' -and \
    $_.PathName -and $_.PathName.TrimStart([char]34).StartsWith($root, [StringComparison]::OrdinalIgnoreCase) }";
const SERVICE_SUFFIX: &str = " (service)";
const UPGRADE_FLAGS: &str =
    "--exact --silent --disable-interactivity --accept-source-agreements --accept-package-agreements";
const DIAGNOSTICS: &str =
    "\\Packages\\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\\LocalState\\DiagOutputDir";

const UPDATE_NOT_APPLICABLE: u32 = 0x8A15_002B;
const PORTABLE_INSTALL_FAILED: u32 = 0x8A15_0052;
const INSTALL_IN_PROGRESS: u32 = 0x8A15_0102;
const INSTALL_CANCELLED_BY_USER: u32 = 0x8A15_010C;
const OPERATION_CANCELLED: u32 = 0x8007_04C7;

const NAME: usize = 0;
const ID: usize = 1;
const VERSION: usize = 2;
const AVAILABLE: usize = 3;
const WITH_AVAILABLE: usize = 5;
const SEPARATOR_WIDTH: usize = 10;

pub struct Winget {
    command: PathBuf,
    shell: Option<PathBuf>,
}

impl Winget {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("winget"))?;
        let shell = SHELLS.into_iter().find_map(find_on_path);
        Ok(Self { command, shell })
    }

    fn elevated_upgrade(&self, ids: &[&str]) -> Option<Command> {
        let ids = ids
            .iter()
            .map(|id| plain_name(id).map(|id| format!("'{id}'")))
            .collect::<Option<Vec<String>>>()?
            .join(", ");
        let inner = format!(
            "$codes = @(); foreach ($id in {ids}) {{ & {} upgrade --id $id {UPGRADE_FLAGS}; \
             if ($LASTEXITCODE) {{ $codes += $LASTEXITCODE }} }}; \
             $shared = @($codes | Select-Object -Unique); \
             if ($shared.Count -eq 1) {{ exit $shared[0] }}; exit [int]($codes.Count -gt 0)",
            quoted(&self.command.to_string_lossy()),
        );
        self.elevated(
            &inner,
            &format!("'winget exited with ' + $run.ExitCode + '; its own logs are in ' + $env:LOCALAPPDATA + '{DIAGNOSTICS}'"),
        )
    }

    fn elevated(&self, inner: &str, failure: &str) -> Option<Command> {
        let shell = self.shell.as_ref()?;
        self.in_shell(&format!(
            "$ErrorActionPreference = 'Stop'; \
             $run = Start-Process -FilePath {} -ArgumentList '-NoProfile', '-NonInteractive', '-Command', {} \
             -Verb RunAs -WindowStyle Hidden -Wait -PassThru; \
             if ($run.ExitCode) {{ [Console]::Error.WriteLine({failure}) }}; \
             exit $run.ExitCode",
            quoted(&shell.to_string_lossy()),
            quoted(inner),
        ))
    }

    fn in_shell(&self, script: &str) -> Option<Command> {
        let mut command = hidden_command(self.shell.as_ref()?);
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        Some(command)
    }

    fn location(&self, id: &str) -> Option<String> {
        let output = run_listing(hidden_command(&self.command).args([
            "list",
            "--id",
            id,
            "--exact",
            "--details",
            "--disable-interactivity",
            "--accept-source-agreements",
        ]))
        .ok()?;
        narrow_location(&installed_location(&stdout_text(&output))?, &broad_dirs())
    }

    fn change(&self, verb: &str, id: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args([
            verb,
            "--id",
            id,
            "--exact",
            "--silent",
            "--disable-interactivity",
            "--accept-source-agreements",
        ]);
        command
    }
}

impl PackageSource for Winget {
    fn kind(&self) -> SourceKind {
        SourceKind::Winget
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = run_listing(hidden_command(&self.command).args([
            "list",
            "--disable-interactivity",
            "--accept-source-agreements",
        ]))?;
        listed(parse_table(&stdout_text(&output)), "winget list", &output)
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = self.change("upgrade", name);
        command.arg("--accept-package-agreements");
        Some(command)
    }

    fn update_together(&self, names: &[&str]) -> Option<Command> {
        if names.is_empty() {
            return None;
        }
        self.elevated_upgrade(names)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        Some(self.change("uninstall", name))
    }

    fn refusal(&self, output: &Output) -> Option<Refusal> {
        refusal_for(output.status.code()?)
    }

    fn running(&self, name: &str) -> Vec<String> {
        self.location(name)
            .and_then(|location| self.in_shell(&running_script(&location)))
            .and_then(|mut command| run_listing(&mut command).ok())
            .filter(|output| output.status.success())
            .map(|output| {
                stdout_text(&output)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn close_running(&self, name: &str) -> Result<()> {
        let location = self
            .location(name)
            .ok_or_else(|| format!("{name}: could not find where winget installed it"))?;
        let script = close_script(&location);
        let services = self
            .running(name)
            .iter()
            .any(|program| program.ends_with(SERVICE_SUFFIX));
        let command = if services {
            self.elevated(
                &script,
                "'closing its programs failed with ' + $run.ExitCode",
            )
        } else {
            self.in_shell(&script)
        };
        let output = super::run_change(command, name, SourceKind::Winget)?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "{name}: could not close its programs: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into())
    }
}

fn running_script(location: &str) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'; $root = {}; \
         {PROCESSES} | ForEach-Object {{ $_.ProcessName }}; \
         {SERVICES} | ForEach-Object {{ $_.Name + '{SERVICE_SUFFIX}' }}",
        quoted(location),
    )
}

fn close_script(location: &str) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'; $root = {}; \
         {SERVICES} | ForEach-Object {{ Stop-Service -Name $_.Name -Force }}; \
         {PROCESSES} | Stop-Process -Force",
        quoted(location),
    )
}

fn installed_location(details: &str) -> Option<String> {
    details
        .lines()
        .find_map(|line| line.trim().strip_prefix("Installed Location:"))
        .map(str::trim)
        .filter(|location| !location.is_empty())
        .map(String::from)
}

fn narrow_location(location: &str, broad: &[String]) -> Option<String> {
    let folder = |path: &str| format!("{}\\", path.trim_end_matches('\\')).to_lowercase();
    let location = folder(location);
    let covers_a_shared_folder = broad.iter().any(|dir| folder(dir).starts_with(&location));
    (!covers_a_shared_folder).then_some(location)
}

fn broad_dirs() -> Vec<String> {
    let mut dirs: Vec<String> = BROAD_DIRS
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .collect();
    if let Ok(programs) = std::env::var("ProgramFiles") {
        dirs.push(format!("{programs}\\WindowsApps"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(format!("{local}\\Programs"));
        dirs.push(format!("{local}\\Microsoft\\WinGet\\Packages"));
    }
    dirs
}

fn refusal_for(code: i32) -> Option<Refusal> {
    match code.cast_unsigned() {
        UPDATE_NOT_APPLICABLE => Some(Refusal::NotApplicable),
        INSTALL_CANCELLED_BY_USER | OPERATION_CANCELLED => Some(Refusal::Cancelled),
        INSTALL_IN_PROGRESS => Some(Refusal::Busy),
        PORTABLE_INSTALL_FAILED => Some(Refusal::InUse),
        _ => None,
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
    let name = row.get(NAME)?;
    if id.is_empty() || version.is_empty() || name.is_empty() || name == OWN_NAME {
        return None;
    }
    if version == UNKNOWN_VERSION && id.starts_with(REGISTRY_ONLY) {
        return None;
    }
    let available = if width >= WITH_AVAILABLE {
        row.get(AVAILABLE).filter(|cell| !cell.is_empty()).cloned()
    } else {
        None
    };
    Some(
        Installed::new(id, version, SourceKind::Winget)
            .with_available(available)
            .with_external(version == UNKNOWN_VERSION),
    )
}

#[cfg(test)]
mod tests;
