---
name: Commit only on green tests
description: Auto-commits on vibe branches require a passing test suite first
type: feedback
---

Only auto-commit on vibe branches when `cargo test` passes with no failures. Run the full suite before every commit. If tests are red, fix them first — never commit a failing suite.

**Why:** User corrected this explicitly; a red commit on a vibe branch is not acceptable.

**How to apply:** Before every `git commit` on a vibe branch, run `cargo test` and verify it's green. If it's red, fix the failures (or hold the commit) before proceeding.
