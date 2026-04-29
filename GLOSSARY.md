# Glossary

- Md.Agile (alt: Mdagile): The project name. The preferred spelling in text is "Mdagile" (intra-word punctuation is confusing)

## Core Syntax

- Task: A top-level checklist item (`- [ ] ...`). Distinct from a Subtask.
- Subtask: A nested Task, indented under a parent Task or Subtask.
- Subtask Level: The nesting depth of a Subtask, starting at 2 for direct children of a Task. A Task can also be considered a Subtask with Subtask Level 1.
- Sibling (Task): A Subtask on the same Subtask Level as the current Task.
- Task Title: The text on the checkbox line itself.
- Task Body: Free-text lines immediately following a Task (before the next blank line).
- Cancelled Task: A Task marked `- [-]`. Explicitly skipped — distinct from done.

## Syntax Related to Markers & Properties

- Marker: Word prefixed by a "#" or "@", e.g. user defined properties are identified with markers `#my_property`.
- Property: User defined "#" markers
- Assignment: User defined "@" marker
- Special Marker: ALL CAPS word prefixed by a hash tag, recognized by plaintask as a keyword ( e.g. `#MANIFEST, #OPT`)

## Subtask Flavours

- Mandatory Subtask: The default kind of Subtask. A parent Task cannot be completed until all Mandatory Subtasks are done.
- Optional Subtask: A Subtask prefixed with `#OPT`. Does not block parent completion.

## Properties & Workflow

- Short Form: An alias for a Property (e.g. `#feat`) that marks a Task as "brainstormed" without requiring its Subtasks. A Task that has one or more Properties in Short Form cannot be marked complete.
- Branch Property: A Property with named outcome branches (e.g. `#review...` resolves to `#review:passed` or `#review:failed`). The outcome must be set before the Task can be completed.
- Neighbor Task (!= Sibling (Task)): A Task that must exist at the same sibling level when a Branch Property resolves to a specific outcome.
- Nested Property: A Property referenced by name within another Property's subtask list (e.g. `subtasks = ["developer #review"]`). When the outer Property is applied to a Task, the inner Property's own Subtasks become required at the next nesting level. Enables reuse of common workflow steps across multiple Properties.
- Required Property: A Property declared mandatory for all Tasks in a file via `#MDAGILE.file.mandatory_property=...`.
- Ordered Task: A Subtask prefixed with a number and `.` (e.g. `1.`) to enforce execution sequence among siblings.

## Planning

- Milestone: A `#MILESTONE: name` divider in a Task file. Reached when all Tasks preceding it are complete.
- ETA (Estimated Time of Arrival): Projected completion date for a Milestone, derived from Task count and past velocity. No per-Task estimates are used (see MANIFESTO.md — #NoEstimates).
- Task Weight: Numeric value used in ETA calculation. A Task itself has weight 1; a Subtask at nesting level n has weight 1/n.
