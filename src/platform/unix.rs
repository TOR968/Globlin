use std::fs::{File, TryLockError};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const LOCK_FILE: &str = "globlin.lock";
const PATH_MARKER: &str = "__GLOBLIN_PATH__";
const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(5);
const LOGIN_SHELL_POLL: Duration = Duration::from_millis(20);

static INSTANCE_LOCK: OnceLock<File> = OnceLock::new();

pub fn adopt_login_path(default_shell: &str) {
    let Some(login) = login_path(default_shell) else {
        return;
    };
    let current = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", merge_paths(&login, &current));
}

pub fn claim_instance_lock(dir: &Path) -> bool {
    let Ok(file) = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))
    else {
        return true;
    };
    match file.try_lock() {
        Ok(()) => {
            INSTANCE_LOCK.set(file).ok();
            true
        }
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Error(_)) => true,
    }
}

fn login_path(default_shell: &str) -> Option<String> {
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| default_shell.into());
    let script = format!("printf '\\n{PATH_MARKER}%s{PATH_MARKER}\\n' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-i", "-l", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + LOGIN_SHELL_TIMEOUT;
    while child.try_wait().ok()?.is_none() {
        if Instant::now() > deadline {
            child.kill().ok();
            child.wait().ok();
            return None;
        }
        std::thread::sleep(LOGIN_SHELL_POLL);
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    extract_path(&output)
}

fn extract_path(output: &str) -> Option<String> {
    let start = output.find(PATH_MARKER)? + PATH_MARKER.len();
    let length = output[start..].find(PATH_MARKER)?;
    let path = output[start..start + length].trim();
    (!path.is_empty()).then(|| path.to_string())
}

fn merge_paths(login: &str, current: &str) -> String {
    let mut merged: Vec<&str> = Vec::new();
    for entry in login.split(':').chain(current.split(':')) {
        if !entry.is_empty() && !merged.contains(&entry) {
            merged.push(entry);
        }
    }
    merged.join(":")
}

#[cfg(test)]
mod tests;
