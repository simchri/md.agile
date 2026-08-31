//! Single-instance detection and port-fallback logic for launching the GUI
//! server as a one-click desktop app.
//!
//! The launcher (see `main.rs`'s `server` feature branch) uses this module to:
//! - avoid starting a second server when one is already running (via a lock
//!   file recording the running instance's PID and port), and
//! - avoid crashing when the preferred port is already taken by some other,
//!   unrelated process (by falling back to an OS-assigned free port).
//!
//! Kept target-independent (no `feature = "server"` gate) so its logic is
//! exercised by the default `cargo test` run, same as the rest of the
//! project's business logic; only its *callers* in `main.rs` are gated to
//! the native, server-enabled build.

use log::{debug, info, warn};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;

/// The running server instance recorded in the lock file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockInfo {
    pub pid: u32,
    pub port: u16,
}

/// Resolves the path of the lock file used to detect an already-running
/// instance. Prefers `$XDG_RUNTIME_DIR` (per-user, cleaned up on logout by
/// the OS); falls back to `/tmp`, disambiguated by username so multiple
/// users on the same host don't collide.
pub fn lock_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            let path = PathBuf::from(dir).join("mdagile-gui.lock");
            debug!("using lock file {} (from XDG_RUNTIME_DIR)", path.display());
            return path;
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let path = PathBuf::from("/tmp").join(format!("mdagile-gui-{user}.lock"));
    debug!(
        "using lock file {} (XDG_RUNTIME_DIR unset, falling back to /tmp)",
        path.display()
    );
    path
}

/// Serializes a [`LockInfo`] into the lock file's on-disk text format.
pub fn format_lock(info: &LockInfo) -> String {
    format!("{}:{}", info.pid, info.port)
}

/// Parses the lock file's on-disk text format (`"<pid>:<port>"`) back into a
/// [`LockInfo`]. Returns `None` for anything malformed, so a corrupted lock
/// file is treated the same as no lock file at all.
pub fn parse_lock(contents: &str) -> Option<LockInfo> {
    let (pid_str, port_str) = contents.trim().split_once(':')?;
    let pid = pid_str.parse().ok()?;
    let port = port_str.parse().ok()?;
    Some(LockInfo { pid, port })
}

/// Reads and parses the lock file at `path`, if present and well-formed.
pub fn read_lock(path: &std::path::Path) -> Option<LockInfo> {
    let contents = fs::read_to_string(path).ok()?;
    parse_lock(&contents)
}

/// Writes `info` to the lock file at `path`, creating or truncating it.
pub fn write_lock(path: &std::path::Path, info: &LockInfo) -> std::io::Result<()> {
    debug!(
        "writing lock file {} (pid {}, port {})",
        path.display(),
        info.pid,
        info.port
    );
    fs::write(path, format_lock(info))
}

/// Removes the lock file at `path`. Treats "already gone" as success, since
/// the desired end state (no lock file) is already reached.
pub fn remove_lock(path: &std::path::Path) -> std::io::Result<()> {
    debug!("removing lock file {}", path.display());
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Reports whether a process with the given PID is currently alive, via
/// `/proc/<pid>/stat` (Linux-only, matching this project's deb/rpm
/// packaging targets). A zombie process (exited but not yet reaped by its
/// parent) is treated as *not* alive — `/proc/<pid>` itself keeps existing
/// for zombies, so checking for the directory alone would wrongly report a
/// process we just terminated as still running until something reaps it.
pub fn is_pid_alive(pid: u32) -> bool {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(contents) => contents,
        Err(_) => return false,
    };
    // Format: "<pid> (<comm>) <state> ...". `comm` may itself contain
    // spaces or parens, so find the *last* ')' rather than splitting
    // naively on whitespace, to reliably locate the state field after it.
    let state = stat
        .rfind(')')
        .and_then(|idx| stat[idx + 1..].split_whitespace().next());
    !matches!(state, Some("Z"))
}

