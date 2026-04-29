use ignore::WalkBuilder;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};

pub fn format_file_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            format!("{name}  {}\n", p.display())
        })
        .collect()
}

pub fn read_task_files(root: &Path) -> String {
    find_task_files(root)
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn find_task_files(root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = WalkBuilder::new(root)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(".agile.md")
        })
        .map(|e| e.into_path())
        .collect();

    paths.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()));
    paths
}

#[derive(Clone, Copy, PartialEq)]
enum ItemKind {
    Todo,
    Done,
    MaybeCancel,
}

struct ItemState {
    kind: ItemKind,
    title_written: bool,
    buf: String,
}

impl ItemState {
    fn new() -> Self {
        Self { kind: ItemKind::MaybeCancel, title_written: false, buf: String::new() }
    }
}

// Returns true if `s` could still be the start of "[-] "
fn is_cancel_prefix(s: &str) -> bool {
    "[-] ".starts_with(s)
}

fn write_task_text(out: &mut String, item: &mut ItemState, text: &str, list_depth: usize) {
    if item.title_written {
        return;
    }
    let indent = "  ".repeat(list_depth - 1);
    match item.kind {
        ItemKind::Todo => {
            out.push_str(&format!("{}[ ] {}\n", indent, text));
            item.title_written = true;
        }
        ItemKind::Done => {
            out.push_str(&format!("{}[x] {}\n", indent, text));
            item.title_written = true;
        }
        ItemKind::MaybeCancel => {
            item.buf.push_str(text);
            if let Some(rest) = item.buf.strip_prefix("[-] ") {
                out.push_str(&format!("{}[-] {}\n", indent, rest));
                item.title_written = true;
            } else if !is_cancel_prefix(&item.buf) {
                item.title_written = true;
            }
        }
    }
}

fn make_parser(input: &str) -> Parser<'_> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(input, opts)
}

pub fn list_tasks(input: &str) -> String {
    let mut out = String::new();
    let mut list_depth: usize = 0;
    let mut stack: Vec<ItemState> = Vec::new();

    for event in make_parser(input) {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::Item) => stack.push(ItemState::new()),
            Event::End(TagEnd::Item) => { stack.pop(); }
            Event::TaskListMarker(checked) => {
                if let Some(item) = stack.last_mut() {
                    item.kind = if checked { ItemKind::Done } else { ItemKind::Todo };
                }
            }
            Event::Text(text) => {
                if let Some(item) = stack.last_mut() {
                    write_task_text(&mut out, item, &text, list_depth);
                }
            }
            _ => {}
        }
    }

    out
}

pub fn next_task(input: &str) -> String {
    let mut out = String::new();
    let mut list_depth: usize = 0;
    let mut stack: Vec<ItemState> = Vec::new();
    let mut capturing = false;

    for event in make_parser(input) {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::Item) => stack.push(ItemState::new()),
            Event::End(TagEnd::Item) => {
                let at_top = list_depth == 1 && stack.len() == 1;
                stack.pop();
                if capturing && at_top {
                    return out;
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = stack.last_mut() {
                    item.kind = if checked { ItemKind::Done } else { ItemKind::Todo };
                }
                if !capturing && !checked && list_depth == 1 && stack.len() == 1 {
                    capturing = true;
                }
            }
            Event::Text(text) => {
                if capturing {
                    if let Some(item) = stack.last_mut() {
                        write_task_text(&mut out, item, &text, list_depth);
                    }
                }
            }
            _ => {}
        }
    }

    out
}
