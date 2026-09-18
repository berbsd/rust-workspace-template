//! Example host binary — demonstrates composing multiple services' routers
//! into one deployable process.
//!
//! A **host** mounts one or more services' routers into a single binary to
//! save on always-warm compute cost. Every service also builds as its own
//! standalone binary (`cargo run -p example`); composition buys nothing
//! functional — this file is wiring only, no domain logic.
//!
//! # Mounting a second service
//!
//! Today this host mounts exactly one domain, `services/example`, under
//! `/example`. Mounting a second — say a hypothetical `services/catalog` —
//! is:
//!
//! 1. **`Cargo.toml`** — depend on the new service crate (`catalog = {
//!    workspace = true }`).
//! 2. **`main`** — build its own pool the same way `example`'s is built below
//!    (each service owns its schema; nothing stops them sharing one physical
//!    database if that's simpler for a small deployment), call
//!    `catalog::router(pool)`, and `.nest("/catalog", ...)` it alongside
//!    `example`'s.
//!
//! No step touches `example`'s own crate — a host composes services, it
//! never reaches into one.

use std::net::SocketAddr;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();

  let database_url =
    std::env::var("DATABASE_URL").map_err(|_env_var_error| "DATABASE_URL must be set")?;
  let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
  sqlx::migrate!("../../services/example/migrations")
    .run(&pool)
    .await?;

  let router = Router::new().nest("/example", example::router(pool));

  let port: u16 = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8080);
  let addr = SocketAddr::from(([0, 0, 0, 0], port));
  let listener = tokio::net::TcpListener::bind(addr).await?;

  tracing::info!(%addr, "example-host listening");
  axum::serve(listener, router).await?;

  Ok(())
}