/// Reports whether the process with the given PID is *this same
/// executable* — i.e. another `mdagile-gui` instance, not an unrelated
/// process that happens to have reused a recycled PID. Compares
/// `/proc/<pid>/exe`'s target against our own `current_exe()`, which stays
/// correct regardless of what the binary or wrapper script around it is
/// named (`agilegui`, `server`, `mdagile-gui`, ...).
pub fn is_own_process(pid: u32) -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    match fs::read_link(format!("/proc/{pid}/exe")) {
        Ok(target) => target == current,
        Err(_) => false,
    }
}

/// Reports whether the lock file's recorded instance is still genuinely
/// running (alive *and* our own binary) — the combined check that
/// distinguishes a live instance from a stale lock left behind by a crashed
/// or killed process (or a foreign process that happened to reuse the PID).
pub fn is_lock_live(info: &LockInfo) -> bool {
    is_pid_alive(info.pid) && is_own_process(info.pid)
}

/// Finds a free TCP port to bind, preferring `preferred` if available, and
/// otherwise falling back to an OS-assigned ephemeral port. Only ever
/// returns a port that was, at the moment of the check, immediately
/// available to bind — there is an inherent, unavoidable race between this
/// check and the caller's own later bind, same as any "check then bind"
/// port-selection strategy.
pub fn find_free_port(preferred: u16) -> u16 {
    if TcpListener::bind(("127.0.0.1", preferred)).is_ok() {
        debug!("preferred port {preferred} is free");
        return preferred;
    }
    warn!("preferred port {preferred} is in use; falling back to an OS-assigned free port");
    let fallback = TcpListener::bind(("127.0.0.1", 0))
        .and_then(|l| l.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(preferred);
    if fallback == preferred {
        warn!("could not determine a free fallback port; retrying preferred port {preferred}");
    } else {
        info!("falling back to port {fallback}");
    }
    fallback
}

/// Sends `SIGTERM` to `pid` via a direct `kill(2)` syscall.
pub fn send_sigterm(pid: u32) -> std::io::Result<()> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Maximum length of the Linux kernel `comm` field, excluding the NUL
/// terminator (`TASK_COMM_LEN` is 16 bytes, one of which is the terminator).
const MAX_COMM_LEN: usize = 15;

/// Sets this (calling) thread's kernel-visible process name — what shows up
/// in `ps -o comm=`, `top`, `htop`, `pgrep -x`, and `/proc/<pid>/comm` — via
/// `prctl(PR_SET_NAME, ...)`.
///
/// This exists because the packaged desktop launcher's `exec ./server` (see
/// `agilegui-wrapper.sh`) leaves the process visible only as the anonymous
/// `server` (the literal name dx's fullstack bundler gives its build
/// artifact, independent of this crate's actual name) — nothing in `ps aux`
/// ties it back to mdagile-gui. Calling this once, early in `main`, fixes
/// that at the source, regardless of what the executable file or wrapper
/// around it happens to be named.
///
/// `name` is truncated to [`MAX_COMM_LEN`] bytes (on a char boundary, so it
/// never splits a multi-byte UTF-8 sequence) if longer, since the kernel
/// would otherwise reject or itself truncate it. Failure — e.g. `name`
/// containing an interior NUL byte, or the `prctl` call itself erroring — is
/// silently ignored: this is a cosmetic aid for process listings, never
/// worth failing startup over.
pub fn set_process_name(name: &str) {
    let mut end = name.len();
    if end > MAX_COMM_LEN {
        end = MAX_COMM_LEN;
        while end > 0 && !name.is_char_boundary(end) {
            end -= 1;
        }
    }
    let Ok(c_name) = std::ffi::CString::new(&name[..end]) else {
        warn!("set_process_name: {name:?} contains an interior NUL byte; leaving name unchanged");
        return;
    };
    // SAFETY: `c_name` is a valid, NUL-terminated C string kept alive for
    // the duration of this call; `PR_SET_NAME` only reads it.
    let ret = unsafe { libc::prctl(libc::PR_SET_NAME, c_name.as_ptr() as libc::c_ulong, 0, 0, 0) };
    if ret != 0 {
        warn!(
            "prctl(PR_SET_NAME, {name:?}) failed: {}",
            std::io::Error::last_os_error()
        );
    }
}

/// How long to wait, after sending `SIGTERM`, for the process to actually
/// exit before giving up and reporting it as still running.
const STOP_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const STOP_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Stops the running instance recorded in the lock file at `lock_path`:
/// verifies it's actually still live (cleaning up a stale/foreign lock
/// otherwise), sends it `SIGTERM`, waits briefly for it to actually exit,
/// and removes the lock file. Returns a human-readable status message on
/// success, or a descriptive error otherwise — both suitable for printing
/// directly from the `agilegui stop` CLI subcommand.
pub fn stop_running_instance(lock_path: &std::path::Path) -> Result<String, String> {
    let info = match read_lock(lock_path) {
        Some(info) => info,
        None => {
            info!("stop requested but no lock file at {}", lock_path.display());
            return Err("no running instance found (no lock file)".to_string());
        }
    };

    if !is_lock_live(&info) {
        warn!(
            "lock file at {} points to pid {} which is not a live mdagile-gui instance; removing stale lock",
            lock_path.display(),
            info.pid
        );
        let _ = remove_lock(lock_path);
        return Err("no running instance found (stale lock file removed)".to_string());
    }

    info!("sending SIGTERM to pid {} (port {})", info.pid, info.port);
    send_sigterm(info.pid).map_err(|e| format!("failed to signal pid {}: {e}", info.pid))?;

    let deadline = std::time::Instant::now() + STOP_WAIT_TIMEOUT;
    while std::time::Instant::now() < deadline && is_pid_alive(info.pid) {
        std::thread::sleep(STOP_POLL_INTERVAL);
    }

    let _ = remove_lock(lock_path);

    if is_pid_alive(info.pid) {
        warn!(
            "pid {} still running {}s after SIGTERM",
            info.pid,
            STOP_WAIT_TIMEOUT.as_secs()
        );
        Err(format!(
            "sent stop signal to pid {}, but it is still running after {}s",
            info.pid,
            STOP_WAIT_TIMEOUT.as_secs()
        ))
    } else {
        info!("pid {} stopped", info.pid);
        Ok(format!("stopped mdagile-gui (pid {})", info.pid))
    }
}

/// Reports whether `args` (as from `std::env::args().collect()`) invoke the
/// `agilegui stop` control subcommand, i.e. the first argument is literally
/// `"stop"`.
///
/// Note: this repurposes what was previously available as a positional
/// working-directory argument (see `get_or_init_working_dir` in
/// `server/mod.rs`) — a project directory literally named `stop` can no
/// longer be selected this way. Accepted as a rare, low-risk edge case in
/// exchange for a simple, memorable `agilegui stop` command.
pub fn is_stop_command(args: &[String]) -> bool {
    args.get(1).map(|a| a == "stop").unwrap_or(false)
}

/// If the lock file at `lock_path` points to a still-live instance of this
/// binary, returns the URL it's serving on — for the launcher to just open
/// a browser at, instead of starting a second server.
pub fn reuse_existing_instance(lock_path: &std::path::Path) -> Option<String> {
    let info = read_lock(lock_path)?;
    if is_lock_live(&info) {
        debug!(
            "found live instance at pid {} (port {}) via lock file {}",
            info.pid,
            info.port,
            lock_path.display()
        );
        Some(format!("http://127.0.0.1:{}", info.port))
    } else {
        debug!(
            "lock file {} points to pid {} which is not live; ignoring",
            lock_path.display(),
            info.pid
        );
        None
    }
}

#[cfg(test)]
#[path = "lock_tests.rs"]
mod tests;
