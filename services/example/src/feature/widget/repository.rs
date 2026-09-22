//! Storage for widgets.
//!
//! [`WidgetRepository`] is a trait rather than a bare `PgWidgetRepository`
//! so a handler depends on the shape of storage, not on Postgres — the
//! seam that would let a unit test substitute an in-memory fake. This
//! template's own tests don't take that route: per the workspace
//! `AGENTS.md`, a real throwaway database (`#[sqlx::test]`) is preferred
//! over mocking storage, so `tests/widget_it.rs` drives
//! [`PgWidgetRepository`] directly through the real router. Keep the trait
//! anyway — it costs nothing today and is exactly what a future test that
//! genuinely wants to isolate handler logic from Postgres would need.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::model::Widget;

/// What the widget feature needs from storage.
#[async_trait]
pub(crate) trait WidgetRepository: Send + Sync {
  /// Inserts a new widget and returns the stored row.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the insert fails.
  async fn create(
    &self,
    name: &str,
  ) -> Result<Widget, sqlx::Error>;

  /// Fetches a widget by id, or `None` if no row matches.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn get(
    &self,
    id: Uuid,
  ) -> Result<Option<Widget>, sqlx::Error>;

  /// Lists up to `limit` widgets ordered `(created_at DESC, id DESC)`,
  /// resuming after `after` (the last item's ordering columns) when given.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn list(
    &self,
    limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
  ) -> Result<Vec<Widget>, sqlx::Error>;

  /// Deletes a widget by id. Returns whether a row was actually removed.
  ///
  /// # Errors
  /// Returns [`sqlx::Error`] if the query fails.
  async fn delete(
    &self,
    id: Uuid,
  ) -> Result<bool, sqlx::Error>;
}

/// Postgres-backed [`WidgetRepository`].
pub(crate) struct PgWidgetRepository {
  pool: PgPool,
}

impl PgWidgetRepository {
  /// Wraps a connection pool as a [`WidgetRepository`].
  #[must_use]
  pub(crate) fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl WidgetRepository for PgWidgetRepository {
  async fn create(
    &self,
    name: &str,
  ) -> Result<Widget, sqlx::Error> {
    sqlx::query_as::<_, Widget>(
      "INSERT INTO example.widgets (id, name) VALUES (gen_random_uuid(), $1) RETURNING id, name, \
       created_at",
    )
    .bind(name)
    .fetch_one(&self.pool)
    .await
  }

  async fn get(
    &self,
    id: Uuid,
  ) -> Result<Option<Widget>, sqlx::Error> {
    sqlx::query_as::<_, Widget>("SELECT id, name, created_at FROM example.widgets WHERE id = $1")
      .bind(id)
      .fetch_optional(&self.pool)
      .await
  }

  async fn list(
    &self,
    limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
  ) -> Result<Vec<Widget>, sqlx::Error> {
    match after {
      | Some((created_at, id)) => {
        sqlx::query_as::<_, Widget>(
          "SELECT id, name, created_at FROM example.widgets WHERE (created_at, id) < ($1, $2) \
           ORDER BY created_at DESC, id DESC LIMIT $3",
        )
        .bind(created_at)
        .bind(id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
      },
      | None => {
        sqlx::query_as::<_, Widget>(
          "SELECT id, name, created_at FROM example.widgets ORDER BY created_at DESC, id DESC \
           LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
      },
    }
  }

  async fn delete(
    &self,
    id: Uuid,
  ) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM example.widgets WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;
    Ok(result.rows_affected() > 0)
  }
}
