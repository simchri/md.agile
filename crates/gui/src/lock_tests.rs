use super::*;

#[test]
fn format_lock_round_trips_through_parse_lock() {
    let info = LockInfo {
        pid: 12345,
        port: 8080,
    };
    let text = format_lock(&info);
    assert_eq!(parse_lock(&text), Some(info));
}

#[test]
fn parse_lock_rejects_malformed_contents() {
    assert_eq!(parse_lock(""), None);
    assert_eq!(parse_lock("not-a-lock-file"), None);
    assert_eq!(parse_lock("123"), None);
    assert_eq!(parse_lock("abc:8080"), None);
    assert_eq!(parse_lock("123:abc"), None);
}

#[test]
fn parse_lock_tolerates_trailing_whitespace() {
    assert_eq!(
        parse_lock("42:9090\n"),
        Some(LockInfo {
            pid: 42,
            port: 9090
        })
    );
}

#[test]
fn write_lock_then_read_lock_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    let info = LockInfo {
        pid: 777,
        port: 3000,
    };

    write_lock(&path, &info).unwrap();

    assert_eq!(read_lock(&path), Some(info));
}

#[test]
fn read_lock_returns_none_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.lock");

    assert_eq!(read_lock(&path), None);
}

#[test]
fn remove_lock_deletes_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    write_lock(&path, &LockInfo { pid: 1, port: 8080 }).unwrap();

    remove_lock(&path).unwrap();

    assert!(!path.exists());
}

#[test]
fn remove_lock_succeeds_when_file_already_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.lock");

    // Removing an already-absent lock file is the desired end state, not an
    // error — a stale/foreign lock check that races a concurrent cleanup
    // shouldn't itself fail because of that race.
    assert!(remove_lock(&path).is_ok());
}

#[test]
fn is_pid_alive_is_true_for_our_own_process() {
    assert!(is_pid_alive(std::process::id()));
}

#[test]
fn is_pid_alive_is_false_for_an_implausibly_high_pid() {
    // PIDs this large can't exist on Linux (max_pid is far lower), so this
    // is a safe "definitely not alive" fixture without depending on any
    // specific process actually being absent at test time.
    assert!(!is_pid_alive(u32::MAX));
}

#[test]
fn is_pid_alive_is_false_for_a_zombie_process() {
    // A terminated-but-not-yet-reaped child still has a `/proc/<pid>` entry
    // — this regression-tests that `is_pid_alive` looks at process *state*
    // (and treats zombies as dead) rather than just directory existence.
    let mut child = std::process::Command::new("true")
        .spawn()
        .expect("failed to spawn `true`");
    let pid = child.id();

    // Wait for it to actually finish running (it's a near-instant command)
    // without reaping it yet, so it becomes a zombie.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            let state = contents
                .rfind(')')
                .and_then(|i| contents[i + 1..].split_whitespace().next());
            if state == Some("Z") {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(!is_pid_alive(pid));

    let _ = child.wait();
}

#[test]
fn is_own_process_is_true_for_our_own_pid() {
    assert!(is_own_process(std::process::id()));
}

#[test]
fn is_own_process_is_false_for_an_implausibly_high_pid() {
    assert!(!is_own_process(u32::MAX));
}

#[test]
fn is_lock_live_is_true_when_pid_is_alive_and_our_own_process() {
    let info = LockInfo {
        pid: std::process::id(),
        port: 8080,
    };
    assert!(is_lock_live(&info));
}

#[test]
fn is_lock_live_is_false_for_a_dead_pid() {
    let info = LockInfo {
        pid: u32::MAX,
        port: 8080,
    };
    assert!(!is_lock_live(&info));
}

#[test]
fn find_free_port_falls_back_when_preferred_port_is_taken() {
    let busy = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let busy_port = busy.local_addr().unwrap().port();

    let fallback = find_free_port(busy_port);

    assert_ne!(fallback, busy_port);
    assert!(TcpListener::bind(("127.0.0.1", fallback)).is_ok());
}

#[test]
fn is_stop_command_is_true_when_first_arg_is_stop() {
    let args = vec!["agilegui".to_string(), "stop".to_string()];
    assert!(is_stop_command(&args));
}

#[test]
fn is_stop_command_is_false_when_first_arg_is_a_workdir_path() {
    let args = vec!["agilegui".to_string(), "/some/project".to_string()];
    assert!(!is_stop_command(&args));
}

#[test]
fn is_stop_command_is_false_with_no_args() {
    let args = vec!["agilegui".to_string()];
    assert!(!is_stop_command(&args));
}

#[test]
fn reuse_existing_instance_returns_url_for_a_live_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    write_lock(
        &path,
        &LockInfo {
            pid: std::process::id(),
            port: 4321,
        },
    )
    .unwrap();

    assert_eq!(
        reuse_existing_instance(&path),
        Some("http://127.0.0.1:4321".to_string())
    );
}

