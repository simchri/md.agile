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

#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Returns `true` if `c` is the single-tick character (`'`) used to fully
/// wrap a marker name and suppress its interpretation (`'#feat'`,
/// `'@alice'`). Double quotes (`"`) have no escaping effect at all; they are
/// reserved for the unrelated property-required-subtask quoting convention
/// (see `parse_subtask_kind`).
pub(crate) fn is_marker_tick(c: char) -> bool {
    c == '\''
}

/// Returns `true` if a marker name is fully wrapped in single ticks, given
/// the character immediately before the sigil (`before_sigil`, `None` at the
/// start of the string) and the character immediately after the name
/// (`after_name`, `None` at the end of the string). Suppression requires a
/// *matching* tick on **both** sides — a tick on only one side does not
/// suppress recognition (e.g. `weird'#feat` is still a marker). Shared by
/// [`parse_markers`] and `goto_definition::token_name_at_position` so the
/// paired-check logic lives in exactly one place.
pub(crate) fn is_tick_wrapped(before_sigil: Option<char>, after_name: Option<char>) -> bool {
    before_sigil.is_some_and(is_marker_tick) && after_name.is_some_and(is_marker_tick)
}

/// Returns `true` if `c` is the escape character that, when immediately
/// preceding a sigil, suppresses marker interpretation (`\#`, `\@`). The
/// backslash itself is dropped from the reconstructed title; the sigil is
/// kept as a literal character. Mirrors [`is_marker_tick`], but the tick
/// characters stay in the title while the backslash does not.
pub(crate) fn is_marker_escape(c: char) -> bool {
    c == '\\'
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
    /// No title text remains after the status box (and any markers are
    /// stripped): `- [ ] `, or a line consisting only of markers.
    EmptyTitle,
}

// ── Ordering ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    None,
    Ordered(u32), // the "1." prefix; enforces execution sequence among siblings
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
    /// The raw inner text of a `PropertyRequired` subtask before marker extraction
    /// (e.g. `"developer #review"` → `Some("developer #review")`).
    /// `None` for `Custom` subtasks.
    pub raw_title: Option<String>,
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
    // Empty when `#MILESTONE` is used with no name at all (e.g. bare
    // `#MILESTONE` or `#MILESTONE:`) -- still recognised as a milestone
    // header (rather than falling through as ordinary prose) so
    // `rules::missing_milestone_name` can flag it: README.md requires "a
    // milestone name must be provided".
    pub name: String,
    // Carries the source file path (unlike a bare `line: usize`) so
    // cross-file rules -- e.g. `rules::duplicate_milestone_name`, which
    // compares milestones across the whole project -- can report a proper
    // per-file location for each occurrence.
    pub location: Location,
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
    raw_title: Option<String>,
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
            raw_title: self.raw_title,
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
                location: Location {
                    path: path.clone(),
                    line: line_no,
                },
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
            // Quote-stripping (kind detection) must run *before* order-prefix
            // detection: a property-required subtask's order prefix can be
            // baked inside the quotes (e.g. `"1. dev implementation"`, per
            // README.vision.md "Ordered Tasks via Properties"), and the
            // leading `"` would otherwise make `parse_order_prefix` fail to
            // recognise the digit run. For `Custom` subtasks the order prefix
            // is still consumed (stripped from the title, as before); for
            // `PropertyRequired` subtasks it is only *detected*, not
            // consumed — `raw_title` must stay byte-identical to the
            // property's configured subtask string (order prefix included)
            // so E010/E012 matching keeps working unchanged.
            let (kind, unquoted, trailing_markers_src) = parse_subtask_kind(&rest);
            // Byte offset within the original title text where the trailing
            // marker suffix (if any) begins — needed to re-anchor its
            // markers' columns once parsed separately below.
            let trailing_offset = rest.len() - trailing_markers_src.len();
            let (order, rest) = match kind {
                SubtaskKind::Custom => parse_order_prefix(unquoted),
                SubtaskKind::PropertyRequired => {
                    let (order, _) = parse_order_prefix(unquoted);
                    (order, unquoted)
                }
            };
            let raw_title = match kind {
                SubtaskKind::PropertyRequired => Some(rest.to_string()),
                SubtaskKind::Custom => None,
            };
            let (mut markers, mut title) = parse_markers(rest);
            if kind == SubtaskKind::PropertyRequired && !trailing_markers_src.is_empty() {
                // Dynamic per-instance assignment: markers placed after the
                // closing quote (e.g. `"PO review" @alice`) are ordinary
                // markers on the subtask. They're kept out of raw_title (so
                // E010/E012 config matching stays byte-exact), but are
                // appended to the display `title` — like `Custom` subtasks,
                // callers that render the title should still show
                // `@alice`/`#feature` inline.
                let (trailing_markers, trailing_title) = parse_markers(trailing_markers_src);
                if !trailing_title.is_empty() {
                    title = format!("{title} {trailing_title}");
                }
                markers.extend(
                    trailing_markers
                        .into_iter()
                        .map(|m| shift_marker_column(m, trailing_offset)),
                );
            }
            let mut parsing_issues = parsing_issues;
            if title.trim().is_empty() {
                parsing_issues.push(ParsingIssue::EmptyTitle);
            }
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
                raw_title,
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
//
// Returns `Some(String::new())` for a bare `#MILESTONE`/`#MILESTONE:` with no
// name following it -- still recognised as a milestone header rather than
// falling through as ordinary prose, so `rules::missing_milestone_name` (E018)
// can flag the empty name per README.md's "a milestone name must be
// provided". Only the glued-suffix case below (`#MILESTONEfoo`) is treated as
// not a milestone line at all.
fn parse_milestone_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("#MILESTONE")?;
    // Require a non-alphanumeric boundary right after the tag, so that e.g.
    // `#MILESTONEfoo` (no punctuation/whitespace separator) is treated as
    // ordinary prose rather than misread as milestone "foo".
    if rest.starts_with(|c: char| c.is_alphanumeric()) {
        return None;
    }
    // Skip any leading non-alphanumeric chars (e.g. ": ")
    let name = rest.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '(');
    Some(name.trim_end().to_string())
}

