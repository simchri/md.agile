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
    Short,                  // #feat_  -- brainstorm mode; task cannot be marked Done
    BranchPending,          // #review...  -- outcome not yet chosen
    BranchResolved(String), // #review:passed  -- branch name in the String
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
    pub indent: usize, // leading spaces in the source line; encodes nesting
    pub status: Status,
    pub order: Order,
    pub kind: SubtaskKind,
    pub title: String,
    pub body: Vec<String>, // lines preserve structure for LSP range calculation
    pub markers: Vec<Marker>,
    pub children: Vec<Subtask>,
    // True if there is a space between the status box and the title.
    pub has_space_after_box: bool,
    pub box_valid: bool,
}

// ── Task ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub location: Location,
    // Leading spaces in the source line. Tasks are top-level by definition, so
    // a non-zero value means the line was indented like a subtask but had no
    // live parent. Combined with `preceded_by_blank`, this lets the checker
    // distinguish orphans (blank line before) from wrong indentation (attached
    // to previous element).
    pub indent: usize,
    // True if the immediately preceding line was blank (or the task is the very
    // first non-empty content in the file). When `indent > 0`, this disambiguates
    // orphaned subtasks (true) from wrongly-indented attached tasks (false).
    pub preceded_by_blank: bool,
    pub status: Status,
    pub title: String,
    pub body: Vec<String>,
    pub markers: Vec<Marker>,
    pub children: Vec<Subtask>,
    // True if there is a space between the status box and the title.
    // E.g., `- [ ] title` has space, `- [ ]title` does not.
    pub has_space_after_box: bool,
    pub box_valid: bool,
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
    pub path: PathBuf,
    pub items: Vec<FileItem>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

// Transient accumulator used while the stack is being built; converted into
// Task or Subtask when popped. Keeps a single code path for both node kinds.
struct PartialItem {
    depth: usize,
    indent: usize,
    preceded_by_blank: bool,
    location: Location,
    status: Status,
    order: Order,
    kind: SubtaskKind,
    title: String,
    body: Vec<String>,
    markers: Vec<Marker>,
    children: Vec<Subtask>,
    has_space_after_box: bool,
    box_valid: bool,
}

impl PartialItem {
    fn into_task(self) -> Task {
        Task {
            location: self.location,
            indent: self.indent,
            preceded_by_blank: self.preceded_by_blank,
            status: self.status,
            title: self.title,
            body: self.body,
            markers: self.markers,
            children: self.children,
            has_space_after_box: self.has_space_after_box,
            box_valid: self.box_valid,
        }
    }
    fn into_subtask(self) -> Subtask {
        Subtask {
            location: self.location,
            indent: self.indent,
            status: self.status,
            order: self.order,
            kind: self.kind,
            title: self.title,
            body: self.body,
            markers: self.markers,
            children: self.children,
            has_space_after_box: self.has_space_after_box,
            box_valid: self.box_valid,
        }
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
    // True if the previous line was blank (or we're at the start of the file).
    // Used to mark each task with whether its source was preceded by a blank line.
    let mut prev_was_blank = true;

    for (idx, line) in input.lines().enumerate() {
        let line_no = idx + 1;
        if line.trim().is_empty() {
            flush_stack(&mut stack, &mut items);
            prev_was_blank = true;
            continue;
        }

        if let Some(name) = parse_milestone_name(line) {
            flush_stack(&mut stack, &mut items);
            items.push(FileItem::Milestone(Milestone { name }));
            prev_was_blank = false;
            continue;
        }

        if let Some((depth, indent, status, rest, has_space_after_box, box_valid)) =
            parse_task_line(line)
        {
            // Close any open siblings and their descendants before pushing the
            // new item. Popping depth >= current depth means a sibling at the
            // same level is finalized before the new one takes its place.
            while stack.last().map_or(false, |i| i.depth >= depth) {
                pop_one(&mut stack, &mut items);
            }
            let (order, rest) = parse_order_prefix(&rest);
            let (kind, rest) = parse_subtask_kind(rest);
            let (markers, title) = parse_markers(rest);
            stack.push(PartialItem {
                depth,
                indent,
                preceded_by_blank: prev_was_blank,
                location: Location {
                    path: path.clone(),
                    line: line_no,
                },
                status,
                order,
                kind,
                title,
                body: Vec::new(),
                markers,
                children: Vec::new(),
                has_space_after_box,
                box_valid,
            });
            prev_was_blank = false;
            continue;
        }

        // Any non-blank, non-task line is body text for the innermost open item.
        if let Some(top) = stack.last_mut() {
            top.body.push(line.to_string());
        }
        prev_was_blank = false;
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
        stack
            .last_mut()
            .unwrap()
            .children
            .push(finished.into_subtask());
    }
}

fn flush_stack(stack: &mut Vec<PartialItem>, items: &mut Vec<FileItem>) {
    while !stack.is_empty() {
        pop_one(stack, items);
    }
}

pub trait DropNChars {
    /// Returns a string slice with the first `n` characters removed, safely handling UTF-8.
    ///
    /// If `n` is greater than the number of characters in the string, returns an empty string.
    ///
    /// # Examples
    /// ```
    /// use mdagile::parser::DropNChars; // <-- Add this line
    /// let s = "héllo";
    /// assert_eq!(s.drop_n_chars(2), "llo");
    /// ```
    fn drop_n_chars(&self, n: usize) -> &str;
}

impl DropNChars for str {
    fn drop_n_chars(&self, n: usize) -> &str {
        let idx = self
            .char_indices()
            .nth(n)
            .map(|(i, _)| i)
            .unwrap_or(self.len());
        &self[idx..]
    }
}

// Returns (depth, indent, status, rest-of-title) for a task line, or None.
// Indent is leading-space count; depth is indent / 2; status comes from the
// checkbox character.
fn parse_task_line(line: &str) -> Option<(usize, usize, Status, String, bool, bool)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let depth = indent / 2;
    let trimmed = &line[indent..];