#[test]
fn reuse_existing_instance_returns_none_for_a_stale_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    write_lock(
        &path,
        &LockInfo {
            pid: u32::MAX,
            port: 4321,
        },
    )
    .unwrap();

    assert_eq!(reuse_existing_instance(&path), None);
}

#[test]
fn reuse_existing_instance_returns_none_when_no_lock_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.lock");

    assert_eq!(reuse_existing_instance(&path), None);
}

/// Not a real test — a deliberately slow marker test used only as a
/// stand-in live child process for
/// `stop_running_instance_terminates_a_live_instance_of_this_binary` below.
/// Running the *current test binary itself* filtered down to just this one
/// test (via `--exact`) gives that test a genuinely live process whose
/// `/proc/<pid>/exe` really does match `current_exe()` — i.e. a real
/// "another instance of this same binary is running" scenario — without
/// recursively re-running the whole suite.
#[test]
fn __sleep_for_kill_test() {
    std::thread::sleep(std::time::Duration::from_secs(10));
}

#[test]
fn stop_running_instance_reports_error_when_no_lock_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.lock");

    assert!(stop_running_instance(&path).is_err());
}

#[test]
fn stop_running_instance_cleans_up_a_stale_lock_and_reports_no_running_instance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    // u32::MAX can never be a real PID on Linux, so this is a safe "dead"
    // fixture regardless of what's actually running on the test host.
    write_lock(
        &path,
        &LockInfo {
            pid: u32::MAX,
            port: 8080,
        },
    )
    .unwrap();

    let result = stop_running_instance(&path);

    assert!(result.is_err());
    assert!(read_lock(&path).is_none(), "stale lock should be removed");
}

#[test]
fn stop_running_instance_terminates_a_live_instance_of_this_binary() {
    let current = std::env::current_exe().unwrap();
    let mut child = std::process::Command::new(&current)
        .args([
            "lock::tests::__sleep_for_kill_test",
            "--exact",
            "--test-threads=1",
        ])
        .spawn()
        .expect("failed to spawn child test process");

    // Give the child a moment to actually start running, so `/proc/<pid>/exe`
    // resolves before we check it.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock");
    write_lock(
        &path,
        &LockInfo {
            pid: child.id(),
            port: 8080,
        },
    )
    .unwrap();

    let result = stop_running_instance(&path);

    assert!(result.is_ok(), "expected success, got {result:?}");
    assert!(
        read_lock(&path).is_none(),
        "lock file should be removed after a successful stop"
    );

    // Reap the child so the test doesn't leave a zombie process behind;
    // it should already be gone (or gone imminently) thanks to the SIGTERM
    // `stop_running_instance` just sent.
    let _ = child.wait();
}

/// Reads back the calling thread's own kernel `comm` name via
/// `prctl(PR_GET_NAME)` — the read-side counterpart of [`set_process_name`],
/// used here instead of `/proc/self/comm` because `comm` is per-thread and
/// `/proc/self` resolves to the process's main thread, which need not be
/// the thread a test actually runs on under `cargo test`'s thread pool.
/// Round-tripping through `prctl` directly keeps the set/get pair on the
/// same thread, avoiding that mismatch.
fn get_process_name() -> String {
    let mut buf = [0u8; MAX_COMM_LEN + 1];
    let ret = unsafe {
        libc::prctl(
            libc::PR_GET_NAME,
            buf.as_mut_ptr() as libc::c_ulong,
            0,
            0,
            0,
        )
    };
    assert_eq!(ret, 0, "prctl(PR_GET_NAME) failed");
    std::ffi::CStr::from_bytes_until_nul(&buf)
        .expect("comm buffer should be NUL-terminated")
        .to_str()
        .expect("comm should be valid UTF-8")
        .to_string()
}

#[test]
fn set_process_name_sets_the_kernel_comm_field() {
    let name = "test-comm-xyz"; // 13 bytes — well under the 15-byte cap
    set_process_name(name);

    assert_eq!(get_process_name(), name);
}

#[test]
fn set_process_name_truncates_names_longer_than_task_comm_len() {
    let long = "this-name-is-definitely-too-long-for-comm";
    set_process_name(long);

    let read_back = get_process_name();
    assert_eq!(read_back, &long[..MAX_COMM_LEN]);
    assert_eq!(read_back.len(), MAX_COMM_LEN);
}

#[test]
fn set_process_name_truncates_on_a_char_boundary() {
    // 16 bytes total, with a 3-byte UTF-8 character ('é' is 2 bytes; use a
    // multi-byte char straddling the 15-byte cut point) so a naive byte-index
    // truncation at exactly `MAX_COMM_LEN` would split it and panic/corrupt.
    let name = "aaaaaaaaaaaaaaé"; // 14 ASCII 'a's + 1 two-byte char = 16 bytes
    assert_eq!(name.len(), 16);

    // Should not panic, and should truncate to the last full char boundary
    // at or before MAX_COMM_LEN.
    set_process_name(name);

    let read_back = get_process_name();
    assert!(read_back.len() <= MAX_COMM_LEN);
    assert!(name.starts_with(&read_back));
}
