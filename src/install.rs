use std::path::Path;

use crate::platform;

pub fn winget_managed() -> bool {
    std::env::current_exe().is_ok_and(|exe| is_winget_path(&exe))
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

#[cfg(all(test, windows))]
mod tests;
