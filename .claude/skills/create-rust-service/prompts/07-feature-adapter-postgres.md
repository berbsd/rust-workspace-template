# Feature: `adapter/postgres.rs` + first migration

The outbound (driven) adapter — the only file that knows this feature is
backed by Postgres. Its shape is a pure port implementation: every method
signature comes straight from `port.rs`, unchanged.

**Only runs if `00-architecture-decisions.md` answered "Postgres" for
persistence.** For "no persistence," this whole prompt is skipped — there
is no `adapter/postgres.rs` and no migration for that feature.

## Files

### 1. `services/{{service}}/migrations/0001_{{feature}}.sql`

```sql
-- {{Service}} service: {{feature_plural}} table.

CREATE SCHEMA IF NOT EXISTS {{service_snake}};

CREATE TABLE IF NOT EXISTS {{service_snake}}.{{feature_plural}} (
    id         UUID PRIMARY KEY,
    {{column_definitions}}
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_{{feature_plural}}_pagination
  ON {{service_snake}}.{{feature_plural}} (created_at DESC, id DESC);
```

Replace `{{column_definitions}}` with one `NOT NULL`/nullable SQL column per
field from `05-feature-domain-port.md`'s `{{additional_fields}}` — types
follow `sqlx`'s Postgres mapping (`String` → `TEXT`, `i32` → `INTEGER`,
`bool` → `BOOLEAN`, `Option<T>` → the same type without `NOT NULL`). Run
`squawk` on this file before moving on (`npx squawk migrations/0001_{{feature}}.sql`
or let the `pre-commit` hook do it at commit time) — this workspace's
`.squawk.toml` lints for exactly the concurrent-index/timeout gotchas that
don't apply here (fresh table, no traffic) but do apply to every migration
after this one.

**Migrations are immutable once applied anywhere** (see AGENTS.md) — this
only matters after the first `just db-ensure && just test` run against this
file. Free to edit right up until then.

### 2. `services/{{service}}/src/feature/{{feature}}/adapter/postgres.rs`

```rust
//! Postgres-backed adapter for the {{feature}} storage port.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::{domain::{{Feature}}Row, port::{{Feature}}Repository};

/// Postgres-backed [`{{Feature}}Repository`].
pub(crate) struct Pg{{Feature}}Repository {
  pool: PgPool,
}

impl Pg{{Feature}}Repository {
  /// Wraps a connection pool as a [`{{Feature}}Repository`].
  #[must_use]
  pub(crate) fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl {{Feature}}Repository for Pg{{Feature}}Repository {
  async fn create(&self, {{create_args}}) -> Result<{{Feature}}Row, sqlx::Error> {
    sqlx::query_as::<_, {{Feature}}Row>(
      "INSERT INTO {{service_snake}}.{{feature_plural}} (id, {{column_list}}) VALUES \
       (gen_random_uuid(), {{bind_placeholders}}) RETURNING id, {{column_list}}, created_at",
    )
    {{bind_calls}}
    .fetch_one(&self.pool)
    .await
  }

  async fn get(&self, id: Uuid) -> Result<Option<{{Feature}}Row>, sqlx::Error> {
    sqlx::query_as::<_, {{Feature}}Row>(
      "SELECT id, {{column_list}}, created_at FROM {{service_snake}}.{{feature_plural}} WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await
  }

  async fn list(
    &self,
    limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
  ) -> Result<Vec<{{Feature}}Row>, sqlx::Error> {
    match after {
      | Some((created_at, id)) => {
        sqlx::query_as::<_, {{Feature}}Row>(
          "SELECT id, {{column_list}}, created_at FROM {{service_snake}}.{{feature_plural}} \
           WHERE (created_at, id) < ($1, $2) ORDER BY created_at DESC, id DESC LIMIT $3",
        )
        .bind(created_at)
        .bind(id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
      },
      | None => {
        sqlx::query_as::<_, {{Feature}}Row>(
          "SELECT id, {{column_list}}, created_at FROM {{service_snake}}.{{feature_plural}} \
           ORDER BY created_at DESC, id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
      },
    }
  }

  async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM {{service_snake}}.{{feature_plural}} WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;
    Ok(result.rows_affected() > 0)
  }
}
```

Replace `{{column_list}}` (comma-separated column names, e.g. `name` or
`name, quantity`), `{{bind_placeholders}}` (`$2` / `$2, $3` — `$1` is always
the generated `id`), and `{{bind_calls}}` (one `.bind({{arg}})` line per
`create` parameter, in the same order as `{{column_list}}`) consistently
with `port.rs`'s `create` signature and the migration's column list — these
three must stay in lockstep or the insert silently binds the wrong value to
the wrong column.

## Placeholders used

- `{{service}}` / `{{service_snake}}` / `{{feature}}` / `{{Feature}}` /
  `{{feature_plural}}`
- `{{column_definitions}}` (SQL), `{{column_list}}` / `{{bind_placeholders}}`
  / `{{bind_calls}}` (Rust) — all derived from the same field list as
  `05-feature-domain-port.md`'s `{{additional_fields}}`/`{{create_args}}`

## Verify

```bash
cargo check -p {{service}}
```

Compiles up through `feature/{{feature}}/adapter/postgres.rs` in isolation —
`adapter.rs` (the `mod http; mod postgres;` file) doesn't exist until
`10-feature-wiring.md`, so `cargo check` still reports the feature module as
unresolved. The query strings themselves aren't checked here (no
`sqlx::query!` compile-time verification against a live database in this
template) — `11-tests.md`'s integration test is what actually proves the SQL
is correct.
