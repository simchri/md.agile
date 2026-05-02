# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project 
The vision for this project is defined in [README.vision.md](README.vision.md). AI agents should never change this file, unless explicitly requested by the user.
The philosophy for this project is defined in [MANIFESTO.md](MANIFESTO.md). AI agents should never change this file, unless explicitly requested by the user.

## Commands

### Run / develop
```bash
cargo run
```

### Test
```bash
cargo test                                                            # full suite
cargo test --lib -- <test_name>                                       # single unit test
cargo test --test acceptance-tests -- --name "<scenario name>"        # acceptance test
```

count tests in the project: (don't run)
```bash
cargo test -- --list | grep -c "^" # count tests
```

### Docker dev environment

The project is configured for development in a docker container, where the project sources are mounted.
Claude runs inside this container, so host resources are not accessible.

```bash
docker compose run dev-container-no-gpu    # start dev container (no GPU needed)
docker compose build                       # rebuild after Dockerfile changes
```

## Toolchain

Rust nightly (pinned in `docker/Dockerfile` by digest). Edition 2024.

## Architecture

Planned module layout (not all exist yet):
- `src/parser/` — parse `.agile.md` files into an AST
- `src/checker/` — validate parsed tasks against rules (used by `agile check`)
- `src/rules/` — business logic; all testable logic lives here, not in components
- `src/ui/` — TUI/GUI components; business-logic-free
- `src/lsp/` — Language Server Protocol implementation

The CLI binary is `agile`. It reads `*.agile.md` files at any depth from the project root. The project's own backlog is `tasks.agile.md`, written in the mdagile syntax defined in `README.vision.md`.

Project-level configuration (properties, users, groups) lives in `mdagile.toml`.

## Development rules

### Sample text in tests

Always write inline file content using the `"\` continuation style so the content renders exactly as it would in a real file — indentation and all:

```rust
let input = "\
- [ ] top task
  - [ ] subtask
    - [ ] nested
";
```

Never use embedded `\n` escapes or string concatenation for multi-line file samples. The goal is that a reader can see indentation and structure at a glance, just as they would in an actual `.agile.md` file.

### TDD — mandatory for all code changes

Follow red-green cycle strictly:

1. **Write the test first** — unit test in the relevant `*_tests.rs` file, or a Cucumber scenario in `tests/acceptance/` for behaviour-level changes.
2. **Run it and confirm it fails** — `cargo test --lib -- test_name` or `cargo test --test acceptance-tests -- --name "scenario name"`. Do not proceed until you see the expected failure.
3. **Write the minimum code to make it pass** — no more.
4. **Run the test again and confirm it passes.**
5. **Run the full suite** (`cargo test`) to check for regressions before finishing.

Never write production code without a failing test that justifies it.

**Exception**: `src/components/` is exempt — GUI changes are hard to unit-test and do not require a failing test first. However, keep business logic out of components: any logic that can be tested belongs in `src/rules/` (or another non-UI module), not in a component. Components should only read state and dispatch actions.

### Auto-commit on vibe branches

Whenever a prompt results in code changes (modifications to versioned files or new files to be versioned), **automatically create a commit** — but **only if**:
- the current branch starts with `vibe` (e.g., `vibes01`, `vibe-feature-x`)

**Test requirements**:
- If only `.md` files are modified: commit directly without running tests.
- If any `.rs` or other code files are modified: run `cargo test` first and ensure all tests pass before committing.
- **Exception**: `tasks.agile.md` must be validated with `agile check` before committing (it's a `.agile.md` file, not documentation).

If tests fail, fix them first — do not commit a red suite.

If the branch does **not** start with `vibe`, warn the user instead and do not commit.

**Commit format**:
- **Short summary**: `(Claude) <description>` — e.g., `(Claude) add confetti animation to Done button`
- **Body**: 
  - Include a detailed explanation of the changes, followed by the verbatim user prompt that triggered the changes
  - Include the total number of tests in the project (c.f. above for how to count). Format: "total tests: <num tests>"

Example:
```
(Claude) add confetti animation to Done button

Added CSS confetti animation that plays when the user clicks "Done!" to complete a task.
- Created assets/confetti.css with particle animation (2.5s duration)
- Updated home.rs to trigger animation on button click
- Integrated CSS into app.rs stylesheet list

User prompt:
> add a simple css confetti animation to the "Done" button (when completing a task)
```

### Task Management in tasks.agile.md

Use `tasks.agile.md` as the authoritative record of work completed and planned:
- **When work is completed**: Mark it as done `[x]` in `tasks.agile.md` immediately, even if it's already committed.
- **When the user prompts something new**: If it's not already in `tasks.agile.md`, create a new task in a suitable location (under an appropriate section/heading, or create a new section if needed).
- **Keep it current**: This file serves as a running history of what's been built and what's planned next. Use it to avoid duplicate work and to understand project momentum.

## Terminology

See `GLOSSARY.md` for precise definitions. Key terms used throughout the codebase and vision docs:
- **Marker** — any `#word` or `@word` token
- **Property** — a user-defined `#marker` declared in `mdagile.toml`
- **Assignment** — a `@marker` assigning a task to a user or group
- **Special Marker** — an ALL-CAPS built-in keyword (e.g. `#OPT`, `#MILESTONE`, `#MDAGILE`)

Do not use these terms interchangeably.

## Known issues / gotchas
...

