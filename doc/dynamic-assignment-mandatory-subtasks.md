# Design Note: Dynamic Assignment of Mandatory (Property-Required) Subtasks

> Status: proposal / not yet implemented. Captures a design discussion; no
> code has been changed as a result of this note.

## Problem

Mandatory subtasks come from `[Properties.X].subtasks` in `mdagile.toml` — a
single shared list of literal strings applied to every task carrying that
property. A subtask line is only recognized as satisfying a requirement via
an **exact match** between the quoted subtask text (`raw_title`) and the
configured string (used by E010 "missing required subtasks", E011
"unrequired quoted subtask", `invalid_order`, and the E010 autofix).

There is currently no way to attach a per-instance `@assignee` to a
property-required subtask: the assignee varies case-by-case per task
instance, but the required-subtask string is shared, project-wide config.
Embedding `@foo` inside the quotes (`"some mandatory subtask @foo"`) breaks
the exact-match against the configured string (`some mandatory subtask`),
triggering false E010/E011 errors.

## Options considered

1. **Marker after the closing quote**: `"some mandatory subtask" @foo`.
   `raw_title` stays exactly the configured string (identity is preserved);
   the trailing `@foo` is parsed as an ordinary `Marker::Assignment`, no
   different from an assignment on a `Custom` subtask. **Chosen direction.**
   Requires `parse_subtask_kind` to strip trailing markers before checking
   the quote-wrap, rather than requiring the whole raw line to be exactly
   `"..."` with nothing following.
2. **Marker inside the quotes, relax matching** to compare a
   marker-stripped title rather than exact `raw_title` bytes. Rejected: `#`
   markers embedded in quotes are already meaningful (nested properties, see
   README.md "Properties can be nested") and are part of the matched
   identity, so "strip `@` but not `#`" is an inconsistent, hard-to-explain
   rule, and touches every consumer of `raw_title` (missing/unrequired/
   invalid_order/autofix).
3. **Marker in the task/subtask body.** Rejected for now: body text is
   currently unstructured prose, never marker-scanned by any rule. Overloading
   it with machine-parsed semantics is a much larger, cross-cutting change
   (new parsing, new rule behavior, GLOSSARY.md updates) and risks false
   marker detection in existing free-form notes.

## Known edge case in Option 1 (accepted)

Classifying `PropertyRequired` by "strip trailing markers, then check if the
remainder is fully quote-wrapped" can misclassify a **custom** task/subtask
whose title happens to be entirely wrapped in quotes plus trailing markers,
e.g.:

```md
- [ ] "ship the release" #foo @someone
```

Today this is `Custom` (the trailing content after the closing quote
already breaks the wrap-check). Under Option 1 it would reclassify as
`PropertyRequired`, and — since `"ship the release"` is not declared as a
required subtask by any property in scope — would spuriously trigger E011
("unrequired quoted subtask").

**Decision: accept this as a known, rare edge case.** Users who hit it can
work around it by adding any other unquoted word to the title (breaking the
full quote-wrap), e.g. `- [ ] note: "ship the release" #foo @someone`. This
is considered acceptable because fully-quoting an entire custom title with
no other surrounding words is uncommon in practice.

Also note (regardless of which option is chosen): the existing
quote-adjacency rule in `parse_markers` (`is_marker_quote`) treats a
`#`/`@` immediately preceded by `'`/`"` as prose. This means
`"some mandatory subtask"@foo` (no space) would **not** parse `@foo` as a
marker at all — a space is required: `"some mandatory subtask" @foo`. See
the follow-up item below about revisiting this rule.

## Follow-up: revisit the quote-adjacency escaping rule

Tracked separately in `1_tasks.agile.md` (see "Compatibility of Assignments
with mandatory subtasks"). Current rule: a single leading quote character
immediately before `#`/`@` is enough to suppress marker recognition (used to
make `"@alice"` render as literal prose). Candidate re-designs, to be
evaluated in more detail before implementing:

- The single-quote-prefix trigger may be too broad/accidental as an
  exclusion rule; if kept, requiring the term to be **fully quoted**
  (`"@alice"`, not just preceded by a stray `"`) would be more deliberate.
- The existing `\#`/`\@` backslash-escape mechanism may already cover this
  need entirely — the quote-adjacency rule might not be needed at all.
- Alternative: reserve single ticks (`'`) as the literal/escaping quote
  convention instead of (or in addition to) `"` — rarer in prose, but
  familiar to programmers from shell quoting conventions.
