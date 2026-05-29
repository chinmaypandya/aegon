---
name: code-workflow
description: >
  Git + just workflow orchestrator. Covers branching, commit conventions, when to call
  sub-skills (clean-code, unit-testing, lint-check, ci-debug), push, PR strategies, and
  merge patterns. Use when starting work, committing, opening a PR, merging, or cleaning up.
---

# Git Workflow — Orchestrator

This skill owns the **shape of development**: how to start, structure, validate, and ship work.
It does not run tests, lint, or write code itself — it delegates to the right sub-skill at the
right moment.

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
branch → /clean-code → write features → /unit-testing → /lint-check → commit → push → PR → CI → merge
                                                                                              ↓ (CI fails)
                                                                                         /ci-debug
```

---

## Stage 1 — Start a Branch

```bash
just branch <type>/<slug>
```

Syncs `main` with `--ff-only` (fails loudly on unexpected local commits), then creates the branch.

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

**Before writing any code:** invoke `/clean-code` to agree on module structure and type design
for the feature. This avoids structural rework after tests are written.

---

## Stage 2 — Write Features

Follow `/clean-code` principles. Write the batch of features for this branch before moving on.
Do not interleave writing + testing + committing — complete the feature batch first.

---

## Stage 3 — Test the Batch

Once features are written, invoke `/unit-testing`. It diffs against `main`, writes tests for
all changed public items in one pass, runs only the affected crates, and iterates until green.

Do not commit before tests pass.

---

## Stage 4 — Lint

After tests are green, invoke `/lint-check`:

```bash
just fmt && just clippy && just check
```

Fix all warnings. Do not commit with clippy warnings.

---

## Stage 5 — Commit

Commit atomically — one logical change per commit:

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
- `scope` is the crate or module affected (optional but recommended).
- Breaking changes: `feat(api)!: rename endpoint`

Multiple commits per branch are fine — one per logical unit of work.

---

## Stage 6 — Push

```bash
just push        # first push — sets upstream
just sync        # subsequent pushes — rebases on main + force-pushes with lease
```

Use `just sync` to stay current with `main` throughout the branch lifetime, not just at PR time.

---

## Stage 7 — Open a PR

```bash
just pr          # interactive: gh prompts for title + body
just pr-draft    # same, marks as draft
```

PR description must include:
- What changed and why (not just what — that's in the diff).
- Any breaking changes.
- Test plan (what `/unit-testing` covered + what CI will run).

---

## Stage 8 — CI

CI runs `cargo fmt --check`, `cargo clippy`, and `cargo test` on push/PR. You do not need to
run the full suite locally — that is CI's job.

If CI fails:
- Lint/clippy failure → re-run `/lint-check` locally, fix, push.
- Test failure → re-run `/unit-testing` on the failing crate, fix, push.
- Failure not reproducible locally → invoke `/ci-debug` to replicate the CI environment.

---

## Stage 9 — Merge

Squash merge (preferred for feature branches — keeps `main` history clean):

```bash
just merge       # squash-merges current PR + deletes remote branch
```

For `release/` branches: merge commit is acceptable to preserve history.

---

## Stage 10 — Cleanup

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
| `just fix` | `cargo fix` |

---

## Sub-Skills Reference

| Skill | When to call |
|-------|-------------|
| `/clean-code` | Before writing features — agree on structure |
| `/unit-testing` | After features are written — batch test the diff |
| `/lint-check` | After tests pass — before committing |
| `/ci-debug` | When CI fails and it's not reproducible locally |

---

## Common Gotchas

- **`just branch` fails:** local `main` has commits not on remote — investigate before proceeding.
- **`just sync` refuses:** remote has commits you haven't fetched — `git fetch` first.
- **Amending:** `git commit --amend --no-edit` then `just sync` to force-push safely.
- **Mid-work stash:** `git stash push -m "wip: ..."` and `git stash pop` — fine to run directly.
