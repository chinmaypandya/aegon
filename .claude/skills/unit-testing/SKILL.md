---
name: unit-testing
description: >
  Diff-aware, batched test agent. After N features are written, invoke this skill once.
  It inspects what changed since the last test run (or a given base ref), identifies all
  affected public functions/modules, writes tests for the whole batch, runs only the affected
  tests (not the full suite), and iterates until green. Use when asked to "test what I just
  wrote", "run tests for these features", "batch test my changes", or "write tests for the diff".
---

# Unit Testing — Diff-Aware Batch Loop

Invoke once after writing multiple features. This skill discovers what changed, writes tests for
the entire batch at once, and runs only the affected subset — not the whole workspace suite,
not one test at a time.

---

## When to Invoke

**Do:** after finishing a batch of features (3–10 functions/modules worth of new code).
**Don't:** after every single edit (too noisy) or blindly on the full workspace (too slow).

The sweet spot is: "I wrote these 5 things, now go create tests for all of them and iterate on those until it looks stable."

---

## Roles

- **Test agent (you, running this skill):** owns test discovery, writing, and the fix loop.
- **Code implementer (the user or another agent):** owns production code. You flag bugs but
  never fix them — report clearly and wait before re-running.

---

## Phase 1 — Discover What Changed

Determine the set of changed files using the diff against the base:

```bash
git diff --name-only main          # if on a feature branch
git diff --name-only HEAD~<N>      # if committing as you go — use the right N
git status --short                 # for uncommitted changes
```

From the changed `.rs` files, extract:
- Every new or modified `pub fn`, `pub async fn`, `pub struct`, `pub enum`, or `impl` block.
- For each: the crate it lives in (from `Cargo.toml` in the nearest parent dir).

Group by crate. This is your **test target list** — one batch, not per-feature.

Skip files that are only test files themselves (`#[cfg(test)]` only, or in `tests/`), generated
code, or files where the only change is formatting.

---

## Phase 2 — Plan the Batch

Before writing a single test, output a plan:

```
## Test batch plan

Crate: <name>
Changed targets:
  - fn foo (src/foo.rs:12) — new
  - fn bar (src/bar.rs:44) — modified
  - struct Baz (src/baz.rs:8) — new

Existing tests that already cover any of the above: <list or "none">
Tests to write: <count>
```

If any changed target already has complete test coverage, skip it. Only write net-new tests.

---

## Phase 3 — Write All Tests (One Pass)

Write all tests for the entire batch in one pass — not function-by-function. Place each
test module in the same file as the code under test (inside `#[cfg(test)]`), or in
`tests/<module>.rs` if that's the project convention.

### Test structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn <function>_<scenario>() {
        // arrange
        // act
        // assert
    }
}
```

### Coverage priority per target

| Priority | What |
|----------|------|
| 1 | Happy path — expected inputs → expected outputs |
| 2 | Boundary values — empty, zero, max, single-element |
| 3 | Error paths — invalid input, out-of-range |
| 4 | Edge cases visible from the implementation |

### Rules

- One assertion concept per test.
- Name: `<fn_name>_<scenario>`, e.g. `parse_returns_err_on_empty_input`.
- No `println!` / `dbg!` — use `-- --nocapture` flag during debug runs only.
- No mocks unless the target does I/O, time, or external calls.

---

## Phase 4 — Run Only the Affected Tests

**Never run `just test` (full workspace) at this stage.** Run only the affected crates and filters:

```bash
# Run all tests in the affected crate(s)
cargo test -p <crate-name>

# If multiple crates changed, run each:
cargo test -p <crate-a> -p <crate-b>

# To filter down further to just the new test module:
cargo test -p <crate-name> <module_or_fn_filter>
```

Capture the full output — you need it for diagnosis.

---

## Phase 5 — Diagnose Failures (Loop, max 5 iterations)

For each failing test, classify:

| Category | Signal | Action |
|----------|--------|--------|
| Test is wrong | Assertion is incorrect; misread the spec | Fix the test |
| Test found a real bug | Code returns wrong value per its own contract | **Report to implementer** |
| Compile error in test | Missing import, wrong type, borrow issue | Fix the test |
| Compile error in prod code | Production code doesn't compile | **Report to implementer** |
| Flaky / ordering | Passes alone, fails in suite | Isolate; add `#[serial]` if needed |

### Reporting a bug to the implementer

Stop iterating and output:

```
╔══════════════════════════════════════════════════╗
║  BUG FOUND — implementer action required         ║
╠══════════════════════════════════════════════════╣
║  Function : <name>                               ║
║  File     : <path>:<line>                        ║
║  Test     : <test name>                          ║
╠══════════════════════════════════════════════════╣
║  Expected : <what the test expects>              ║
║  Got      : <what the code actually returned>    ║
╠══════════════════════════════════════════════════╣
║  Relevant output:                                ║
║  <paste the exact cargo test failure lines>      ║
╚══════════════════════════════════════════════════╝
```

Do not touch production code. Wait for the implementer to fix it, then re-run Phase 4.

### Fix and re-run

If the failure is in the test itself: fix it, re-run Phase 4, repeat.
Stop after **5 iterations**. If still failing, output a full summary and halt.

---

## Phase 6 — Finalize

Once the affected tests are green, hand off to `/lint-check` — do not run the full suite here.
CI owns the full `cargo test --workspace` run on push/PR.

If you suspect your changes broke pre-existing tests in another crate, note it in the handoff
summary so the developer can decide whether to run `just test` manually before pushing.

---

## Done Criteria

- [ ] All newly written tests pass (`cargo test -p <affected-crates>`).
- [ ] No `#[ignore]` left without an explanatory comment.
- [ ] Summary output: crates tested, test count added, any bugs reported to implementer.
- [ ] Hand off to `/lint-check` before committing.

---

## Common Pitfalls

- **Don't run `just test` first** — that's the end-of-loop smoke check, not the iteration tool.
- **Don't fix code bugs in tests** — asserting the wrong value just to make it green is lying.
- **Don't write one test at a time** — batch the whole diff, then run.
- **Don't re-run the full suite mid-loop** — it's slow and noisy; filter to the crate.
- **Match existing test conventions** — if the project uses `rstest` or `proptest`, follow that
  pattern rather than introducing a different style.
