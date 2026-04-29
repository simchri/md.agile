# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project 
The vision for this project is defined in [README.vision.md](README.vision.md). AI agents should never change this file, unless explicitly requested by the user.
The philosophy for this project is defined in [MANIFESTO.md](MANIFESTO.md). AI agents should never change this file, unless explicitly requested by the user.

## Commands

### Run / develop
```bash
...
```

### Test
```bash
...
```


### Docker dev environment

The project is configured for development in a docker container, where the project sources are mounted.
This is usually not very relevant to Claude - because we run Claude inside of this container. However the AI agent can only see container file + mounted sources and may have only limited access to the host's resources.

## Architecture
...

## Development rules

### TDD — mandatory for all code changes

Follow red-green cycle strictly:

1. **Write the test first** — unit test in the relevant `*_tests.rs` file, or a Cucumber scenario in `tests/acceptance/` for behaviour-level changes.
2. **Run it and confirm it fails** — `cargo test --lib -- test_name` or `cargo test --test acceptance-tests -- --name "scenario name"`. Do not proceed until you see the expected failure.
3. **Write the minimum code to make it pass** — no more.
4. **Run the test again and confirm it passes.**
5. **Run the full suite** (`cargo test`) to check for regressions before finishing.

Never write production code without a failing test that justifies it.

**Exception**: `src/components/` is exempt — GUI changes are hard to unit-test and do not require a failing test first. However, keep business logic out of components: any logic that can be tested belongs in `src/bucket/` (or another non-UI module), not in a component. Components should only read state and dispatch actions.

### Auto-commit on vibe branches

Whenever a prompt results in code changes (modifications to versioned files or new files to be versioned), **automatically create a commit** — but **only if the current branch starts with `vibe`** (e.g., `vibes01`, `vibe-feature-x`).

If the branch does **not** start with `vibe`, warn the user instead and do not commit.

**Commit format**:
- **Short summary**: `(Claude) <description>` — e.g., `(Claude) add confetti animation to Done button`
- **Body**: Include a detailed explanation of the changes, followed by the verbatim user prompt that triggered the changes

Example:
```
(Claude) add confetti animation to Done button

Added CSS confetti animation that plays when the user clicks "Done!" to complete a workout.
- Created assets/confetti.css with particle animation (2.5s duration)
- Updated home.rs to trigger animation on button click
- Integrated CSS into app.rs stylesheet list

User prompt:
> add a simple css confetti animation to the "Done" button (when completing a workout)
```

## Known issues / gotchas
...

