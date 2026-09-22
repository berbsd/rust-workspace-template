# `lib.rs` and `main.rs`

The crate root and the binary entrypoint. `lib.rs` owns the module tree and
the crate-level `router(pool)` function a future host can call directly
(mirroring `services/example`); `main.rs` is the thinnest possible wrapper
around `Config::load()` → `module::init()` → `axum::serve`.

## Files

### 1. `services/{{service}}/src/lib.rs`

```rust
//! {{Service}} service — {{description}}.
//!
//! # Architecture
//!
//! Feature-first, hexagonal layout. Each `feature/<f>/` is a vertical slice
//! owning its own domain entity, storage port, application service, and
//! adapters (an HTTP inbound adapter, a Postgres outbound adapter):
//!
//! - **`{{feature}}`** — {{one-line feature description}}.
//!
//! `src/main.rs` loads [`Config`], hands it to [`module::init`], and serves
//! the returned router. A future multi-service host calls [`router`]
//! directly over its own already-connected pool, the same way
//! `hosts/example-host` calls `example::router` today.

mod config;
mod feature;
pub mod module;

pub use config::Config;

use axum::{Router, http::StatusCode, routing::get};
use sqlx::PgPool;

/// Builds the service's router over `pool`: a `/health` check plus every
/// feature's routes. Mount this directly, or go through [`module::init`]
/// for the standalone binary path (connect + migrate + build).
pub fn router(pool: PgPool) -> Router {
  Router::new()
    .route("/health", get(health))
    .merge(feature::{{feature}}::router(pool))
}

/// Liveness check. No dependency checks — a service that can answer HTTP at
/// all is up; readiness (can it reach Postgres) is a separate concern this
/// template doesn't wire up yet.
async fn health() -> StatusCode {
  StatusCode::OK
}
```

Each additional feature slice adds one `.merge(feature::<name>::router(pool.clone()))`
here — see `10-feature-wiring.md` for the first feature and
`13-add-feature.md` for every one after.

### 2. `services/{{service}}/src/main.rs`

```rust
//! {{Service}} service binary entrypoint.
//!
//! Loads [`{{service}}::Config`] and delegates Postgres connection,
//! migration, and router construction to [`{{service}}::module::init`] —
//! this file's only remaining job is starting the listener and waiting for
//! shutdown.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();

  let config = {{service}}::Config::load()?;
  let port = config.port;
  let (_pool, router) = {{service}}::module::init(&config).await?;

  let addr = SocketAddr::from(([0, 0, 0, 0], port));
  let listener = tokio::net::TcpListener::bind(addr).await?;

  tracing::info!(%addr, "{{service}} service listening");
  axum::serve(listener, router)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  Ok(())
}

/// Waits for Ctrl-C (or, on Unix, SIGTERM — what a container orchestrator
/// sends) so `axum::serve` can drain in-flight requests before exiting.
async fn shutdown_signal() {
  let ctrl_c = async {
    // Registration failure here is as rare as the SIGTERM case below and,
    // like it, not worth taking the process down over — but log it instead
    // of silently discarding it, since a failure here means Ctrl-C stops
    // working for graceful shutdown without any signal that it did.
    let _result: Result<(), std::io::Error> = tokio::signal::ctrl_c()
      .await
      .inspect_err(|error| tracing::warn!(%error, "failed to install ctrl-c handler"));
  };

  #[cfg(unix)]
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      | Ok(mut signal) => {
        signal.recv().await;
      },
      | Err(error) => {
        // Falls back to ctrl-c only rather than panicking — the zero-panic
        // policy applies to a process serving live requests, and a failed
        // signal registration is not worth taking the whole service down for.
        tracing::warn!(%error, "failed to install SIGTERM handler; ctrl-c still works");
        std::future::pending::<()>().await;
      },
    }
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    () = ctrl_c => {},
    () = terminate => {},
  }
}
```

`shutdown_signal` is copied verbatim from `services/example/src/main.rs` —
it has no service-specific content and should never drift between services.

## Placeholders used

- `{{Service}}` — Title-case of `{{service}}` for prose (`Note`, `Time
  Entry`) — distinct from `{{Feature}}`, which is the domain type name.
- `{{description}}` — the one-line Cargo.toml description gathered in
  `SKILL.md` Step 1.
- `{{feature}}` — the first feature's snake_case name.

## Verify

`feature`, `config`, and `module` don't exist yet, so this alone does not
compile — that's expected. `mod feature;` and `mod config;` (and
`module::init`'s call into `router`) become satisfiable once
`03-config.md`, `04-module.md`, and `10-feature-wiring.md` land. Don't run
`cargo check` until after those.
