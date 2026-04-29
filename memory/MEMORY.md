# Memory Index

- [Commit only on green tests](feedback_commit_on_green.md) — auto-commit on vibe branches requires `cargo test` to pass first
- [Never duplicate production logic in tests](feedback_no_duplicate_logic_in_tests.md) — extract to lib.rs and use the real function; never copy logic into a test helper
