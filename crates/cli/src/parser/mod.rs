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
pub struct AssignmentRef {
    pub name: String,
    /// 1-based column of the `@` within the task **title text** (the portion
    /// after `"- [ ] "`). The full source-line column is
    /// `indent + TASK_LINE_PREFIX_LEN + column`.
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Property(PropertyRef),
    Assignment(AssignmentRef),
    Special(SpecialMarker),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub name: String,
    pub form: PropertyForm,
    /// 1-based column of the `#` within the task **title text** (the portion
    /// after `"- [ ] "`). The full source-line column is
    /// `indent + TASK_LINE_PREFIX_LEN + column`.
    pub column: usize,
}

/// Length of the `"- [ ] "` prefix on every task/subtask line.
/// Used by rules to convert a title-relative column to a source-line column.
pub const TASK_LINE_PREFIX_LEN: usize = 6;

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
pub enum SpecialMarkerKind {
    Opt,       // #OPT -- subtask does not block parent completion
    Milestone, // #MILESTONE -- file-level divider; see FileItem
    MdAgile,   // #MDAGILE -- file-level directive
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialMarker {
    pub column: usize,
    pub kind: SpecialMarkerKind,
}

impl SpecialMarker {
    /// The ALL-CAPS keyword that represents this marker in source (e.g. `"OPT"`).
    pub fn as_str(&self) -> &'static str {
        match self.kind {
            SpecialMarkerKind::Opt => "OPT",
            SpecialMarkerKind::Milestone => "MILESTONE",
            SpecialMarkerKind::MdAgile => "MDAGILE",
        }
    }

    /// Construct a `SpecialMarker` from its ALL-CAPS keyword, or `None` if the
    /// name is not a known special marker.
    pub fn from_name(name: &str, column: usize) -> Option<Self> {
        let kind = match name {
            "OPT" => SpecialMarkerKind::Opt,
            "MILESTONE" => SpecialMarkerKind::Milestone,
            "MDAGILE" => SpecialMarkerKind::MdAgile,
            _ => return None,
        };
        Some(SpecialMarker { column, kind })
    }
}

// ── Marker boundary rules (shared with LSP) ───────────────────────────────────

/// Characters that end a marker name (`#foo` or `@foo`) when scanned forward.
///
/// Used by the parser when scanning source text, and re-exported for the LSP
/// `goto_definition` module so both operate on exactly the same rule set.
pub(crate) fn is_marker_boundary(c: char) -> bool {
    c.is_ascii_whitespace()
        || c == '('
        || c == ')'
        || c == '['
        || c == ']'
        || c == '{'
        || c == '}'
        || c == '\''
        || c == '"'
}

/// Returns `true` if `c` is a quoting character that, when immediately
/// preceding a sigil, causes the sigil to be treated as prose rather than a
/// marker start.
pub(crate) fn is_marker_quote(c: char) -> bool {
    c == '\'' || c == '"'
}

/// Trailing punctuation characters stripped from the end of a raw marker name.
///
/// Applies to both `#property` (in `parse_hash_token`) and `@assignment`
/// (in `parse_markers`) after the name has been bounded by [`is_marker_boundary`].
pub(crate) const MARKER_TRAILING_PUNCT: &str = ":;,.";

// ── Parsing issues ────────────────────────────────────────────────────────────

/// Problems detected while parsing a single task line.
///
/// Stored on [`Task`] and [`Subtask`] so lint rules can check them without
/// re-parsing. Keeping issues in a `Vec` means adding a new variant never
/// requires a new boolean field on every node type.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsingIssue {
    /// No space between the status box and the title: `- [ ]title`
    MissingSpaceAfterBox,
    /// Box contains an invalid character or is empty: `- [o] …`, `- [] …`
    InvalidBox,
    /// Box uses uppercase X instead of lowercase: `- [X] …`
    UppercaseX,
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
    pub parsing_issues: Vec<ParsingIssue>,
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
    pub parsing_issues: Vec<ParsingIssue>,
}

// ── File-level items ──────────────────────────────────────────────────────────

