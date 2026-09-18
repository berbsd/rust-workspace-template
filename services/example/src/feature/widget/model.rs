//! Wire and domain shapes for the widget resource.

use chrono::{DateTime, Utc};
use garde::Validate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A widget as stored.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Widget {
  pub id:         Uuid,
  pub name:       String,
  pub created_at: DateTime<Utc>,
}

/// `POST /widgets` request body.
#[derive(Debug, Deserialize, Validate)]
pub(crate) struct CreateWidgetRequest {
  #[garde(length(min = 1, max = 200))]
  pub name: String,
}

/// The wire shape returned for a widget.
///
/// A distinct type from [`Widget`] rather than deriving both
/// `sqlx::FromRow` and `Serialize` on one struct: a persistence row and a
/// wire schema are different concerns even when, as here, they happen to
/// share every field today — a column added to the table later should not
/// silently become an API field just because nothing marks the boundary.
#[derive(Debug, Serialize)]
pub(crate) struct WidgetResponse {
  pub id:         Uuid,
  pub name:       String,
  pub created_at: DateTime<Utc>,
}

impl From<Widget> for WidgetResponse {
  fn from(widget: Widget) -> Self {
    Self {
      id:         widget.id,
      name:       widget.name,
      created_at: widget.created_at,
    }
  }
}
