---
name: Never duplicate production logic in tests
description: Tests must call the real production functions, not inline copies of their logic
type: feedback
---

Never copy production logic into test helpers. If a function is needed in a test, extract it to lib.rs and use it from both the test and production code.

**Why:** User explicitly corrected this — a test helper that replicates what main.rs does is not testing main.rs, it's testing a copy. Bugs in the real code can go undetected.

**How to apply:** When writing a test that needs logic currently in main.rs or another untestable location, move that logic to lib.rs first, then use it in both places. Never write a `fn collect(...)` or similar helper in a test file that reimplements production behaviour.
