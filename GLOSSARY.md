# Glossary

- Md.Agile (alt: Mdagile): The project name. The preferred spelling in text is "Mdagile" (intra-word punctuation is confusing)

## Core Concepts

- Task: A top-level checklist item (`- [ ] ...`). Distinct from a Subtask.
- Subtask: A nested Task, indented under a parent Task or Subtask.
- Subtask Level: The nesting depth of a Subtask, starting at 2 for direct children of a Task. A Task can also be considered a Subtask with Subtask Level 1.
- Sibling (Task): A Subtask on the same Subtask Level as the current Task.

- Cancelled Task: A Task marked `- [-]`. Explicitly skipped — distinct from done.
- Done Task: A Task marked `- [x]`. Completed successfully.

## Basic Task Syntax

- Task Title: The text on the checkbox line itself.
- Task Body: Free-text lines immediately following a Task (before the next blank line).
- Other Content: Any text that is not part of a Task or Subtask, such as free-form notes, comments, or descriptions. Not associated with any Task.

## Priority

- Eligibility: Whether a Task/Subtask may be selected for a given identity under `--mine`/`--as`. An explicit Assignment to someone else makes it ineligible; a Task broken into Subtasks is eligible iff at least one of its Todo Subtasks is itself eligible (or none remain — nothing left to delegate). Filters/selects Tasks for `agile task next`/`list --mine`/`--as`.

- Offset: The `N` in `agile task next N` — skip the first N-1 matching Tasks and select the Nth. "Matching" means every not-done top-level Task in document order (see Rank) by default, or only Eligible Tasks for the given identity when `--mine`/`--as` is given.

- Rank (alt: Priority Rank, Address, Task Address): A Task's or Subtask's position reading the file top to bottom, ignoring Eligibility. The first not-fully-done top-level Task is rank "1", the next is "2", etc.; fully-done top-level Tasks have no rank. Subtasks are numbered hierarchically ("1.1", "1.2", ... recursively) and keep a rank regardless of their own status, as long as their parent isn't fully done. Printed by `agile task next` and resolved by `agile task done`/`undone`. `--mine`/`--as` only change which Task an Offset *selects*, never the Rank *printed* once found — so `done`/`undone` (no identity filter) always resolve a given Rank to the same Task. Distinct from Order Number (see Ordered Task): Rank is never written explicitly in a file, unlike an Order Number's explicit `1.`/`2.` prefix.

- Reverse Rank (alt: Reverse Priority Rank, Reverse Address, Reverse Task Address): Rank counted from the bottom, based on closed work (Done or Cancelled, itself or any descendant) instead of not-done status. The last top-level Task with closed work is reverse rank "1", the previous one "2", etc. Subtasks follow the same hierarchical scheme as Rank. Printed by `agile task previous` and resolved by `agile task undone`. A Task can have both a Rank and a Reverse Rank at once (e.g. partially done).

## Markers

- Marker: Word prefixed by a "#" or "@", e.g. user defined properties are identified with markers `#my_property`.
- Special Marker: ALL CAPS word prefixed by a hash tag, recognized by mdagile as a keyword (e.g. `#MILESTONE, #OPT`)

## Properties

- Property: A named workflow rule declared in `mdagile.toml`. Defines optional Subtasks, a Short Form alias, ordering constraints, and other rules. Applied to a Task via a Property Marker. Using a `#name` token without a corresponding `[Properties.name]` declaration is an error.
- Property Marker: A `#name` token in a task file that applies a declared Property to a Task.

## Assignments

- Assignment: The association of a Task with a declared User or Group. When a Task has an Assignment, only the assigned User(s) or Group member(s) may mark it complete. Users and Groups must be declared in `mdagile.toml`. Using `@name` without a corresponding `[Users.name]` or `[Groups.name]` declaration is an error.
- Assignment Marker: A `@name` token in a task file that creates an Assignment to a declared User or Group.

## Subtask Flavours

- Mandatory Subtask: The default kind of Subtask. A parent Task cannot be completed until all Mandatory Subtasks are done.
- Optional Subtask: A Subtask prefixed with `#OPT`. Does not block parent completion.

## Properties & Workflow

- Short Form: An alias for a Property (e.g. `#feat`) that marks a Task as "brainstormed" without requiring its Subtasks. A Task that has one or more Properties in Short Form cannot be marked complete.
- Branch Property: A Property with named outcome branches (e.g. `#review:...` resolves to `#review:passed` or `#review:failed`). The outcome must be set before the Task can be completed.
- Neighbor Task (!= Sibling (Task)): (required sibling task) A Task that must exist at the same Sibling level as the Task bearing a Property that declares it. Mainly used with Branch Properties but applicable to any Property.
- Nested Property: A Property referenced by name within another Property's subtask list (e.g. `subtasks = ["developer #review"]`). When the outer Property is applied to a Task, the inner Property's own Subtasks become required at the next nesting level. Enables reuse of common workflow steps across multiple Properties.
- Required Property: A Property declared mandatory for all Tasks in a file via `#MDAGILE.file.mandatory_property=...`.
- Ordered Task: A Subtask prefixed with a number and `.` (e.g. `1.`) — its Order Number — to enforce execution sequence among direct siblings. Do not confuse with "Rank".

## Planning

- Milestone: A `#MILESTONE: name` divider in a Task file. Reached when all Tasks preceding it are complete.
- ETA (Estimated Time of Arrival): Projected completion date for a Milestone, derived from Task count and past velocity. No per-Task estimates are used (see MANIFESTO.md — #NoEstimates).
- Task Weight: Numeric value used in ETA calculation. A Task itself has weight 1; a Subtask at nesting level n has weight 1/n.

