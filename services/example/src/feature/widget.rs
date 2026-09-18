//! The widget feature — a trivially simple CRUD resource (id, name,
//! `created_at`) demonstrating this workspace's handler → repository shape,
//! the shared error envelope, garde validation, and keyset pagination.
//!
//! Swap `widget` for your first real feature when you fork this template;
//! everything below is the minimum shape to copy.

mod error;
mod handler;
mod model;
mod repository;

use std::sync::Arc;

use axum::{
  Router,
  routing::{get, post},
};
use sqlx::PgPool;

use self::repository::{PgWidgetRepository, WidgetRepository};

/// Builds this feature's routes over a fresh Postgres-backed repository.
pub(crate) fn router(pool: PgPool) -> Router {
  let repo: Arc<dyn WidgetRepository> = Arc::new(PgWidgetRepository::new(pool));
  Router::new()
    .route(
      "/widgets",
      post(handler::create_widget).get(handler::list_widgets),
    )
    .route(
      "/widgets/{id}",
      get(handler::get_widget).delete(handler::delete_widget),
    )
    .with_state(repo)
}
