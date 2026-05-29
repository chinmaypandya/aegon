---
name: clean-code
description: >
  Rust design principles skill. Applies Rust-idiomatic SOLID equivalents and feature-driven
  module organisation when writing or reviewing code. Use when starting a new feature, designing
  a module, reviewing code structure, or when asked "how should I structure this", "is this
  idiomatic", or "clean this up".
---

# Clean Code — Rust Principles + Feature-Driven Design

Use this skill when writing new features or reviewing existing structure. It defines how code
should be shaped before it is written — not a post-hoc cleanup checklist.

---

## Rust-Idiomatic SOLID

### S — Single Purpose Types and Modules

Each struct, enum, and module does one thing. If you need to describe a type with "and", split it.

```rust
// bad: one struct doing two jobs
struct UserManager { user: User, db_conn: DbConn, email_client: EmailClient }

// good: each type owns one concern
struct UserRepo { db: DbConn }
struct Notifier { email: EmailClient }
```

Keep modules flat and named after what they own, not what they do (`user.rs` not `manager.rs`).

---

### O — Extend via Traits, Not Modification

Define behaviour as traits. New behaviour = new `impl`, not editing existing types.

```rust
pub trait Summarise {
    fn summary(&self) -> String;
}

// adding a new format: impl a new type, don't touch existing ones
struct JsonSummary;
impl Summarise for JsonSummary { ... }
```

Use sealed traits (trait + private supertrait) when you own all implementors and want to prevent
external extension.

---

### L — Consistent Trait Contracts

If a function accepts `impl Trait`, every implementor must honour the full contract — not just
compile. Document invariants in the trait definition, not the impl.

```rust
pub trait Store {
    /// Must return Err if key is empty. Never panics.
    fn get(&self, key: &str) -> Result<Value, StoreError>;
}
```

Panicking in a trait impl is an LSP violation. Use `Result` or `Option` instead.

---

### I — Small, Focused Traits

Prefer many small traits over one large one. Callers only depend on what they use.

```rust
// bad: one fat trait
pub trait UserService { fn create(&self, ..); fn delete(&self, ..); fn send_email(&self, ..); }

// good: separated concerns
pub trait CreateUser { fn create(&self, ..) -> Result<User, Error>; }
pub trait NotifyUser { fn notify(&self, user: &User) -> Result<(), Error>; }
```

If a trait has more than ~4 methods, ask whether it's really one concern.

---

### D — Depend on Traits, Not Concrete Types

Functions and structs take `impl Trait` or generic `T: Trait`, not concrete types. This keeps
logic testable and decoupled from infrastructure.

```rust
// bad: hardwired to one impl
fn process(repo: &PostgresRepo) { ... }

// good: works with any impl
fn process<R: UserRepo>(repo: &R) { ... }
// or with dynamic dispatch when needed:
fn process(repo: &dyn UserRepo) { ... }
```

---

## Rust-Specific Ergonomics

### Ownership as a Design Signal

If you're fighting the borrow checker, the design is likely wrong — not the borrow checker.
Common signals:

| Fighting the checker | Design fix |
|----------------------|-----------|
| Passing `&mut` everywhere | Centralise mutation in one owner |
| Cloning to escape lifetimes | Restructure so the owner outlives the borrower |
| `Rc<RefCell<T>>` proliferating | Reconsider shared mutable state; prefer message passing |

### Newtype for Type Safety

Wrap primitives when identity matters:

```rust
struct UserId(u64);
struct OrderId(u64);
// now you can't pass an OrderId where a UserId is expected
```

### Error Types per Module

Each module defines its own error type with `thiserror`. Never leak internal error types across
module boundaries — convert at the boundary with `map_err` or `From`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("user {0} not found")]
    NotFound(UserId),
    #[error("invalid email: {0}")]
    InvalidEmail(String),
}
```

Use `anyhow` only in binaries and top-level application code, never in library crates.

### Builder Pattern for Complex Construction

Use the builder pattern (or `Default` + setters) when a struct has more than ~3 optional fields.
Avoid constructors with long positional argument lists.

---

## Feature-Driven Module Structure

Organise by **feature/domain**, not by technical layer.

```
// bad: layered
src/
  models/user.rs
  handlers/user.rs
  services/user.rs

