//! Runs a form validation script (see README.vision.md "Forms & Fields" and
//! `doc/usage/form_validation_scripts.md` for the JSON interface).
//!
//! Hardening: the script is executed directly (no shell, no arguments) and
//! only if its current content is approved in the [`TrustStore`]. Its JSON
//! input goes over stdin, the environment is cleared, the working directory
//! is the project root, and a timeout plus an output-size cap bound what a
//! hung or runaway script can do. This does NOT block the script's own file
//! or network access -- it is not an OS-level sandbox.

use crate::trust::{ScriptPathError, TrustStore, resolve_script, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// JSON sent to the script on stdin: only the form's own fields, never other
/// properties/fields of the task.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScriptInput {
    pub form: String,
    pub task: TaskInfo,
    /// Field label -> value.
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskInfo {
    pub title: String,
    /// 1-based source line of the task.
    pub line: usize,
}

/// JSON the script prints to stdout. A script that ran properly always exits
/// 0, whether or not the form is valid.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScriptOutput {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<ScriptError>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScriptError {
    /// Label of the offending field, if the error is about one field.
    #[serde(default)]
    pub field: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            timeout: Duration::from_secs(10),
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// The script could not be run properly -- distinct from it reporting the
/// form as invalid.
#[derive(Debug)]
pub enum RunError {
    Path(ScriptPathError),
    Untrusted,
    Spawn(std::io::Error),
    NonZeroExit { code: Option<i32>, stderr: String },
    Timeout,
    OutputTooLarge,
    InvalidOutput(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Path(e) => write!(f, "invalid script path: {e}"),
            RunError::Untrusted => f.write_str("script is not trusted"),
            RunError::Spawn(e) => write!(f, "could not start script: {e}"),
            RunError::NonZeroExit { code, stderr } => {
                match code {
                    Some(code) => write!(f, "script exited with status {code}")?,
                    None => f.write_str("script was terminated by a signal")?,
                }
                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
            RunError::Timeout => f.write_str("script timed out"),
            RunError::OutputTooLarge => f.write_str("script output exceeded the size limit"),
            RunError::InvalidOutput(e) => write!(f, "script printed invalid JSON output: {e}"),
        }
    }
}

type Captured = std::io::Result<Vec<u8>>;

/// Reads at most `max + 1` bytes, so an oversized stream is detectable
/// without buffering it entirely.
fn capture(mut stream: impl Read + Send + 'static, max: usize) -> mpsc::Receiver<Captured> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let result = (&mut stream)
            .take(max as u64 + 1)
            .read_to_end(&mut buf)
            .map(|_| buf);
        let _ = tx.send(result);
    });
    rx
}

pub fn run_script(
    root: &Path,
    script: &str,
    input: &ScriptInput,
    store: &TrustStore,
    limits: &Limits,
) -> Result<ScriptOutput, RunError> {
    let path = resolve_script(root, script).map_err(RunError::Path)?;
    let content = std::fs::read(&path).map_err(RunError::Spawn)?;
    if !store.is_trusted(script, &sha256_hex(&content)) {
        return Err(RunError::Untrusted);
    }

    let mut child = Command::new(&path)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunError::Spawn)?;

    let payload = serde_json::to_vec(input).expect("script input is always serializable");
    let mut stdin = child.stdin.take().expect("stdin was piped");
    // A script that never reads stdin must not block us: write off-thread and
    // ignore a broken pipe.
    std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
    });
    let stdout_rx = capture(
        child.stdout.take().expect("stdout was piped"),
        limits.max_output_bytes,
    );
    let stderr_rx = capture(child.stderr.take().expect("stderr was piped"), 64 * 1024);

    let deadline = Instant::now() + limits.timeout;
    let mut stdout: Option<Vec<u8>> = None;
    let status = loop {
        if let Ok(result) = stdout_rx.try_recv() {
            let bytes = result.map_err(RunError::Spawn)?;
            if bytes.len() > limits.max_output_bytes {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::OutputTooLarge);
            }
            stdout = Some(bytes);
        }
        if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(5));
    };

    let remaining = deadline.saturating_duration_since(Instant::now());
    let stdout = match stdout {
        Some(bytes) => bytes,
        None => stdout_rx
            .recv_timeout(remaining)
            .map_err(|_| RunError::Timeout)?
            .map_err(RunError::Spawn)?,
    };
    let stderr = stderr_rx
        .recv_timeout(Duration::from_millis(500))
        .ok()
        .and_then(Result::ok)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();

    if !status.success() {
        return Err(RunError::NonZeroExit {
            code: status.code(),
            stderr,
        });
    }
    if stdout.len() > limits.max_output_bytes {
        return Err(RunError::OutputTooLarge);
    }
    serde_json::from_slice(&stdout).map_err(|e| RunError::InvalidOutput(e.to_string()))
}

#[cfg(test)]
mod tests;
