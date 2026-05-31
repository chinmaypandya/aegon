---
name: ci-debug
description: >
  Reproduces a failing CI job locally inside a subprocess or Docker container that mirrors
  the CI environment. Use when CI fails and you need to understand why — not for general
  local testing (use /unit-testing or /lint-check for that).
---

# CI Debug — Skill Playbook

This skill triggers the `ci-debug` agent, which independently reads CI config, spins up
an isolated environment, re-runs the exact failing command, diagnoses root cause, and
applies a fix.

---

## When to Invoke

- A push or PR triggered CI and a job failed.
- The failure is **not** reproducible with plain `just test` or `just clippy` locally.
- You suspect an environment difference: toolchain version, missing env var, OS-specific behaviour.

For straightforward failures that reproduce locally, go directly to `/lint-check` or
`/unit-testing` — faster and no container needed.

---

## Position in Workflow

```
CI fails on push/PR
       │
       ├─ Reproduces locally? ──► /lint-check or /unit-testing
       │
       └─ Does NOT reproduce? ──► /ci-debug (this skill → ci-debug agent)
                                        │
                                        ▼
                                  Diagnosis + fix
                                        │
                                        ▼
                                  just push → CI re-run
```

---

## Handoff

The `ci-debug` agent owns all phases of reproduction and diagnosis. When it completes:
- If the fix is applied: CI re-run should pass.
- If it reported a production bug: implementer resolves it, then push again.
- If it could not reproduce: check with the user before escalating.

---

## What This Skill Does NOT Cover

- Routine local test failures → `/unit-testing`
- Routine lint failures → `/lint-check`
- Git / PR operations → `/git-workflows`
