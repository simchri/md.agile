# Design Note: Dynamic Assignment of Mandatory (Property-Required) Subtasks

> Status: the "dynamic assignment" feature (Option 1, marker after the
> closing quote) is implemented — see `parse_subtask_kind` in
> `crates/cli/src/parser/mod.rs`. The quote-adjacency escaping rule
> follow-up (see below) is also implemented.

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

Also note: since double quotes no longer have any escaping effect (see
"Follow-up" below, now resolved), `"some mandatory subtask"@foo` (no space)
**does** parse `@foo` as a real marker even without a separating space — the
`"` no longer suppresses it. A space is still recommended for readability
(`"some mandatory subtask" @foo`) but is no longer functionally required.

## Follow-up: quote-adjacency escaping rule — resolved

Tracked separately in `1_tasks.agile.md` (see "Compatibility of Assignments
with mandatory subtasks"). **Implemented.** Only two escaping mechanisms are
now supported:

- `\` immediately before `#`/`@` (unchanged) — the backslash is dropped, the
  sigil becomes a literal character.
- Single ticks **fully surrounding** the marker term — an opening `'`
  immediately before the sigil **and** a closing `'` immediately after the
  marker name, e.g. `'@alice'`, `'#feat'`. A tick on only one side no longer
  suppresses recognition (e.g. `weird'#feat` is now correctly still a real
  marker — this asymmetric single-sided trigger was the bug).

Double quotes (`"`) have **no escaping effect at all** — `"@alice"` and
`"#feat"` are now real markers. `"` remains reserved for the unrelated
property-required-subtask quoting convention (`parse_subtask_kind`), which
this change does not touch.

`is_marker_quote` was renamed to `is_marker_tick` in `parser/mod.rs`
(mirrored in `lsp/goto_definition.rs`), since it now only ever checks for a
single-quote character, and the caller is responsible for pairing an opening
and closing occurrence rather than checking a lone preceding character.
