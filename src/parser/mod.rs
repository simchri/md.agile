use std::path::PathBuf;

// ── Location ──────────────────────────────────────────────────────────────────

// Every Task and Subtask carries the file path and 1-based line number where
// its `- [...] ...` row appears. Locations are populated by `parse()` from the
// path argument and the source line index.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
}

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Todo,
    Done,
    Cancelled,
}

// ── Markers ───────────────────────────────────────────────────────────────────

// A single enum covers all marker kinds (#word and @word) so the checker can
// walk task.markers in one pass regardless of which kind it's looking for.
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Property(PropertyRef),
    Assignment(String), // the @name token; validated against mdagile.toml at check time
    Special(SpecialMarker),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub name: String,
    pub form: PropertyForm,
}

// PropertyForm carries the variant state so the checker can enforce rules
// directly: e.g. BranchPending && status == Done is always an error.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyForm {
    Full,
    Short,                    // #feat_  -- brainstorm mode; task cannot be marked Done
    BranchPending,            // #review...  -- outcome not yet chosen
    BranchResolved(String),   // #review:passed  -- branch name in the String
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpecialMarker {
    Opt,       // #OPT -- subtask does not block parent completion
    Milestone, // #MILESTONE -- file-level divider; see FileItem
    MdAgile,   // #MDAGILE -- file-level directive
}

// ── Ordering ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    None,
    Ranked(u32), // the "1." prefix; enforces execution sequence among siblings
}

// ── Subtask ───────────────────────────────────────────────────────────────────

// Quoted subtasks ("PO review") come from property definitions; unquoted ones
// are user-added. The checker needs this distinction to verify property
// conformance without re-scanning title text for quote characters.
#[derive(Debug, Clone, PartialEq)]
pub enum SubtaskKind {
    Custom,           // user-written, unquoted
    PropertyRequired, // quoted "", mandated by a Property declaration
}

// Subtask is recursive: both Task and Subtask use `children: Vec<Subtask>`
// for consistency. Task and Subtask are kept as separate types so the compiler
// prevents putting Order/SubtaskKind on a top-level Task where they have no
// meaning.
#[derive(Debug, Clone, PartialEq)]
pub struct Subtask {
    pub location: Location,
    pub indent:   usize, // leading spaces in the source line; encodes nesting
    pub status:   Status,
    pub order:    Order,
    pub kind:     SubtaskKind,
    pub title:    String,
    pub body:     Vec<String>, // lines preserve structure for LSP range calculation
    pub markers:  Vec<Marker>,
    pub children: Vec<Subtask>,
}

// ── Task ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub location: Location,
    // Leading spaces in the source line. Tasks are top-level by definition, so
    // a non-zero value means the line was indented like a subtask but had no
    // live parent (e.g. orphaned by a preceding blank line). The checker uses
    // this to flag the `wrong_indent` issue.
    pub indent:   usize,
    pub status:   Status,
    pub title:    String,
    pub body:     Vec<String>,
    pub markers:  Vec<Marker>,
    pub children: Vec<Subtask>,
}

// ── File-level items ──────────────────────────────────────────────────────────

