//! Example service — the workspace template's living exemplar.
//!
//! # Architecture
//!
//! A minimal, self-contained Axum service: one `router(pool)` function
//! builds the whole app over a Postgres pool. `src/main.rs` calls it and
//! serves the result directly; `hosts/example-host` calls the same function
//! and nests it under a prefix alongside other services. Neither depends on
//! anything but this crate, `axum`, `sqlx`, and `common-types` — there is no
//! shared service framework in this template to depend on instead.
//!
//! - **`feature/widget`** — a trivially simple CRUD resource (id, name,
//!   `created_at`) demonstrating handler → repository layering, the shared
//!   error envelope, garde validation, and keyset pagination — with no business
//!   logic beyond that.
//!
//! Scaffold a new service by copying this shape and swapping `widget` for
//! your first real feature.

mod feature;

use axum::{Router, http::StatusCode, routing::get};
use sqlx::PgPool;

/// Builds the service's router over `pool`: a `/health` check plus every
/// feature's routes. Mount this directly (see `main.rs`) or nest it under a
/// prefix when composing multiple services into one host (see
/// `hosts/example-host`).
pub fn router(pool: PgPool) -> Router {
  Router::new()
    .route("/health", get(health))
    .merge(feature::widget::router(pool))
}

/// Liveness check. No dependency checks — a service that can answer HTTP at
/// all is up; readiness (can it reach Postgres) is a separate concern this
/// minimal template doesn't wire up.
async fn health() -> StatusCode {
  StatusCode::OK
}
