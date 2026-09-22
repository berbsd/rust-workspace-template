# `module.rs`

The composition root: connects Postgres, runs migrations, and builds the
router, given an already-loaded [`Config`]. Kept separate from `main.rs` so
the connect-and-migrate sequence is reusable — the seam a future
multi-service host would call instead of duplicating it, and the boundary
that keeps `main.rs` free of anything but "load config, wire, serve."

## File

### `services/{{service}}/src/module.rs`

```rust
//! Composition root for the {{service}} domain.
//!
//! [`init`] connects Postgres (running migrations) and builds the router —
//! the one entry point `main.rs` calls for the standalone binary path. A
//! future multi-service host would call [`crate::router`] directly instead,
//! over its own already-connected, already-migrated pool.

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::Config;

/// Boxed error for module wiring.
pub type InitError = Box<dyn std::error::Error>;

/// Wires the {{service}} domain: connects Postgres (running migrations),
/// then builds the router over the resulting pool.
///
/// Returns the pool alongside the router rather than dropping it — a
/// background task or a future readiness probe needs a handle to it, even
/// though `main.rs` itself only consumes the router.
///
/// # Errors
/// Returns an error if the database connection or migrations fail.
pub async fn init(config: &Config) -> Result<(PgPool, axum::Router), InitError> {
  let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&config.database_url)
    .await?;
  sqlx::migrate!("./migrations").run(&pool).await?;

  let router = crate::router(pool.clone());
  Ok((pool, router))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
  use super::*;

  /// `init` against an unreachable database fails fast rather than hanging
  /// or panicking — the zero-panic policy applies to composition-root code
  /// too, and a bad `DATABASE_URL` is an operator mistake, not a crash.
  #[tokio::test]
  async fn init_surfaces_connection_failure() {
    let config = Config {
      database_url: "postgres://invalid:invalid@127.0.0.1:1/invalid".to_owned(),
      port: 8080,
    };
    assert!(init(&config).await.is_err());
  }
}
```

## Placeholders used

- `{{service}}` — prose only (the doc comment).

## Verify

```bash
cargo check -p {{service}}
```

Still fails — `crate::router` needs `feature::{{feature}}::router` to exist,
which lands at `10-feature-wiring.md`. From here through
`09-feature-error.md`, `cargo check -p {{service}}` will report unresolved
`feature` module errors; that's expected until wiring lands. Each individual
file should still be free of *its own* syntax/type errors — read the
compiler output for errors inside the file you just wrote versus errors
about the not-yet-created `feature` module, and only worry about the former
before moving on.
