# aegon Justfile
# Run `just --list` to see all targets.
#
# Quick start:
#   just setup    — install all system and cargo dependencies
#   just run      — launch the live TUI watcher
#   just watch    — alias for run (opens in the current terminal)
#   just demo     — launch in a new macOS Terminal window

# ── Setup and dependencies ──────────────────────────────────────────────────────

# Install all system and Cargo tools required to develop and run Aegon
setup: install-sys-deps install-cargo-tools
    @echo "✓ All dependencies installed — run 'just run' to start Aegon"

# Install system dependencies via Homebrew (macOS)
# Requires: https://brew.sh
install-sys-deps:
    @which brew > /dev/null || (echo "Error: Homebrew not found. Install from https://brew.sh" && exit 1)
    brew install tmux gh just
    @echo "✓ System deps ready (tmux, gh, just)"

# Install Cargo tools used in development and CI
install-cargo-tools:
    cargo install cargo-outdated --locked
    cargo install cargo-audit --locked
    @echo "✓ Cargo tools ready (cargo-outdated, cargo-audit)"

# ── Entry points ────────────────────────────────────────────────────────────────

# Launch the Aegon live TUI watcher in the current terminal
# Watches ~/.claude/projects/**/*.jsonl — start a Claude Code session to see events
run: build
    ./target/debug/aegon

# Alias: same as run
watch: run

# Launch Aegon in a new macOS Terminal window (background — keeps this shell free)
demo: build
    osascript -e 'tell application "Terminal" to do script "{{justfile_directory()}}/target/debug/aegon"'
    @echo "Aegon launched in a new Terminal window"

# Run the JSONL audit script against ~/.claude/projects (requires Python 3)
audit-jsonl:
    python3 examples/jsonl/audit.py



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

# Build docs without opening — mirrors CI: broken intra-doc links are errors
docs-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

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
