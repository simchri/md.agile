//! Default action: open the next active task in $VISUAL / $EDITOR.
//!
//! Invoked when the user runs `agile` with no subcommand.

use crate::cli::common::{find_task_files, parse_file};
use crate::parser::{FileItem, Status};
use std::path::{Path, PathBuf};

/// Default-action entry point. Opens the editor at the next active task, or
/// prints a message to stderr if every task is done.
pub fn run(root: &Path) {
    match find_next_task(root) {
        Some((path, line)) => open_editor(&path, line),
        None => eprintln!("agile: no active tasks found"),
    }
}

/// Returns the `(path, line)` of the first incomplete top-level task across
/// all task files under `root`, in priority order.
///
/// Files are walked in priority order (alphabetical by relative path). For
/// each file, the parser runs once: if the file contains at least one todo
/// (`[ ]`) top-level task, this returns `(file, line_of_first_todo)`. Done
/// (`[x]`) and cancelled (`[-]`) top-level tasks are skipped, as are subtasks.
/// Returns `None` if no file under `root` has any active top-level task.
pub fn find_next_task(root: &Path) -> Option<(PathBuf, usize)> {
    find_task_files(root)
        .into_iter()
        .find_map(|path| first_active_task_line(&path).map(|line| (path, line)))
}

fn first_active_task_line(path: &Path) -> Option<usize> {
    parse_file(path).into_iter().find_map(|item| match item {
        FileItem::Task(t) if t.status == Status::Todo => Some(t.location.line),
        _ => None,
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
