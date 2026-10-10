use std::path::Path;

use crate::platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Winget,
    SystemPackage,
}

impl Manager {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Winget => "winget",
            Self::SystemPackage => "your package manager",
        }
    }

    pub const fn advice(self) -> &'static str {
        match self {
            Self::Winget => "Installed with winget — run winget upgrade",
            Self::SystemPackage => {
                "Installed as a system package — update it with your package manager"
            }
        }
    }
}

pub fn managed_by() -> Option<Manager> {
    let exe = std::env::current_exe().ok()?;
    if is_winget_path(&exe) {
        Some(Manager::Winget)
    } else if cfg!(target_os = "linux") && is_system_package_path(&exe) {
        Some(Manager::SystemPackage)
    } else {
        None
    }
}

pub fn setup_managed() -> bool {
    let Some(location) = platform::setup_install_location() else {
        return false;
    };
    std::env::current_exe().is_ok_and(|exe| is_setup_path(&exe, &location))
}

pub fn is_winget_path(exe: &Path) -> bool {
    lowered(exe)
        .windows(2)
        .any(|pair| pair[0] == "winget" && pair[1] == "packages")
}

pub fn is_system_package_path(exe: &Path) -> bool {
    exe.starts_with("/usr")
}

pub fn is_setup_path(exe: &Path, location: &Path) -> bool {
    exe.parent()
        .is_some_and(|dir| lowered(dir) == lowered(location))
}

fn lowered(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|part| part.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests;
