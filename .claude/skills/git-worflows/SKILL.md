---
name: code-workflow
description: >
  Git + just workflow orchestrator. Covers branching, commit conventions, when to call
  sub-skills and agents (clean-code, code-reviewer, unit-testing, lint-check, ci-debug,
  documentation), push, PR strategies, and merge patterns. Use when starting work,
  committing, opening a PR, merging, or cleaning up.
---

# Git Workflow — Orchestrator

This skill owns the **shape of development**: how to start, structure, validate, and ship work.
It does not run tests, lint, or write code itself — it delegates to the right agent or sub-skill
at the right moment.

---

## Hard Rules

- **All workflow commands go through `just`.** Never run `gh` or raw `git push/pull/switch`
  for workflow steps.
- **Never commit to `main`** — always branch.
- **`--force-with-lease` only** — never bare `--force`.
- **Rebase to sync**, **squash merge** at PR time.

---

## The Full Development Loop

```
branch
  │
  ▼
/clean-code  (design types, modules, traits before writing)
  │
  ▼
write features
  │
  ▼
/code-reviewer  ◄─────────────────────────────────────┐
  │                                                    │
  ├─ REQUEST CHANGES ──────────────────────────────────┘
  │
  ▼ APPROVE
/unit-testing
  │
  ├─ Bug in prod code ──► fix ──► /code-reviewer ──► /unit-testing
  │
  ▼ Tests green
/lint-check
  │
  ▼
/documentation  ◄── docs are part of the commit, not a follow-up
  │
  ▼
commit → push → PR
  │
  ▼
CI
  │
  ├─ Lint/test fail (local) ──► /lint-check or /unit-testing
  ├─ Fail not reproducible  ──► /ci-debug
  │
  ▼ CI green
merge → cleanup
```

---

## Stage 1 — Start a Branch

```bash
just branch <type>/<slug>
```

Syncs `main` with `--ff-only`, then creates the branch.

**Branch naming:**

| Type | When |
|------|------|
| `feat/` | New feature |
| `fix/` | Bug fix |
| `chore/` | Maintenance, deps, tooling |
| `refactor/` | Restructure, no behaviour change |
| `docs/` | Documentation only |
| `test/` | Tests only |
| `release/` | Release prep |

**Before writing any code:** invoke `/clean-code` to agree on module structure and type design.
This avoids structural rework after review and tests are written.

---

## Stage 2 — Write Features

Follow `/clean-code` principles. Write the full batch of features before moving on.
Do not interleave writing + reviewing + testing + committing.

---

## Stage 3 — Code Review

Invoke `/code-reviewer`. It audits architecture, DRY, trait design, crate layer violations,
and documentation completeness.

- **APPROVE** → proceed to unit-testing.
- **REQUEST CHANGES** → fix findings, re-invoke `/code-reviewer`.

Do not proceed to testing until the reviewer approves.

---

## Stage 4 — Test the Batch

Invoke `/unit-testing`. It diffs against `main`, writes tests in one pass, runs only affected
crates, and iterates until green.

Do not commit before tests pass.

---

## Stage 5 — Lint

Invoke `/lint-check`:

```bash
just ci-lint    # fmt-check + clippy + check — mirrors CI exactly
```

Fix all warnings. Do not commit with clippy warnings.

---

## Stage 6 — Documentation

Invoke `/documentation` **before committing**. Documentation is part of the changeset — not
a follow-up. Reviewers see the changelog entry in the PR diff. The merge commit on `main`
contains both the feature and its record.

The agent audits the diff and updates:
- `CHANGELOG.md` — user-facing changes under `[Unreleased]`
- `JOURNAL.md` — achievements, caveats, next steps
- `README.md` — any changed public surface
- `CLAUDE.md` — any structural or workflow changes
- `.claude/` files — consistency with current code

Do not commit until the documentation agent reports all files updated.

---

## Stage 7 — Commit

```bash
just commit "<type>(<scope>): <description>"
```

**Conventional commit format:**

```
<type>(<scope>): <short description, imperative, ≤72 chars>

[optional body]

[optional footer: BREAKING CHANGE, closes #123]
```

- `type` must match the branch type.
- `scope` is the crate or module affected.
- Breaking changes: `feat(api)!: rename endpoint`

---

## Stage 8 — Push and PR

```bash
just push        # first push — sets upstream
just sync        # subsequent — rebases on main + force-pushes with lease
just pr          # open PR (gh prompts for title + body)
just pr-draft    # same, marks as draft
```

PR description must include: what changed and why, breaking changes, test plan.

---

## Stage 9 — CI

CI runs `cargo fmt --check`, `cargo clippy`, `cargo test` on push/PR.

If CI fails:
- Lint/clippy → re-run `/lint-check`, fix, push.
- Test → re-run `/unit-testing` on failing crate, fix, push.
- Not reproducible locally → invoke `/ci-debug`.

---

## Stage 10 — Merge

```bash
just merge       # squash-merges current PR + deletes remote branch
```

For `release/` branches: merge commit is acceptable.

---

## Stage 11 — Cleanup

```bash
just cleanup <branch-name>
```

Switches to `main`, fast-forward pulls, deletes local branch, prunes stale remote refs.

---

## All Justfile Recipes

| Recipe | What it does |
|--------|-------------|
| `just branch <name>` | Create branch from latest main |
| `just commit "<msg>"` | Stage tracked files and commit |
| `just push` | Push branch and set upstream |
| `just sync` | Rebase on main + force-push safely |
| `just pr` | Open PR interactively |
| `just pr-draft` | Open draft PR |
| `just merge` | Squash-merge current PR |
| `just cleanup <branch>` | Post-merge local cleanup |
| `just pr-status` | Show current PR status |
| `just pr-list` | List open PRs |
| `just pr-view` | Open PR in browser |
| `just build` | `cargo build --workspace` |
| `just check` | `cargo check --workspace` |
| `just test` | `cargo test --workspace` |
| `just fmt` | `cargo fmt --all` |
| `just clippy` | `cargo clippy --workspace -D warnings` |
| `just ci-lint` | fmt-check + clippy + check (mirrors CI) |
| `just fix` | `cargo fix` |

---

## Agent and Skill Reference

| Agent / Skill | When to call |
|---------------|-------------|
| `/clean-code` | Before writing — agree on structure |
| `/code-reviewer` | After writing — architecture + quality gate |
| `/unit-testing` | After review approves — batch test the diff |
| `/lint-check` | After tests pass — before documentation |
| `/documentation` | After lint passes — before committing (docs ship with the feature) |
| `/ci-debug` | CI fails and not reproducible locally |

---

## Common Gotchas

- **`just branch` fails:** local `main` has commits not on remote — investigate before proceeding.
- **`just sync` refuses:** remote has commits you haven't fetched — `git fetch` first.
- **Amending:** `git commit --amend --no-edit` then `just sync`.
- **Mid-work stash:** `git stash push -m "wip: ..."` then `git stash pop`.
