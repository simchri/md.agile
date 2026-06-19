# E010 — Missing Required Subtasks

## Decisions
- Option A: `Subtask` gets `raw_title: Option<String>` for `PropertyRequired` nodes (pre-marker-parsing text), used for exact matching against TOML config strings
- Full recursion: rule applies to subtasks too (nested properties checked in same pass)
- Defer `subtasks_allow_cancel`

## Steps

- [x] 1. `config/mod.rs` — add `subtasks: Vec<String>` to `PropertyConfig`; extend `RawConfig` to deserialize the `subtasks` array from `[Properties.X]` TOML sections
- [x] 2. `parser/mod.rs` — add `raw_title: Option<String>` to `Subtask`; populate with inner quoted text for `PropertyRequired` nodes before `parse_markers` strips it
- [x] 3. `rules/missing_required_subtasks/mod.rs` (new, E010) — for each task/subtask: collect properties with declared subtasks, merge required strings across multiple properties, check direct children for matching `PropertyRequired` nodes by `raw_title`, report missing; recurse into subtasks
- [x] 4. `rules/mod.rs` — register `ErrorCode::MissingRequiredSubtasks` (E010), wire into `check_all`
- [x] 5. Tests — single property, multiple properties on same task, nested properties, subtask with property, all present (no error), some missing (error with list)