// Milestones sit positionally *between* tasks in the file, so a flat
// Vec<FileItem> is the natural representation -- no separate index needed.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileItem {
    Task(Task),
    Milestone(Milestone),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFile {
    pub path:  PathBuf,
    pub items: Vec<FileItem>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

// Transient accumulator used while the stack is being built; converted into
// Task or Subtask when popped. Keeps a single code path for both node kinds.
struct PartialItem {
    depth:    usize,
    indent:   usize,
    location: Location,
    status:   Status,
    order:    Order,
    kind:     SubtaskKind,
    title:    String,
    body:     Vec<String>,
    markers:  Vec<Marker>,
    children: Vec<Subtask>,
}

impl PartialItem {
    fn into_task(self) -> Task {
        Task { location: self.location, indent: self.indent, status: self.status,
               title: self.title, body: self.body, markers: self.markers,
               children: self.children }
    }
    fn into_subtask(self) -> Subtask {
        Subtask { location: self.location, indent: self.indent, status: self.status,
                  order: self.order, kind: self.kind, title: self.title,
                  body: self.body, markers: self.markers, children: self.children }
    }
}

/// Parses a single `.agile.md` file's text into a sequence of [`FileItem`]s.
///
/// `path` is the source file path; it is recorded into every parsed Task and
/// Subtask via [`Location`] so callers (editor jump, LSP, error messages) can
/// trace each node back to its origin. Pass `PathBuf::new()` only if no real
/// path exists (e.g. unit tests that don't care about location).
///
/// Non-task content (headings, prose outside a task block) is silently ignored.
pub fn parse(input: &str, path: PathBuf) -> Vec<FileItem> {
    let mut items: Vec<FileItem> = Vec::new();
    let mut stack: Vec<PartialItem> = Vec::new();

    for (idx, line) in input.lines().enumerate() {
        let line_no = idx + 1;
        if line.trim().is_empty() {
            flush_stack(&mut stack, &mut items);
            continue;
        }

        if let Some(name) = parse_milestone_name(line) {
            flush_stack(&mut stack, &mut items);
            items.push(FileItem::Milestone(Milestone { name }));
            continue;
        }

        if let Some((depth, indent, status, rest)) = parse_task_line(line) {
            // Close any open siblings and their descendants before pushing the
            // new item. Popping depth >= current depth means a sibling at the
            // same level is finalized before the new one takes its place.
            while stack.last().map_or(false, |i| i.depth >= depth) {
                pop_one(&mut stack, &mut items);
            }
            let (order, rest) = parse_order_prefix(&rest);
            let (kind, rest)  = parse_subtask_kind(rest);
            let (markers, title) = parse_markers(rest);
            stack.push(PartialItem {
                depth, indent,
                location: Location { path: path.clone(), line: line_no },
                status, order, kind,
                title, body: Vec::new(), markers, children: Vec::new(),
            });
            continue;
        }

        // Any non-blank, non-task line is body text for the innermost open item.
        if let Some(top) = stack.last_mut() {
            top.body.push(line.to_string());
        }
    }

    flush_stack(&mut stack, &mut items);
    items
}

// Pops the top of the stack and attaches it to its parent (or to `items` if
// it was a top-level task). Always reduces the stack by exactly one entry.
fn pop_one(stack: &mut Vec<PartialItem>, items: &mut Vec<FileItem>) {
    let finished = stack.pop().expect("pop_one called on empty stack");
    if stack.is_empty() {
        items.push(FileItem::Task(finished.into_task()));
    } else {
        stack.last_mut().unwrap().children.push(finished.into_subtask());
    }
}

fn flush_stack(stack: &mut Vec<PartialItem>, items: &mut Vec<FileItem>) {
    while !stack.is_empty() {
        pop_one(stack, items);
    }
}

// Returns (depth, indent, status, rest-of-title) for a task line, or None.
// Indent is leading-space count; depth is indent / 2; status comes from the
// checkbox character.
fn parse_task_line(line: &str) -> Option<(usize, usize, Status, String)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let depth  = indent / 2;
    let trimmed = &line[indent..];
    let (status, rest) = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        (Status::Todo, r)
    } else if let Some(r) = trimmed.strip_prefix("- [x] ") {
        (Status::Done, r)
    } else if let Some(r) = trimmed.strip_prefix("- [-] ") {
        (Status::Cancelled, r)
    } else {
        return None;
    };
    Some((depth, indent, status, rest.trim_end().to_string()))
}

// Recognises a standalone `#MILESTONE: name` line and returns the name.
// The punctuation immediately after `#MILESTONE` is ignored per the spec.
fn parse_milestone_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("#MILESTONE")?;
    // Skip any leading non-alphanumeric chars (e.g. ": ")
    let name = rest.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '(');
    if name.is_empty() { return None; }
    Some(name.trim_end().to_string())
}

// Strips a leading order number ("1. ") and returns the rank and remaining text.
fn parse_order_prefix(title: &str) -> (Order, &str) {
    let bytes = title.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
    if i > 0 && bytes.get(i) == Some(&b'.') && bytes.get(i + 1) == Some(&b' ') {
        if let Ok(n) = title[..i].parse::<u32>() {
            return (Order::Ranked(n), title[i + 2..].trim_start());
        }
    }
    (Order::None, title)
}

