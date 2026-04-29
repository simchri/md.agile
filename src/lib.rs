use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Copy, PartialEq)]
enum ItemKind {
    Todo,
    Done,
    MaybeCancel,
}

struct ItemState {
    kind: ItemKind,
    title_written: bool,
    buf: String, // accumulates text fragments for MaybeCancel items
}

pub fn list_tasks(input: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut out = String::new();
    let mut list_depth: usize = 0;
    let mut stack: Vec<ItemState> = Vec::new();

    for event in Parser::new_ext(input, opts) {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,

            Event::Start(Tag::Item) => stack.push(ItemState {
                kind: ItemKind::MaybeCancel,
                title_written: false,
                buf: String::new(),
            }),

            Event::End(TagEnd::Item) => {
                stack.pop();
            }

            Event::TaskListMarker(checked) => {
                if let Some(item) = stack.last_mut() {
                    item.kind = if checked { ItemKind::Done } else { ItemKind::Todo };
                }
            }

            Event::Text(text) => {
                if let Some(item) = stack.last_mut() {
                    if item.title_written {
                        continue;
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
                            item.buf.push_str(&text);
                            if let Some(rest) = item.buf.strip_prefix("[-] ") {
                                out.push_str(&format!("{}[-] {}\n", indent, rest));
                                item.title_written = true;
                            } else if !is_cancel_prefix(&item.buf) {
                                // Not a cancelled task — skip the whole item
                                item.title_written = true;
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    out
}

// Returns true if `s` could still be the beginning of "[-] "
fn is_cancel_prefix(s: &str) -> bool {
    "[-] ".starts_with(s)
}
