use std::path::Path;

pub fn winget_managed() -> bool {
    std::env::current_exe().is_ok_and(|exe| is_winget_path(&exe))
}

pub fn is_winget_path(exe: &Path) -> bool {
    let parts: Vec<String> = exe
        .components()
        .filter_map(|part| part.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .collect();
    parts
        .windows(2)
        .any(|pair| pair[0] == "winget" && pair[1] == "packages")
}

#[cfg(test)]
mod tests;
