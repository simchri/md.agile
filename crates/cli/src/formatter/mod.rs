//! ESLint-style error formatting.
//!
//! Formats [`crate::rules::Issue`] values into readable, editor-parseable output
//! with source context and visual indicators, following the style of ESLint,
//! Clippy, and other standard linters.

use crate::rules::Issue;
use std::fs;

// ANSI color codes for terminal output
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";

/// Formats a single Issue into ESLint-style output with source context.
///
/// Returns a multi-line string containing:
/// - Error header with file, line, and column
/// - Source context line(s)
/// - Visual indicator pointing to the problem
/// - Help text if available
///
/// The output is designed to be both human-readable and parseable by editor
/// integrations (LSP, linters, etc.).
pub fn format_issue(issue: &Issue) -> String {
    let mut output = String::new();

    // Header line: error[CODE]: message [(fix avail.)]
    let fix_hint = if issue.code.has_quickfix() {
        " (fix avail.)"
    } else {
        ""
    };
    output.push_str(&format!(
        "{}{BOLD}error[{}]{RESET}: {}{}\n",
        RED, issue.code, issue.message, fix_hint
    ));

    // File:line:column location
    output.push_str(&format!(
        " {}-->{}  {}:{}:{}\n",
        CYAN,
        RESET,
        issue.location.path.display(),
        issue.location.line,
        issue.column
    ));

    // Try to read and display source context
    if let Ok(content) = fs::read_to_string(&issue.location.path) {
        let lines: Vec<&str> = content.lines().collect();

        // Safety check: ensure line number is valid (1-based indexing)
        let line_idx = issue.location.line.saturating_sub(1);
        if line_idx < lines.len() {
            let error_line = lines[line_idx];

            // Display the source line with line number
            output.push_str(&format!(
                "  {YELLOW}|{RESET}\n  {YELLOW}| {RESET}{}\n",
                error_line
            ));

            // Create and display the error pointer
            let pointer = create_pointer(error_line, issue.column);
            output.push_str(&format!("  {YELLOW}| {RED}{BOLD}{}{RESET}\n", pointer));
        }
    }

    // Add help text if available
    if let Some(help) = &issue.help {
        output.push_str(&format!(
            "  {YELLOW}={RESET}\n  {YELLOW}help:{RESET} {}\n",
            help
        ));
    }

    output.push('\n');
    output
}

/// Creates a visual pointer string at the specified column.
///
/// Returns a string with spaces up to `column - 1`, then a `^` character
/// to indicate the problem location. Column is 1-based.
fn create_pointer(line: &str, column: usize) -> String {
    let target_col = column.saturating_sub(1); // Convert to 0-based
    let mut pointer = String::new();

    // Count visible characters (handling multi-byte UTF-8)
    let mut visible_pos = 0;
    let mut found = false;

    for ch in line.chars() {
        if visible_pos >= target_col {
            for _ in visible_pos..target_col {
                pointer.push(' ');
            }
            pointer.push('^');
            found = true;
            break;
        }

        // Add a space for each visible character
        pointer.push(' ');

        // For display purposes, tabs count as moving to next tab stop
        if ch == '\t' {
            visible_pos += 4;
        } else {
            visible_pos += 1;
        }
    }

    // If we didn't find the column in the line, point to the end
    if !found {
        // Pad to reach the target column
        while pointer.len() < target_col {
            pointer.push(' ');
        }
        pointer.push('^');
    }

    pointer
}

#[cfg(test)]
mod tests;
