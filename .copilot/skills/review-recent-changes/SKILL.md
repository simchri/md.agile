---
name: review-recent-changes
description: >
  Use when the user asks to review recent changes, a diff, or a series of
  commits to find refactoring opportunities — not for reviewing for bugs/
  security (see code-review/security-review) and not for implementing a
  specific already-known refactor.
---

# Review Recent Changes for Refactoring Opportunities

## Scope
Operates on a git range (default: commits since the last "clean" checkpoint,
or a user-specified range/PR/working tree diff). Read-only by default —
report findings first; only implement when the user asks (per CLAUDE.md:
never make unprompted file changes).

## What to look for (recurring categories observed in this project's history)

1. **Mixed concerns in one function/type** — e.g. a "compute_trends" function
   that also downsamples and prepares plot geometry. Flag any function/struct
   doing more than its name promises.
2. **Duplicated logic** — copy-pasted blocks across call sites (e.g. repeated
   trend-fitting code, repeated civil-calendar math). Exclude duplication that
   is just a type/unit conversion forced by a library.
3. **Misplaced logic** — code living in the wrong module (e.g. geometry/
   plotting code inside a "trends" math module; it belongs in the renderer).
4. **Stringly-typed / weakly-typed data** — raw `String` for dates, string
   literals for error codes — where a proper type (`NaiveDate`, enum) exists
   or is cheap to introduce.
5. **Misleading or stale names** — types/variables/fields whose name no
   longer matches their responsibility after earlier refactors (e.g. a
   generic `trends` variable actually holding plot-ready data).
6. **Unnecessary `Option`/indirection** — values wrapped in `Option` that are
   always present once their invariants hold.
7. **File/module size & responsibility splits** — large files doing several
   distinct jobs (e.g. one file rendering 3 different chart backends) that
   should become a directory module split by responsibility.
8. **Dead code / low-value tests** — code or tests that no longer earn their
   keep (e.g. a flaky test for untested-on-purpose logic) — confirm with the
   user before deleting.
9. **Correctness bugs surfaced by review** — note any bug found while
   tracing the above (e.g. coordinate-system mismatches, clamping vs.
   clipping) as a separate, flagged item — don't silently fold it into a
   "refactor".

## Procedure
1. Gather the diff/commit range (`git log`, `git diff`) and read the full
   touched files, not just the hunks — context is required to spot misplaced
   concerns.
2. For each candidate, write one line: `file:line — issue — proposed change`.
3. Group findings by category (above). Do **not** fix style nits or
   unrelated pre-existing issues.
4. Present the findings list to the user and stop. For any item that
   implies a non-trivial design decision (new type, moved responsibility,
   changed public interface), call out the decision explicitly and wait for
   confirmation before implementing (per CLAUDE.md).
5. If the user selects items to implement:
   - Follow strict TDD (failing test → minimal fix → pass → full suite).
   - One concern per commit, `refactor: <what/why>` message, matching this
     project's commit conventions (see CLAUDE.md auto-commit rules).
   - After each change, re-scan nearby code for the same bug/issue class
     ("scan again for similar issues") before moving to the next item.
   - Run `cargo fmt`, full `cargo test`, and update `tasks.agile.md` /
     `tests_count.txt` per project conventions.

## Non-goals
- Not a security or correctness-only review (use code-review/security-review
  agents for those).
- Not for applying a refactor the user already fully specified (just do it
  directly in that case).