// Strips a leading order number ("1. ") and returns the order and remaining text.
fn parse_order_prefix(title: &str) -> (Order, &str) {
    let bytes = title.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && bytes.get(i) == Some(&b'.') && bytes.get(i + 1) == Some(&b' ') {
        if let Ok(n) = title[..i].parse::<u32>() {
            return (Order::Ordered(n), title[i + 2..].trim_start());
        }
    }
    (Order::None, title)
}

// A title fully wrapped in `"..."` marks a property-required subtask; the
// quotes are stripped and the inner text is returned. A run of marker-shaped
// tokens (`#foo`, `@bar`) trailing *after* the closing quote — e.g.
// `"PO review" @alice`, the dynamic per-instance assignment syntax from
// doc/dynamic-assignment-mandatory-subtasks.md — is peeled off first so it
// doesn't prevent the quote-wrap from being recognised. The peeled-off
// suffix is returned separately so the caller can parse it as ordinary
// markers on the subtask, without it becoming part of the byte-exact
// raw_title used for config matching.
fn parse_subtask_kind(title: &str) -> (SubtaskKind, &str, &str) {
    let core_end = find_trailing_markers_start(title);
    let (core, trailing) = title.split_at(core_end);
    if core.len() >= 2 && core.starts_with('"') && core.ends_with('"') {
        (
            SubtaskKind::PropertyRequired,
            &core[1..core.len() - 1],
            trailing,
        )
    } else {
        (SubtaskKind::Custom, title, "")
    }
}

// Returns the byte offset in `s` where a trailing run of marker-shaped
// tokens (whitespace-separated words starting with `#` or `@`) begins, so
// that `s[..offset]` is the "core" content and `s[offset..]` is the trailing
// marker suffix (including its separating whitespace). Returns `s.len()` if
// there's no such trailing run.
fn find_trailing_markers_start(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut end = trim_trailing_ascii_whitespace(bytes, bytes.len());
    loop {
        let mut word_start = end;
        while word_start > 0 && !bytes[word_start - 1].is_ascii_whitespace() {
            word_start -= 1;
        }
        if word_start == end {
            break;
        }
        let word = &s[word_start..end];
        // A trailing marker word must not itself contain a `"` — otherwise
        // this would wrongly eat a closing quote that belongs to the
        // quoted title (e.g. `"developer #review"`, where `#review` is
        // *inside* the quotes, not a marker trailing them).
        if (word.starts_with('#') || word.starts_with('@')) && !word.contains('"') {
            end = trim_trailing_ascii_whitespace(bytes, word_start);
        } else {
            break;
        }
    }
    end
}