// A title fully wrapped in `"..."` marks a property-required subtask; the
// quotes are stripped and the inner text is returned.
fn parse_subtask_kind(title: &str) -> (SubtaskKind, &str) {
    if title.len() >= 2 && title.starts_with('"') && title.ends_with('"') {
        (SubtaskKind::PropertyRequired, &title[1..title.len() - 1])
    } else {
        (SubtaskKind::Custom, title)
    }
}

// Splits whitespace-delimited tokens into markers (`#...`, `@...`) and plain
// title words, then re-joins the plain words.
fn parse_markers(title: &str) -> (Vec<Marker>, String) {
    let mut markers = Vec::new();
    let mut words   = Vec::new();
    for token in title.split_whitespace() {
        if let Some(after) = token.strip_prefix('#') {
            if let Some(m) = parse_hash_token(after) {
                markers.push(m);
                continue;
            }
        } else if let Some(name) = token.strip_prefix('@') {
            let name = name.trim_end_matches(|c: char| ":;,.".contains(c));
            if !name.is_empty() {
                markers.push(Marker::Assignment(name.to_string()));
                continue;
            }
        }
        words.push(token);
    }
    (markers, words.join(" "))
}

fn parse_hash_token(name: &str) -> Option<Marker> {
    if name.is_empty() { return None; }

    // Known ALL-CAPS special markers checked explicitly; avoids misidentifying
    // a user property whose name happens to be all-caps.
    match name {
        "OPT"       => return Some(Marker::Special(SpecialMarker::Opt)),
        "MILESTONE" => return Some(Marker::Special(SpecialMarker::Milestone)),
        "MDAGILE"   => return Some(Marker::Special(SpecialMarker::MdAgile)),
        _ => {}
    }

    // `#review...`  -- branch outcome not yet chosen
    if let Some(base) = name.strip_suffix("...") {
        if !base.is_empty() {
            return Some(Marker::Property(PropertyRef {
                name: base.to_string(), form: PropertyForm::BranchPending,
            }));
        }
    }

    // `#review:passed`  -- branch outcome resolved; colon + non-empty suffix
    if let Some(pos) = name.find(':') {
        let (base, branch) = (&name[..pos], &name[pos + 1..]);
        if !base.is_empty() && !branch.is_empty() {
            return Some(Marker::Property(PropertyRef {
                name: base.to_string(),
                form: PropertyForm::BranchResolved(branch.to_string()),
            }));
        }
    }

    // Plain property, possibly with trailing punctuation: `#feature:`
    let clean = name.trim_end_matches(|c: char| ":;,.".contains(c));
    if clean.is_empty() { return None; }
    Some(Marker::Property(PropertyRef {
        name: clean.to_string(), form: PropertyForm::Full,
    }))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Tests that don't care about source location share this dummy path.
    fn loc(line: usize) -> Location {
        Location { path: PathBuf::from("test.agile.md"), line }
    }

    // Wrapper around `parse` so tests don't have to repeat the dummy path.
    fn p(input: &str) -> Vec<FileItem> {
        parse(input, PathBuf::from("test.agile.md"))
    }

    // Constructs the canonical vision-doc example:
    //   - [ ] #feature: add item to basket
    //     - [ ] "PO review"          <- property-required, mandatory
    //     - [ ] #OPT extra polish    <- optional subtask
    //     - [ ] 1. first step        <- ordered
    //     - [ ] implement @markus    <- assigned
    #[test]
    fn can_construct_task_with_all_node_kinds() {
        let task = Task {
            location: loc(1),
            indent: 0,
            status: Status::Todo,
            title: "add item to basket".to_string(),
            body: vec![],
            markers: vec![Marker::Property(PropertyRef {
                name: "feature".to_string(),
                form: PropertyForm::Full,
            })],
            children: vec![
                Subtask {
                    location: loc(2),
                    indent: 2,
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::PropertyRequired,
                    title: "PO review".to_string(),
                    body: vec![],
                    markers: vec![],
                    children: vec![],
                },
                Subtask {
                    location: loc(3),
                    indent: 2,
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::Custom,
                    title: "extra polish".to_string(),
                    body: vec![],
                    markers: vec![Marker::Special(SpecialMarker::Opt)],
                    children: vec![],
                },
                Subtask {
                    location: loc(4),
                    indent: 2,
                    status: Status::Todo,
                    order: Order::Ranked(1),
                    kind: SubtaskKind::Custom,
                    title: "first step".to_string(),
                    body: vec![],
                    markers: vec![],
                    children: vec![],
                },
                Subtask {
                    location: loc(5),
                    indent: 2,
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::Custom,
                    title: "implement".to_string(),
                    body: vec![],
                    markers: vec![Marker::Assignment("markus".to_string())],
                    children: vec![],
                },
            ],
        };
        assert_eq!(task.status, Status::Todo);
        assert_eq!(task.children.len(), 4);
    }

    #[test]
    fn file_items_interleave_tasks_and_milestones() {
        let items = vec![
            FileItem::Task(Task {
                location: loc(1),
                indent: 0,
                status: Status::Done,
                title: "ship MVP".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
            }),
            FileItem::Milestone(Milestone {
                name: "Release of MVP".to_string(),
            }),
            FileItem::Task(Task {
                location: loc(5),
                indent: 0,
                status: Status::Todo,
                title: "gather feedback".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
            }),
        ];
        assert_eq!(items.len(), 3);
        assert!(matches!(items[1], FileItem::Milestone(_)));
    }

    #[test]
    fn branch_property_form_carries_resolved_branch_name() {
        let marker = Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchResolved("passed".to_string()),
        });
        if let Marker::Property(r) = marker {
            assert!(matches!(r.form, PropertyForm::BranchResolved(_)));
        }
    }

    // ── parse() tests ─────────────────────────────────────────────────────────

    fn task(items: &[FileItem], i: usize) -> &Task {
        if let FileItem::Task(t) = &items[i] { t } else { panic!("item {i} is not a Task") }
    }

    #[test]
    fn parse_todo_task() {
        let input = "\
- [ ] do the thing
";
        let items = p(input);
        assert_eq!(items.len(), 1);
        assert_eq!(task(&items, 0).status, Status::Todo);
        assert_eq!(task(&items, 0).title, "do the thing");
    }

    #[test]
    fn parse_done_task() {
        let input = "\
- [x] finished
";
        let items = p(input);
        assert_eq!(task(&items, 0).status, Status::Done);
    }

    #[test]
    fn parse_cancelled_task() {
        let input = "\
- [-] won't do
";
        let items = p(input);
        assert_eq!(task(&items, 0).status, Status::Cancelled);
        assert_eq!(task(&items, 0).title, "won't do");
    }

    #[test]
    fn parse_task_with_subtask() {
        let input = "\
- [ ] parent
  - [x] child
";
        let items = p(input);
        assert_eq!(items.len(), 1);
        let t = task(&items, 0);
        assert_eq!(t.children.len(), 1);
        assert_eq!(t.children[0].status, Status::Done);
        assert_eq!(t.children[0].title, "child");
    }

    #[test]
    fn parse_deeply_nested_subtasks() {
        let input = "\
- [ ] root
  - [ ] level1
    - [ ] level2
";
        let items = p(input);
        let root = task(&items, 0);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].title, "level1");
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].title, "level2");
    }

    #[test]
    fn parse_multiple_consecutive_tasks() {
        let input = "\
- [ ] task a
- [ ] task b
- [x] task c
";
        let items = p(input);
        assert_eq!(items.len(), 3);
        assert_eq!(task(&items, 0).title, "task a");
        assert_eq!(task(&items, 1).title, "task b");
        assert_eq!(task(&items, 2).status, Status::Done);
    }

    #[test]
    fn parse_property_marker_in_title() {
        let input = "\
- [ ] #feature: add basket
";
        let items = p(input);
        let t = task(&items, 0);
        assert_eq!(t.title, "add basket");
        assert_eq!(t.markers, vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
        })]);
    }

    #[test]
    fn parse_assignment_marker_in_title() {
        let input = "\
- [ ] implement @markus
";
        let items = p(input);
        let t = task(&items, 0);
        assert_eq!(t.title, "implement");
        assert_eq!(t.markers, vec![Marker::Assignment("markus".to_string())]);
    }

    #[test]
    fn parse_opt_subtask() {
        let input = "\
- [ ] parent
  - [ ] #OPT optional thing
";
        let items = p(input);
        let sub = &task(&items, 0).children[0];
        assert_eq!(sub.title, "optional thing");
        assert_eq!(sub.markers, vec![Marker::Special(SpecialMarker::Opt)]);
    }

    #[test]
    fn parse_ordered_subtask() {
        let input = "\
- [ ] parent
  - [ ] 1. first step
  - [ ] 2. second step
";
        let items = p(input);
        let children = &task(&items, 0).children;
        assert_eq!(children[0].order, Order::Ranked(1));
        assert_eq!(children[0].title, "first step");
        assert_eq!(children[1].order, Order::Ranked(2));
    }

    #[test]
    fn parse_property_required_subtask() {
        let input = "\
- [ ] parent
  - [ ] \"PO review\"
";
        let items = p(input);
        let sub = &task(&items, 0).children[0];
        assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
        assert_eq!(sub.title, "PO review");
    }

    #[test]
    fn parse_milestone() {
        let input = "\
#MILESTONE: Release of MVP
";
        let items = p(input);
        assert_eq!(items.len(), 1);
        assert!(matches!(&items[0], FileItem::Milestone(m) if m.name == "Release of MVP"));
    }

    #[test]
    fn parse_tasks_with_milestone_between() {
        let input = "\
- [x] ship MVP

#MILESTONE: Release of MVP

- [ ] gather feedback
";
        let items = p(input);
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], FileItem::Task(_)));
        assert!(matches!(&items[1], FileItem::Milestone(_)));
        assert!(matches!(&items[2], FileItem::Task(_)));
    }

    #[test]
    fn parse_branch_pending_marker() {
        let input = "\
- [ ] perform #review...
";
        let items = p(input);
        let markers = &task(&items, 0).markers;
        assert_eq!(markers, &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchPending,
        })]);
    }

    #[test]
    fn parse_branch_resolved_marker() {
        let input = "\
- [x] perform #review:passed
";
        let items = p(input);
        let markers = &task(&items, 0).markers;
        assert_eq!(markers, &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchResolved("passed".to_string()),
        })]);
    }

    #[test]
    fn parse_task_body_lines() {
        let input = "\
- [ ] do the thing
some details here
more info

- [ ] next task
";
        let items = p(input);
        assert_eq!(items.len(), 2);
        assert_eq!(task(&items, 0).body, vec!["some details here", "more info"]);
        assert!(task(&items, 1).body.is_empty());
    }

    #[test]
    fn parse_records_task_locations() {
        let input = "\
# heading

- [x] done
- [ ] active
  - [ ] sub
";
        let path = PathBuf::from("/abs/file.agile.md");
        let items = parse(input, path.clone());
        let t0 = task(&items, 0);
        let t1 = task(&items, 1);
        assert_eq!(t0.location, Location { path: path.clone(), line: 3 });
        assert_eq!(t1.location, Location { path: path.clone(), line: 4 });
        assert_eq!(t1.children[0].location, Location { path, line: 5 });
    }

    #[test]
    fn parse_records_source_indent() {
        let input = "\
- [ ] top
  - [ ] sub
    - [ ] deeper
";
        let items = p(input);
        let t = task(&items, 0);
        assert_eq!(t.indent, 0);
        assert_eq!(t.children[0].indent, 2);
        assert_eq!(t.children[0].children[0].indent, 4);
    }

    #[test]
    fn parse_keeps_indent_for_orphaned_indented_task() {
        // The `- [ ] orphan` line is indented like a subtask, but the
        // preceding blank line breaks the parent-child chain, so the
        // parser produces it as a top-level Task with indent > 0 — that
        // is exactly the "wrongly indented" case the checker will flag.
        let input = "\
- [ ] real top level

  - [ ] orphan indented
";
        let items = p(input);
        assert_eq!(items.len(), 2);
        let orphan = task(&items, 1);
        assert_eq!(orphan.indent, 2);
        assert_eq!(orphan.title, "orphan indented");
    }
}
