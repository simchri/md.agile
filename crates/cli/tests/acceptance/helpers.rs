use std::path::Path;
use std::process::{Command, Output};

pub fn run_check(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .arg("check")
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile check`")
}

pub fn run_task_list(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .args(["task", "list"])
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile task list`")
}

/// Run `agile` with arbitrary args in `cwd`, inheriting the current environment.
pub fn run_agile(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agile"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn `agile`")
}

/// Run `agile` with arbitrary args, a completely cleared environment,
/// and `extra_env` key-value pairs added back in. Useful for controlling
/// `$VISUAL`/`$EDITOR` in default-subcommand tests.
pub fn run_agile_clean_env(cwd: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_agile"));
    cmd.args(args).current_dir(cwd).env_clear();
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output().expect("failed to spawn `agile`")
}
