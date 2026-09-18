//! HTTP handlers for the widget resource.
//!
//! Each follows the same shape: extract → validate → delegate to the
//! repository → map the result onto a response. No handler talks to
//! Postgres directly — that stays in `repository.rs`.

use std::sync::Arc;

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
};
use common_types::{
  BadCursor, DEFAULT_LIMIT, KeysetCursor, MAX_LIMIT, PaginatedResponse, validation_details,
};
use garde::Validate;
use serde::Deserialize;
use uuid::Uuid;

use super::{
  error::WidgetError,
  model::{CreateWidgetRequest, WidgetResponse},
  repository::WidgetRepository,
};

/// Shared state every widget handler runs against.
type SharedRepo = Arc<dyn WidgetRepository>;

/// The default keyset cursor for this feature: `(created_at, id)`.
type WidgetCursor = KeysetCursor<Uuid>;

/// `POST /widgets` — creates a widget and returns it.
///
/// # Errors
/// [`WidgetError::Validation`] if `name` is empty or over 200 characters;
/// [`WidgetError::Database`] if the insert fails.
pub(crate) async fn create_widget(
  State(repo): State<SharedRepo>,
  Json(req): Json<CreateWidgetRequest>,
) -> Result<(StatusCode, Json<WidgetResponse>), WidgetError> {
  req
    .validate()
    .map_err(|report| WidgetError::Validation(validation_details(&report)))?;
  let widget = repo.create(&req.name).await?;
  Ok((StatusCode::CREATED, Json(widget.into())))
}

/// `GET /widgets/{id}` — fetches one widget.
///
/// # Errors
/// [`WidgetError::NotFound`] if no widget has that id;
/// [`WidgetError::Database`] if the query fails.
pub(crate) async fn get_widget(
  State(repo): State<SharedRepo>,
  Path(id): Path<Uuid>,
) -> Result<Json<WidgetResponse>, WidgetError> {
  let widget = repo.get(id).await?.ok_or(WidgetError::NotFound)?;
  Ok(Json(widget.into()))
}

/// `GET /widgets` query parameters.
#[derive(Debug, Deserialize, Validate)]
pub(crate) struct WidgetListParams {
  /// Page size; defaults to [`DEFAULT_LIMIT`] when absent. An out-of-range
  /// value is a `400`, never silently clamped — see [`MAX_LIMIT`]'s own doc
  /// comment for why a clamped response is worse than an error.
  #[garde(range(min = 1, max = MAX_LIMIT))]
  limit:  Option<i64>,
  /// Opaque cursor from a previous page's `next_cursor`. Its format is
  /// checked on decode ([`WidgetError::decode_cursor`]), not here.
  #[garde(skip)]
  cursor: Option<String>,
}

/// `GET /widgets` — lists widgets newest-first, paginated by cursor.
///
/// # Errors
/// [`WidgetError::Validation`] if `limit` is out of range or `cursor` does
/// not decode; [`WidgetError::Database`] if the query fails.
pub(crate) async fn list_widgets(
  State(repo): State<SharedRepo>,
  Query(params): Query<WidgetListParams>,
) -> Result<Json<PaginatedResponse<WidgetResponse>>, WidgetError> {
  params
    .validate()
    .map_err(|report| WidgetError::Validation(validation_details(&report)))?;
  let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
  let after = WidgetError::decode_cursor::<WidgetCursor>(params.cursor.as_deref())?
    .map(|cursor| (cursor.created_at, cursor.id));

  let rows = repo.list(limit + 1, after).await?;
  let page = PaginatedResponse::from_rows(rows, limit, |widget| WidgetCursor {
    created_at: widget.created_at,
    id:         widget.id,
  });

  Ok(Json(PaginatedResponse {
    data:        page.data.into_iter().map(WidgetResponse::from).collect(),
    next_cursor: page.next_cursor,
    has_more:    page.has_more,
  }))
}

/// `DELETE /widgets/{id}` — removes a widget.
///
/// # Errors
/// [`WidgetError::NotFound`] if no widget had that id;
/// [`WidgetError::Database`] if the query fails.
pub(crate) async fn delete_widget(
  State(repo): State<SharedRepo>,
  Path(id): Path<Uuid>,
) -> Result<StatusCode, WidgetError> {
  if repo.delete(id).await? {
    Ok(StatusCode::NO_CONTENT)
  } else {
    Err(WidgetError::NotFound)
  }
}