// Milestones sit positionally *between* tasks in the file, so a flat
// Vec<FileItem> is the natural representation -- no separate index needed.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    pub name: String,
    pub line: usize, // 1-based source line of the #MILESTONE: header
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
    parsing_issues: Vec<ParsingIssue>,
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
            parsing_issues: self.parsing_issues,
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
            parsing_issues: self.parsing_issues,
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
            items.push(FileItem::Milestone(Milestone {
                name,
                line: line_no,
            }));
            prev_was_blank = false;
            continue;
        }

        if let Some((depth, indent, status, rest, parsing_issues)) = parse_task_line(line) {
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
                parsing_issues,
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
    /// use mdagile::parser::DropNChars;
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

// Returns (depth, indent, status, rest-of-title, parsing_issues) for a task
// line, or None. Indent is leading-space count; depth is indent / 2; status
// comes from the checkbox character.
fn parse_task_line(line: &str) -> Option<(usize, usize, Status, String, Vec<ParsingIssue>)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let depth = indent / 2;
    let trimmed = &line[indent..];

    let (status, rest, issues) = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        (Status::Todo, r, vec![])
    } else if let Some(r) = trimmed.strip_prefix("- [x] ") {
        (Status::Done, r, vec![])
    } else if let Some(r) = trimmed.strip_prefix("- [-] ") {
        (Status::Cancelled, r, vec![])
    } else if let Some(r) = trimmed.strip_prefix("- [X] ") {
        (Status::Done, r, vec![ParsingIssue::UppercaseX])
    } else if let Some(r) = trimmed.strip_prefix("- [ ]") {
        (Status::Todo, r, vec![ParsingIssue::MissingSpaceAfterBox])
    } else if let Some(r) = trimmed.strip_prefix("- [] ") {
        (Status::Todo, r, vec![ParsingIssue::InvalidBox])
    } else if let Some(r) = trimmed.strip_prefix("- []") {
        (
            Status::Todo,
            r,
            vec![ParsingIssue::InvalidBox, ParsingIssue::MissingSpaceAfterBox],
        )
    } else if let Some(r) = trimmed.strip_prefix("- [x]") {
        (Status::Done, r, vec![ParsingIssue::MissingSpaceAfterBox])
    } else if let Some(r) = trimmed.strip_prefix("- [-]") {
        (
            Status::Cancelled,
            r,
            vec![ParsingIssue::MissingSpaceAfterBox],
        )
    } else if let Some(r) = trimmed.strip_prefix("- [X]") {
        (
            Status::Done,
            r,
            vec![ParsingIssue::UppercaseX, ParsingIssue::MissingSpaceAfterBox],
        )
    } else {
        // Wrong char in box ( [o], [l] … )
        let stripped_first_part = trimmed.strip_prefix("- [");
        match stripped_first_part {
            Some(r) => {
                let stripped_second_part = r.drop_n_chars(1).strip_prefix("]");
                match stripped_second_part {
                    Some(r) => {
                        return Some((
                            depth,
                            indent,
                            Status::Todo,
                            r.to_string(),
                            vec![ParsingIssue::InvalidBox],
                        ));
                    }
                    None => {}
                }
            }
            _ => {}
        }

        return None;
    };

    Some((depth, indent, status, rest.trim_end().to_string(), issues))
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

// Scans the full title for `#` and `@` markers at any position (not just at
// whitespace boundaries). Markers may be embedded inside tokens, e.g.
// `(@bob)`, `(#feature)`, or `asdf#prop`. Everything that is not consumed
// as a marker is collected back into the returned title string.
//
// Quote policy — two cooperating mechanisms implement one rule:
//   1. `'` and `"` are stop bytes for name scanning, so a trailing quote is
//      never absorbed into a marker name (e.g. `feat'` → name is `feat`).
//   2. A `#`/`@` that is *immediately preceded* by `'` or `"` is skipped
//      entirely (not recognised as a marker start).
// Together these ensure that `'#feat'` and `"@alice"` are prose, while
// `(#feat)` and `asdf#feat` are markers.
fn parse_markers(title: &str) -> (Vec<Marker>, String) {
    let mut markers = Vec::new();
    let bytes = title.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    // Byte offset of the start of the next title fragment to keep.
    let mut title_keep_from = 0;
    // Fragments of the reconstructed plain title.
    let mut title_frags: Vec<&str> = Vec::new();

    while i < len {
        let b = bytes[i];
        if b == b'#' || b == b'@' {
            // Skip when immediately preceded by a quote — treat as prose.
            let preceded_by_quote = i > 0 && is_marker_quote(bytes[i - 1] as char);
            if preceded_by_quote {
                i += 1;
                continue;
            }

            // 1-based column of this `#`/`@` within the title string.
            let col = i + 1;
            let marker_byte = b;
            let name_start = i + 1;

            // Advance past the marker name: stop at whitespace or delimiter chars.
            let mut j = name_start;
            while j < len && !is_marker_stop_byte(bytes[j]) {
                j += 1;
            }
            let name = &title[name_start..j];

            let recognized = if marker_byte == b'#' {
                if let Some(m) = parse_hash_token(name, col) {
                    markers.push(m);
                    true
                } else {
                    false
                }
            } else {
                // '@'
                let clean = name.trim_end_matches(|c: char| MARKER_TRAILING_PUNCT.contains(c));
                if !clean.is_empty() {
                    markers.push(Marker::Assignment(AssignmentRef {
                        name: clean.to_string(),
                        column: col,
                    }));
                    true
                } else {
                    false
                }
            };

            if recognized {
                // Keep everything before this marker in the title.
                title_frags.push(&title[title_keep_from..i]);
                title_keep_from = j;
            }
            i = j;
        } else {
            i += 1;
        }
    }

    // Keep any trailing text after the last marker.
    title_frags.push(&title[title_keep_from..]);

    let raw = title_frags.concat();
    let clean_title = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (markers, clean_title)
}

fn is_marker_stop_byte(b: u8) -> bool {
    is_marker_boundary(b as char)
}

fn parse_hash_token(name: &str, column: usize) -> Option<Marker> {
    if name.is_empty() {
        return None;
    }

    // Known ALL-CAPS special markers checked explicitly; avoids misidentifying
    // a user property whose name happens to be all-caps.
    if let Some(special) = SpecialMarker::from_name(name, column) {
        return Some(Marker::Special(special));
    }

    // `#review...`  -- branch outcome not yet chosen
    if let Some(base) = name.strip_suffix("...") {
        if !base.is_empty() {
            return Some(Marker::Property(PropertyRef {
                name: base.to_string(),
                form: PropertyForm::BranchPending,
                column,
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
                column,
            }));
        }
    }

    // Plain property, possibly with trailing punctuation: `#feature:`
    let clean = name.trim_end_matches(|c: char| MARKER_TRAILING_PUNCT.contains(c));
    if clean.is_empty() {
        return None;
    }
    Some(Marker::Property(PropertyRef {
        name: clean.to_string(),
        form: PropertyForm::Full,
        column,
    }))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
