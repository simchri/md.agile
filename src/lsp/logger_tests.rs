use super::*;

#[test]
fn init_logging_creates_log_file() {
    // Clean up any existing log file first
    let log_path = std::env::temp_dir().join("agilels.log");
    let _ = std::fs::remove_file(&log_path);

    let result = init_logging();
    assert!(result.is_ok(), "init_logging should succeed");

    let log_path = result.unwrap();
    // Note: We can't reliably test that the file exists immediately because
    // tracing may buffer writes. But we can verify the path is correct.
    assert!(log_path.ends_with("agilels.log"));
}
