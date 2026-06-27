use std::path::Path;
use std::process::{Command, Output};

pub fn run_check(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("check")
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile check`")
}

pub fn run_list(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("list")
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile list`")
}
