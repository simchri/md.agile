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
- Branch Property: A Property with named outcome branches (e.g. `#review...` resolves to `#review:passed` or `#review:failed`). The outcome must be set before the Task can be completed.
- Neighbor Task (!= Sibling (Task)): A Task that must exist at the same Sibling level as the Task bearing a Property that declares it. Mainly used with Branch Properties but applicable to any Property.
- Nested Property: A Property referenced by name within another Property's subtask list (e.g. `subtasks = ["developer #review"]`). When the outer Property is applied to a Task, the inner Property's own Subtasks become required at the next nesting level. Enables reuse of common workflow steps across multiple Properties.
- Required Property: A Property declared mandatory for all Tasks in a file via `#MDAGILE.file.mandatory_property=...`.
- Ordered Task: A Subtask prefixed with a number and `.` (e.g. `1.`) to enforce execution sequence among siblings.

## Planning

- Milestone: A `#MILESTONE: name` divider in a Task file. Reached when all Tasks preceding it are complete.
- ETA (Estimated Time of Arrival): Projected completion date for a Milestone, derived from Task count and past velocity. No per-Task estimates are used (see MANIFESTO.md — #NoEstimates).
- Task Weight: Numeric value used in ETA calculation. A Task itself has weight 1; a Subtask at nesting level n has weight 1/n.
