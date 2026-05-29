---
name: ci-debug
description: >
  Reproduces a failing CI job locally inside a subprocess or Docker container that mirrors
  the CI environment. Use when CI fails and you need to understand why — not for general
  local testing (use /unit-testing or /lint-check for that).
---

# CI Debug — Reproduce CI Failures Locally

CI failures are environment failures as much as code failures. This skill spins up an
environment that matches CI (same OS, same Rust toolchain, same env vars) and re-runs the
exact failing job using the underlying skills — so what you see locally is what CI saw.

---

## When to Use

- A push or PR triggered CI and a job failed.
- The failure is not reproducible with a plain `just test` or `just clippy` locally.
- You suspect an environment difference (toolchain version, missing env var, OS-specific behaviour).

For straightforward lint or test failures that reproduce locally without a container, use
`/lint-check` or `/unit-testing` directly — they are faster.

---

## Phase 1 — Read the CI Failure

Before touching anything local:

1. Read `.github/workflows/*.yml` to find the failing job and its steps.
2. Note exactly:
   - The runner OS (`ubuntu-latest`, `macos-latest`, etc.)
   - The Rust toolchain version (`stable`, `1.xx`, `nightly`)
   - Any `env:` variables set at job or step level
   - The exact command that failed (e.g. `cargo clippy`, `cargo test`, `cargo build`)
   - Any setup steps before the failing command (e.g. `rustup component add`, `apt-get install`)

Extract the **minimal reproduction**: the smallest set of commands that, when run in order,
produce the failure.

---

## Phase 2 — Spin Up a Local Environment

### Option A — Docker (preferred for OS-specific failures)

```bash
# Match the CI runner image as closely as possible
docker run --rm -it \
  -v "$(pwd)":/workspace \
  -w /workspace \
  rust:<toolchain-version> \
  bash
```

Inside the container, replicate the CI setup steps, then run the failing command.

Common CI setup steps to replicate:
```bash
rustup component add clippy rustfmt   # if CI adds these
cargo fetch                            # if CI pre-fetches deps
```

### Option B — Subprocess with env isolation (for env var issues)

```bash
env -i HOME="$HOME" PATH="$PATH" RUSTUP_HOME="$RUSTUP_HOME" CARGO_HOME="$CARGO_HOME" \
  bash -c "cd $(pwd) && <failing-command>"
```

Strips your local shell env and runs with only what CI would have, revealing env-var leaks.

### Option C — Toolchain pin (for toolchain version mismatches)

```bash
rustup override set <ci-toolchain-version>
# run the failing command
rustup override unset   # restore when done
```

---

## Phase 3 — Run the Failing Job via Underlying Skills

To run the full CI suite locally exactly as GitHub runs it:

```bash
just ci          # fmt-check + clippy + check + test + docs — mirrors ci.yml exactly
just ci-lint     # fmt-check + clippy + check only (faster)
just ci-test     # test only
```

To reproduce a specific failing job, delegate to the matching skill:

| ci.yml job failing | Local command | Skill |
|--------------------|--------------|-------|
| `fmt` | `just fmt-check` | `/lint-check` |
| `clippy` | `just clippy` | `/lint-check` |
| `check` | `just check` | `/lint-check` |
| `test` | `just test` | `/unit-testing` |
| `docs` | `just docs-check` | — |

Run the skill (or command) inside the container/subprocess from Phase 2.

---

## Phase 4 — Diagnose the Difference

Compare local (outside container) vs. inside container:

| What differs | Likely cause |
|--------------|-------------|
| Passes locally, fails in container | Missing dep, env var, OS lib |
| Fails on specific toolchain version | Rust edition issue, unstable feature, lint added in newer clippy |
| Env var missing | CI sets it, local shell has it implicitly |
| File permissions | Line endings (CRLF vs LF), execute bits |

Output a diagnosis:

```
CI failure diagnosis
────────────────────
Job     : <job name>
Step    : <step name and command>
Cause   : <one-line root cause>
Evidence: <the exact error output from the container run>
Fix     : <what needs to change — code, workflow file, or env>
```

---

## Phase 5 — Fix and Verify

Apply the fix (code change, workflow env var, toolchain pin, etc.), then re-run inside the
container to confirm it passes. Only exit the container environment once the reproduction
is clean.

---

## Done Criteria

- [ ] The failing CI command passes inside the local container/subprocess.
- [ ] The fix is applied and `just check` passes outside the container too.
- [ ] The root cause is documented in the PR description or commit message.

---

## What This Skill Does NOT Cover

- Routine local test failures (no container needed) → `/unit-testing`
- Routine lint failures → `/lint-check`
- Git / PR operations → `/git-workflows`
