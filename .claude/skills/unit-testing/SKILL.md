---
name: unit-testing
description: >
  Diff-aware, batched test agent. After N features are written, invoke this skill once.
  It inspects what changed since the last test run (or a given base ref), identifies all
  affected public functions/modules, writes tests for the whole batch, runs only the affected
  tests (not the full suite), and iterates until green. Use when asked to "test what I just
  wrote", "run tests for these features", "batch test my changes", or "write tests for the diff".
---

# Unit Testing — Skill Playbook

This skill triggers the `unit-testing` agent, which discovers what changed, writes all tests
in one batch, runs only the affected crates, and iterates until green. It distinguishes test
bugs from production bugs and reports the latter without touching production code.

---

## When to Invoke

**Do:** after finishing a batch of features (3–10 functions/modules worth of new code).
**Don't:** after every single edit (too noisy) or blindly on the full workspace (too slow).

The sweet spot: "I wrote these features, now write tests for all of them and iterate until stable."

---

## Position in Workflow

```
Features written
       │
       ▼
/code-reviewer  ←──── (loop back if Major/Critical findings)
       │
       ▼ (APPROVE)
/unit-testing (this skill → unit-testing agent)
       │
       ├─ Tests green ──► /lint-check ──► /documentation ──► commit
       │
       └─ Bug in prod code ──► report to implementer → fix → re-run
```

---

## Role Boundary

- **Test agent:** owns test discovery, writing, and the fix loop.
- **Code implementer:** owns production code. The agent flags bugs but never fixes them.

---

## Handoff

The `unit-testing` agent runs all phases. When it completes:
- Tests green → proceed to `/lint-check`, then `/documentation`, then commit.
- Bug reported → implementer fixes production code → re-invoke this skill.
- After 5 failed iterations → halt and review with user.

---

## What This Skill Does NOT Cover

- Architecture and design review → `/code-reviewer`
- Lint and format → `/lint-check`
- CI environment failures → `/ci-debug`
- Git workflow → `/git-workflows`