// Returns the largest `i <= end` such that `bytes[..i]` has no trailing
// ASCII whitespace byte.
fn trim_trailing_ascii_whitespace(bytes: &[u8], mut end: usize) -> usize {
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    end
}

// Shifts a marker's column by `offset` — used to re-anchor markers parsed
// from a substring (e.g. the trailing-marker suffix after a property-required
// subtask's closing quote) back to their real position within the subtask's
// full title text.
fn shift_marker_column(marker: Marker, offset: usize) -> Marker {
    match marker {
        Marker::Property(mut p) => {
            p.column += offset;
            Marker::Property(p)
        }
        Marker::Assignment(mut a) => {
            a.column += offset;
            Marker::Assignment(a)
        }
        Marker::Special(mut s) => {
            s.column += offset;
            Marker::Special(s)
        }
    }
}

// Scans the full title for `#` and `@` markers at any position (not just at
// whitespace boundaries). Markers may be embedded inside tokens, e.g.
// `(@bob)`, `(#feature)`, or `asdf#prop`. Everything that is not consumed
// as a marker is collected back into the returned title string.
//
// Quote policy: a `#`/`@` is treated as prose, not a marker start, only when
// the marker *name* is fully wrapped in single ticks — an opening `'`
// immediately before the sigil AND a closing `'` immediately after the name
// (e.g. `'#feat'`, `'@alice'`; this is the convention documented in
// README.md "Properties can also be added to subtasks"). A tick on only one
// side does *not* suppress recognition (e.g. `weird'#feat` is still a
// marker) — this asymmetric, single-sided rule was a past bug, since fixed.
// Double quotes have no escaping effect at all: `"#feat"` and `"@alice"` are
// real markers (double quotes are reserved for the unrelated
// property-required-subtask quoting convention, handled in
// `parse_subtask_kind`, not here). `'` and `"` remain stop bytes for name
// scanning either way, so a trailing quote is never absorbed into a marker
// name (e.g. `feat'` → name is `feat`).
//
// Escape policy: a `#`/`@` immediately preceded by a backslash (`\#`, `\@`)
// is also treated as prose, not a marker start — but unlike the tick rule,
// the backslash itself is dropped from the reconstructed title, leaving only
// the literal sigil (e.g. `\#not_a_property` → `#not_a_property` in the
// title, no Property marker recorded).
/// Extracts `#`/`@` markers from `title` for the `markers` list, while
/// returning a "clean" title that keeps the marker text itself (unlike raw
/// escape sequences, which still have their backslash stripped). Recognized
/// markers are left in place so callers that display the title (e.g.
/// `agile task next`) still show `#feature`/`@alice` inline; callers that
/// need marker-free text (e.g. fuzzy title-similarity matching in
/// `eta`/`lifecycle_cache`) are expected to strip markers themselves if
/// needed.
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
        if (b == b'#' || b == b'@') && i > 0 && is_marker_escape(bytes[i - 1] as char) {
            // Escaped sigil: keep text up to (but not including) the
            // backslash, then keep the literal sigil itself, and resume
            // scanning right after it. The backslash is dropped.
            title_frags.push(&title[title_keep_from..i - 1]);
            title_frags.push(&title[i..i + 1]);
            title_keep_from = i + 1;
            i += 1;
            continue;
        }
        if b == b'#' || b == b'@' {
            // Look ahead to the end of the marker name up front — needed
            // both for the tick-wrap check below and for the marker
            // itself, so it's computed once here instead of twice.
            let name_start = i + 1;
            let mut j = name_start;
            while j < len && !is_marker_stop_byte(bytes[j]) {
                j += 1;
            }

            // Single-tick wrap rule: only a *matching* opening AND closing
            // tick suppresses recognition — a lone tick on one side does not.
            let before_sigil = if i > 0 {
                Some(bytes[i - 1] as char)
            } else {
                None
            };
            let after_name = bytes.get(j).map(|&c| c as char);
            if is_tick_wrapped(before_sigil, after_name) {
                i += 1;
                continue;
            }

            // 1-based column of this `#`/`@` within the title string.
            let col = i + 1;
            let marker_byte = b;
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
                // Keep the marker text itself in the title (unlike escaped
                // sigils, which drop their backslash) — display should
                // still show `#feature`/`@alice` inline.
                title_frags.push(&title[title_keep_from..j]);
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
