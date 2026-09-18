//! Example service binary entrypoint.
//!
//! Connects to Postgres, runs migrations, builds the router via
//! [`example::router`], and serves it. This is the whole of what a
//! standalone service binary in this workspace does — no shared bootstrap
//! crate, just `sqlx`/`axum`/`tokio` directly.

use std::net::SocketAddr;

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
  sqlx::migrate!("./migrations").run(&pool).await?;

  let port: u16 = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8080);
  let addr = SocketAddr::from(([0, 0, 0, 0], port));

  let router = example::router(pool);
  let listener = tokio::net::TcpListener::bind(addr).await?;

  tracing::info!(%addr, "example service listening");
  axum::serve(listener, router)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  Ok(())
}

/// Waits for Ctrl-C (or, on Unix, SIGTERM — what a container orchestrator
/// sends) so `axum::serve` can drain in-flight requests before exiting.
async fn shutdown_signal() {
  let ctrl_c = async {
    let _unused = tokio::signal::ctrl_c().await;
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
