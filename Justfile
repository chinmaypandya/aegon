# aegon Justfile
# Run `just --list` to see all targets.

# ── Git workflow ────────────────────────────────────────────────────────────────

# Start a new branch from latest main: just branch feat/my-feature
branch name:
    git switch main
    git pull --ff-only origin main
    git switch -c {{name}}

# Stage all tracked changes and commit: just commit "feat(x): message"
commit msg:
    git add -u
    git commit -m "{{msg}}"

# Stage everything including untracked files and commit (use for initial setup only)
commit-all msg:
    git add -A
    git commit -m "{{msg}}"

# Push current branch and set upstream
push:
    git push -u origin HEAD

# Rebase branch on latest main and force-push safely
sync:
    git fetch origin
    git rebase origin/main
    git push --force-with-lease

# Open a PR (interactive — gh will prompt for title/body)
pr:
    gh pr create --assignee @me

# Open a draft PR
pr-draft:
    gh pr create --assignee @me --draft

# Squash-merge the current PR and delete the remote branch
merge:
    gh pr merge --squash --delete-branch

# Full post-merge cleanup: just cleanup feat/my-feature
cleanup branch:
    git switch main
    git pull --ff-only origin main
    git branch -d {{branch}}
    git remote prune origin

# Show current PR status
pr-status:
    gh pr status

# List open PRs
pr-list:
    gh pr list

# View current PR in browser
pr-view:
    gh pr view --web

# ── Rust: core checks ───────────────────────────────────────────────────────────

# Compile without producing binaries (fast feedback)
check:
    cargo check --workspace --all-targets

# Full build
build:
    cargo build --workspace

# Release build
build-release:
    cargo build --workspace --release

# Format all code
fmt:
    cargo fmt --all

# Check formatting without applying (used by CI)
fmt-check:
    cargo fmt --all -- --check

# Run clippy (all warnings as errors)
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Apply machine-applicable clippy and compiler suggestions
fix:
    cargo fix --allow-dirty --allow-staged --workspace

# ── Rust: testing ───────────────────────────────────────────────────────────────

# Run the full test suite
test:
    cargo test --workspace

# Run tests for a specific crate: just test-crate aegon-core
test-crate crate:
    cargo test -p {{crate}}

# Run tests matching a filter: just test-filter "parser"
test-filter filter:
    cargo test --workspace {{filter}}

# Run tests with output visible (useful for debugging)
test-verbose:
    cargo test --workspace -- --nocapture

# ── Rust: docs ──────────────────────────────────────────────────────────────────

# Build and open docs locally
docs:
    cargo doc --workspace --no-deps --open

# Build docs without opening (used by CI)
docs-check:
    cargo doc --workspace --no-deps

# ── CI: local mirror ────────────────────────────────────────────────────────────

# Run exactly what CI runs — use this before pushing to catch failures early
ci: fmt-check clippy check test docs-check
    @echo "✓ All CI checks passed"

# Run only the lint subset of CI (faster)
ci-lint: fmt-check clippy check

# Run only the test subset of CI
ci-test: test

# ── Misc ────────────────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Show outdated dependencies
outdated:
    cargo outdated --workspace

# Audit dependencies for known vulnerabilities
audit:
    cargo audit
