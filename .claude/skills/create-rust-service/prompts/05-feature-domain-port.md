# Feature: `domain.rs` + `port.rs`

The innermost layer of the feature slice: the entity itself, and the
storage contract the rest of the feature depends on — no axum, no wire
types, no SQL. This is what `docs/specs/2026-09-18-hexagonal-feature-architecture-design.md`
calls the target shape; these two files are its first half.

**Assumes `00-architecture-decisions.md` answered "Postgres" for
persistence and named a real operation set.** If that prompt's answer was
"no persistence," skip `port.rs` entirely (or replace it with whatever real
collaborator the feature depends on) and name `domain.rs`'s struct
`{{Feature}}` — not `{{Feature}}Row` — since it was never a database row.
Everything below assumes the default: Postgres-backed, at least one
non-id/created_at field.

## Files

### 1. `services/{{service}}/src/feature/{{feature}}/domain.rs`

```rust
//! The {{feature}} domain entity.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A {{feature}} as stored — the persistence shape. Never returned
/// directly from a handler; `adapter/http.rs` defines its own
/// `{{Feature}}Response` and converts explicitly (see `08-feature-adapter-http.md`
/// and `SKILL.md`'s "Core Principles").
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct {{Feature}}Row {
  pub id: Uuid,
  {{additional_fields}}
  pub created_at: DateTime<Utc>,
}
```

Replace `{{additional_fields}}` with every field gathered in `SKILL.md` Step
1 (e.g. `pub name: String,` / `pub quantity: i32,` — one per line, plain
`pub` fields, no wire-format derives here). This struct is the persistence
shape; `adapter/http.rs` gets its own response DTO in `08-feature-adapter-http.md`
even when every field happens to match today, so a column added later
doesn't silently become an API field.

### 2. `services/{{service}}/src/feature/{{feature}}/port.rs`

```rust
//! The {{feature}} feature's storage port.
//!
//! [`{{Feature}}Repository`] is a trait rather than a bare
//! `Pg{{Feature}}Repository` so [`super::service::{{Feature}}Service`]
//! depends on the shape of storage, not on Postgres — the seam a unit test
//! exercises with a hand-rolled fake instead of a real database (see
//! `11-tests.md`). `tests/{{feature}}_it.rs` doesn't take that route: per
//! AGENTS.md, a real throwaway database (`#[sqlx::test]`) is preferred over
//! mocking storage in integration tests, so it drives
//! [`super::adapter::postgres::Pg{{Feature}}Repository`] through the real
//! router instead.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::domain::{{Feature}}Row;

/// What the {{feature}} feature needs from storage.
#[async_trait]
pub(crate) trait {{Feature}}Repository: Send + Sync {
  /// Inserts a new {{feature}} and returns the stored row.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the insert fails.
  async fn create(&self, {{create_args}}) -> Result<{{Feature}}Row, sqlx::Error>;

  /// Fetches a {{feature}} by id, or `None` if no row matches.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn get(&self, id: Uuid) -> Result<Option<{{Feature}}Row>, sqlx::Error>;

  /// Lists up to `limit` {{feature_plural}} ordered `(created_at DESC, id
  /// DESC)`, resuming after `after` (the last item's ordering columns) when
  /// given.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn list(
    &self,
    limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
  ) -> Result<Vec<{{Feature}}Row>, sqlx::Error>;

  /// Deletes a {{feature}} by id. Returns whether a row was actually
  /// removed.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
}
```

Replace `{{create_args}}` with one parameter per field gathered in Step 1
(e.g. `name: &str` or `name: &str, quantity: i32`) — the port takes plain
values, never the wire-format request DTO (that conversion happens in
`adapter/http.rs`). Drop or add methods to match the feature's real
operations — CRUD is the default shape `services/example`'s `widget` uses,
not a mandatory contract every feature must implement in full. A read-only
feature has no `create`/`delete`; an append-only one has no `delete`.

## Placeholders used

- `{{feature}}` / `{{Feature}}` / `{{feature_plural}}`
- `{{additional_fields}}` — the entity's fields beyond `id`/`created_at`
- `{{create_args}}` — the `create`'s parameter list, matching those fields

Note `{{Feature}}Row` (not bare `{{Feature}}`) is the struct name — see the
doc comment above and `SKILL.md`'s "Core Principles" for why the suffix is
load-bearing, not decorative.

## Verify

```bash
cargo check -p {{service}} 2>&1 | grep -v "feature::{{feature}}::router\|unresolved import\|cannot find"
```

`domain.rs` and `port.rs` should compile clean on their own (both only
depend on `chrono`/`uuid`/`async-trait`, already in `Cargo.toml`). Any error
*inside* these two files — not about the not-yet-created `service.rs`
depending on them — must be fixed before continuing.
