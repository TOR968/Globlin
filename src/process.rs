use std::io::{self, Read};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

pub const LISTING_LIMIT: Duration = Duration::from_mins(2);
pub const CHANGE_LIMIT: Duration = Duration::from_mins(30);
const POLL: Duration = Duration::from_millis(50);
const DRAIN_GRACE: Duration = Duration::from_secs(5);

pub fn output_within(command: &mut Command, limit: Duration) -> io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            stop(&mut child);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{} did not finish within {} seconds and was stopped",
                    program_name(command),
                    limit.as_secs()
                ),
            ));
        }
        std::thread::sleep(POLL);
    };
    Ok(Output {
        status,
        stdout: collect(&stdout),
        stderr: collect(&stderr),
    })
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).ok();
            sender.send(bytes).ok();
        });
    }
    receiver
}

fn collect(receiver: &Receiver<Vec<u8>>) -> Vec<u8> {
    receiver.recv_timeout(DRAIN_GRACE).unwrap_or_default()
}

fn program_name(command: &Command) -> String {
    std::path::Path::new(command.get_program())
        .file_name()
        .map_or_else(
            || command.get_program().to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
}

#[cfg(windows)]
fn stop(child: &mut Child) {
    let mut taskkill = crate::source::hidden_command(std::path::Path::new("taskkill"));
    taskkill
        .args(["/F", "/T", "/PID", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();
    child.kill().ok();
    child.wait().ok();
}

#[cfg(not(windows))]
fn stop(child: &mut Child) {
    child.kill().ok();
    child.wait().ok();
}

#[cfg(test)]
mod tests;