    // Try with space first (correct format)
    let (status, rest, has_space, box_valid) = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        (Status::Todo, r, true, true)
    } else if let Some(r) = trimmed.strip_prefix("- [x] ") {
        (Status::Done, r, true, true)
    } else if let Some(r) = trimmed.strip_prefix("- [-] ") {
        (Status::Cancelled, r, true, true)
    } else if let Some(r) = trimmed.strip_prefix("- [ ]") {
        // No space after box - still parse it, but flag it
        (Status::Todo, r, false, true)
    } else if let Some(r) = trimmed.strip_prefix("- [] ") {
        // No space inside box - still parse it, but flag it
        (Status::Todo, r, true, false)
    } else if let Some(r) = trimmed.strip_prefix("- []") {
        (Status::Todo, r, false, false)
    } else if let Some(r) = trimmed.strip_prefix("- [x]") {
        (Status::Done, r, false, true)
    } else if let Some(r) = trimmed.strip_prefix("- [-]") {
        (Status::Cancelled, r, false, true)
    } else {
        // cases of boxes with wrong char ( [o], [l] .. whatever)
        let stripped_first_part = trimmed.strip_prefix("- [");
        match stripped_first_part {
            Some(r) => {
                let stripped_second_part = r.drop_n_chars(1).strip_prefix("]");
                match stripped_second_part {
                    Some(_) => {
                        // Wrong char in box, but (largely) correct format otherwise - flag it but parse as normal
                        return Some((depth, indent, Status::Todo, r.to_string(), true, false));
                    }
                    None => {}
                }
            }
            _ => {}
        }

        return None;
    };

    // SFI: has_space and box_valid could go into a "parsing issues" struct
    Some((
        depth,
        indent,
        status,
        rest.trim_end().to_string(),
        has_space,
        box_valid,
    ))
}

// Recognises a standalone `#MILESTONE: name` line and returns the name.
// The punctuation immediately after `#MILESTONE` is ignored per the spec.
fn parse_milestone_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("#MILESTONE")?;
    // Skip any leading non-alphanumeric chars (e.g. ": ")
    let name = rest.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '(');
    if name.is_empty() {
        return None;
    }
    Some(name.trim_end().to_string())
}

// Strips a leading order number ("1. ") and returns the rank and remaining text.
fn parse_order_prefix(title: &str) -> (Order, &str) {
    let bytes = title.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
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
    let mut words = Vec::new();
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
    if name.is_empty() {
        return None;
    }

    // Known ALL-CAPS special markers checked explicitly; avoids misidentifying
    // a user property whose name happens to be all-caps.
    match name {
        "OPT" => return Some(Marker::Special(SpecialMarker::Opt)),
        "MILESTONE" => return Some(Marker::Special(SpecialMarker::Milestone)),
        "MDAGILE" => return Some(Marker::Special(SpecialMarker::MdAgile)),
        _ => {}
    }

    // `#review...`  -- branch outcome not yet chosen
    if let Some(base) = name.strip_suffix("...") {
        if !base.is_empty() {
            return Some(Marker::Property(PropertyRef {
                name: base.to_string(),
                form: PropertyForm::BranchPending,
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
    if clean.is_empty() {
        return None;
    }
    Some(Marker::Property(PropertyRef {
        name: clean.to_string(),
        form: PropertyForm::Full,
    }))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