// good: feature-driven
src/
  user/
    mod.rs       ← public API surface only
    types.rs     ← User, UserId, UserError
    repo.rs      ← UserRepo trait + impls
    service.rs   ← business logic
  order/
    ...
```

Rules:
- `mod.rs` (or `feature/mod.rs`) re-exports only what external code needs — keep the public
  surface minimal.
- Each feature owns its types, traits, errors, and logic. No cross-feature imports except
  through the public API.
- Shared primitives (IDs, common errors, utilities) live in a `common` or `core` module.
- Infrastructure (DB, HTTP, config) lives outside feature modules and is injected via traits.

---

## Feature Development Checklist

Before marking a feature done, verify:

- [ ] Types are named after domain concepts, not implementation details.
- [ ] No `unwrap()` or `expect()` in library code — only in tests or truly-unreachable paths.
- [ ] Errors are typed (`thiserror`), not stringly-typed.
- [ ] Public API surface is the minimum needed — everything else is `pub(crate)` or private.
- [ ] The feature compiles with `cargo check --workspace`.
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`.

After the checklist: hand off to `/unit-testing` for test coverage.

---

## Documentation

Everything public must be documented. Everything non-obvious must be documented even if private.
Documentation is not a summary of what the code does — the code already shows that. It explains
**why it exists, what problem it solves, and what the author intended** so a reader never has to
guess at intent.

### What to write

**Modules** — explain the purpose of the module and how it fits into the feature. A reader
landing here for the first time should immediately understand what this module owns and why it
exists as its own unit.

```rust
//! Handles user identity resolution — maps raw session tokens to verified `UserId` values.
//! Sits between the HTTP layer and the rest of the domain; nothing downstream deals with tokens.
```

**Public types and traits** — explain the concept the type represents, not its fields. Why does
this type exist? What invariant does it uphold?

```rust
/// A verified, deduplicated user identity. Only constructible via [`UserRepo::resolve`] —
/// callers can trust that any `UserId` in scope refers to a real, active account.
pub struct UserId(u64);
```

**Public functions and methods** — lead with the purpose (what it achieves for the caller), then
note any non-obvious constraints, error conditions, or side effects. Do not restate the signature.

```rust
/// Resolves a session token to its owner, refreshing the token TTL on success.
///
/// Returns `Err(UserError::Expired)` if the token is valid but past its TTL.
/// Returns `Err(UserError::NotFound)` if the token has never been issued.
pub fn resolve(&self, token: &Token) -> Result<UserId, UserError> {}
```

**Non-obvious private code** — a single line is enough. If you had to think for more than a few
seconds about why something is written the way it is, future readers will too.

```rust
// Drain before drop: the channel sender must outlive the background thread or we deadlock.
drop(self.sender.take());
self.handle.join().ok();
```

### What NOT to write

- Don't restate the name: `/// Returns the user id` on `fn user_id()` adds nothing.
- Don't document the obvious: `/// The user's email address` on a field named `email` is noise.
- Don't write stale intent: if the code changed but the doc didn't, the doc is a lie. Update both
  together or delete the doc.

### Documentation checklist (part of Feature Development Checklist)

- [ ] Every `pub` module has a `//!` doc comment explaining its role in the feature.
- [ ] Every `pub` type, trait, and function has a `///` doc comment stating its purpose.
- [ ] Non-obvious private code has an inline `//` comment explaining the why.
- [ ] No doc comment merely restates the name or signature.
- [ ] `cargo doc --no-deps` builds without warnings.

---

## What This Skill Does NOT Cover

- Writing tests → `/unit-testing`
- Formatting and lint enforcement → `/lint-check`
- Git workflow → `/git-workflows`
