//! The widget feature's own error type, mapped onto the shared
//! [`common_types::ApiErrorBody`] envelope.

use axum::http::StatusCode;
use common_types::{ApiErrorMapping, BadCursor, impl_api_error_response};

/// Everything that can go wrong handling a widget request.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WidgetError {
  /// No widget exists with the requested id.
  #[error("widget not found")]
  NotFound,
  /// The request body or query parameters failed validation. Carries
  /// structured per-field details for [`ApiErrorMapping::details`].
  #[error("validation failed")]
  Validation(serde_json::Value),
  /// The database rejected or failed to serve the query. The Display text
  /// is what reaches the client via `ApiErrorBody::message` — never the raw
  /// `sqlx::Error`, which includes table/column names and query fragments.
  #[error("an internal error occurred")]
  Database(#[from] sqlx::Error),
}

impl ApiErrorMapping for WidgetError {
  fn status_code(&self) -> StatusCode {
    match self {
      | Self::NotFound => StatusCode::NOT_FOUND,
      | Self::Validation(_) => StatusCode::BAD_REQUEST,
      | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn error_code(&self) -> &'static str {
    match self {
      | Self::NotFound => "NOT_FOUND",
      | Self::Validation(_) => "VALIDATION_ERROR",
      | Self::Database(_) => "INTERNAL_ERROR",
    }
  }

  fn details(&self) -> Option<serde_json::Value> {
    match self {
      | Self::Validation(details) => Some(details.clone()),
      | Self::NotFound | Self::Database(_) => None,
    }
  }
}

impl BadCursor for WidgetError {
  fn bad_cursor(message: String) -> Self {
    Self::Validation(serde_json::json!({ "cursor": [message] }))
  }
}

impl_api_error_response!(WidgetError);
