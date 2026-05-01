//! Default action: open the next active task in $VISUAL / $EDITOR.
//!
//! Invoked when the user runs `agile` with no subcommand.

use crate::cli::common::parse_file;
use crate::parser::{FileItem, Status};
use std::path::{Path, PathBuf};

/// Default-action entry point. Opens the editor at the next active task, or
/// prints a message to stderr if every task is done.
pub fn run(root: &Path) {
    match find_file_with_next_task(root) {
        Some(path) => {
            let line = find_next_task_line(&path).unwrap_or(1);
            open_editor(&path, line);
        }
        None => eprintln!("agile: no active tasks found"),
    }
}

/// Returns the path of the highest-priority task file that contains at least one active task.
///
/// Files are evaluated in priority order (alphabetical by relative path). Returns `None` if no
/// file has any incomplete `[ ]` top-level tasks, or if no task files exist under `root`.
pub fn find_file_with_next_task(root: &Path) -> Option<PathBuf> {
    crate::cli::common::find_task_files(root)
        .into_iter()
        .find(|p| file_has_active_task(p))
}

/// Returns the 1-based line number of the first incomplete top-level task in `path`.
///
/// Reads and parses `path`, then returns the [`crate::parser::Location::line`] of the
/// first top-level Task whose status is [`Status::Todo`]. Returns `None` if the
/// file cannot be read or contains no incomplete top-level tasks.
pub fn find_next_task_line(path: &Path) -> Option<usize> {
    parse_file(path).into_iter().find_map(|item| match item {
        FileItem::Task(t) if t.status == Status::Todo => Some(t.location.line),
        _ => None,
    })
}

fn file_has_active_task(path: &Path) -> bool {
    parse_file(path).iter().any(|item| {
        matches!(item, FileItem::Task(t) if t.status == Status::Todo)
    })
}

fn open_editor(path: &Path, line: usize) {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            eprintln!("agile: neither $VISUAL nor $EDITOR is set");
            std::process::exit(1);
        });
    let args = editor_open_args(&editor, path, line);
    let status = std::process::Command::new(&editor).args(&args).status();
    if let Err(e) = status {
        eprintln!("agile: failed to launch editor '{editor}': {e}");
        std::process::exit(1);
    }
}

/// Builds the argv for invoking `editor` to open `path` at `line`.
///
/// Recognised editors get a "+LINE" or "--goto FILE:LINE" flag; unknown
/// editors just get the path with no line number.
pub fn editor_open_args(editor: &str, path: &Path, line: usize) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;
    let bin_name = Path::new(editor)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    match bin_name.as_ref() {
        "vim" | "vi" | "nvim" | "nano" | "emacs" => {
            vec![OsString::from(format!("+{line}")), path.into()]
        }
        "code" => vec![
            OsString::from("--goto"),
            OsString::from(format!("{}:{line}", path.display())),
        ],
        _ => vec![path.into()],
    }
}

#[cfg(test)]
mod tests;
