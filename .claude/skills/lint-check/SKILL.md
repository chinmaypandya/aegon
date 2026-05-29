---
name: lint-check
description: >
  Local lint, format, and clippy enforcement — mirrors exactly what CI runs so failures
  are reproducible locally. Use when asked to "lint", "run clippy", "format code", "check
  before committing", or after unit-testing passes and before pushing.
---

# Lint Check — Local CI Mirror

Runs the same format, lint, and static analysis checks that CI enforces. If this passes
locally, the CI lint job passes too.

---

## When to Run

- After `/unit-testing` passes, before `just commit` / `just push`.
- Any time you want to validate code quality without running tests.
- As a first step when CI fails on a lint/clippy job before reaching for `/ci-debug`.

---

## Check Sequence

Run in order — stop and fix at the first failure before continuing.

To run all three checks at once (mirrors the `fmt`, `clippy`, `check` jobs in `ci.yml`):

```bash
just ci-lint
```

Or individually — stop and fix at the first failure before continuing.

### 1. Format

```bash
just fmt          # apply formatting
just fmt-check    # check only, no changes (what CI runs)
```

If `just fmt` changes files, review them (whitespace/ordering only), stage, and proceed.
Never commit unformatted code.

### 2. Clippy

```bash
just clippy
```

Runs `cargo clippy --workspace --all-targets -- -D warnings`. All warnings are errors.

Apply fixes directly. Do not `#[allow(...)]` unless it is a known false positive — and leave
a comment explaining why if you do.

### 3. Compile Check

```bash
just check
```

Catches compile errors across all targets without a full build. Fix before proceeding.

---

## Done Criteria

- [ ] `just fmt` — no file changes on second run
- [ ] `just clippy` — zero warnings
- [ ] `just check` — zero errors

When done: proceed to `just commit` via `/git-workflows`.

---

## Common Clippy Fixes

| Lint | Fix |
|------|-----|
| `needless_return` | Remove explicit `return` |
| `clone_on_copy` | Remove `.clone()` on `Copy` types |
| `map_unwrap_or` | Use `.map_or(default, f)` |
| `redundant_closure` | Replace `\|x\| f(x)` with `f` |
| `dead_code` | Delete or make `pub` if intentional |

---

## What This Skill Does NOT Cover

- Writing or running tests → `/unit-testing`
- Diagnosing CI environment failures → `/ci-debug`
- Git operations → `/git-workflows`
