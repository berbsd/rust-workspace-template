# Feature: `adapter/http.rs`

The inbound (driving) adapter — wire types plus handlers. Each handler
follows the same shape as `services/example`'s `widget` handlers: extract →
validate (garde, on the DTO) → delegate to `{{Feature}}Service` (plain
values, not axum types) → map the result onto a response. State is the
service, not the repository directly — the repository is one layer further
in, behind `service.rs`.

Include only the handlers for the operation set `00-architecture-decisions.md`
named — a read-only feature has no `create_{{feature}}`/`delete_{{feature}}`
below, and no `Create{{Feature}}Request` type either.

## File

### `services/{{service}}/src/feature/{{feature}}/adapter/http.rs`

```rust
//! HTTP handlers for the {{feature}} resource — the driving adapter.
//!
//! Each follows the same shape: extract → validate → delegate → map the
//! result onto a response. No handler talks to Postgres or holds a
//! repository directly — that stays behind [`super::super::service::{{Feature}}Service`].

use std::sync::Arc;

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
};
use chrono::{DateTime, Utc};
use common_types::{
  BadCursor, DEFAULT_LIMIT, KeysetCursor, MAX_LIMIT, PaginatedResponse, validation_details,
};
use garde::Validate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::{domain::{{Feature}}Row, error::{{Feature}}Error, service::{{Feature}}Service};

/// Shared state every {{feature}} handler runs against.
pub(crate) type Shared{{Feature}}Service = Arc<{{Feature}}Service>;

/// The default keyset cursor for this feature: `(created_at, id)`.
type {{Feature}}Cursor = KeysetCursor<Uuid>;

/// `POST /{{feature_plural}}` request body.
#[derive(Debug, Deserialize, Validate)]
pub(crate) struct Create{{Feature}}Request {
  {{create_request_fields}}
}

/// The wire shape returned for a {{feature}}.
///
/// A distinct type from [`{{Feature}}Row`] rather than deriving both
/// `sqlx::FromRow` and `Serialize` on one struct: a persistence row and a
/// wire schema are different concerns even when, as here, they happen to
/// share every field today — a column added to the table later should not
/// silently become an API field just because nothing marks the boundary.
#[derive(Debug, Serialize)]
pub(crate) struct {{Feature}}Response {
  pub id: Uuid,
  {{response_fields}}
  pub created_at: DateTime<Utc>,
}

impl From<{{Feature}}Row> for {{Feature}}Response {
  fn from({{feature}}: {{Feature}}Row) -> Self {
    Self {
      id: {{feature}}.id,
      {{response_field_mapping}}
      created_at: {{feature}}.created_at,
    }
  }
}

/// `GET /{{feature_plural}}` query parameters.
#[derive(Debug, Deserialize, Validate)]
pub(crate) struct {{Feature}}ListParams {
  /// Page size; defaults to [`DEFAULT_LIMIT`] when absent. An out-of-range
  /// value is a `400`, never silently clamped — see [`MAX_LIMIT`]'s own doc
  /// comment for why a clamped response is worse than an error.
  #[garde(range(min = 1, max = MAX_LIMIT))]
  limit: Option<i64>,
  /// Opaque cursor from a previous page's `next_cursor`. Its format is
  /// checked on decode ([`{{Feature}}Error::decode_cursor`]), not here.
  #[garde(skip)]
  cursor: Option<String>,
}

/// `POST /{{feature_plural}}` — creates a {{feature}} and returns it.
///
/// # Errors
/// [`{{Feature}}Error::Validation`] if the request body fails validation;
/// [`{{Feature}}Error::Database`] if the insert fails.
pub(crate) async fn create_{{feature}}(
  State(service): State<Shared{{Feature}}Service>,
  Json(req): Json<Create{{Feature}}Request>,
) -> Result<(StatusCode, Json<{{Feature}}Response>), {{Feature}}Error> {
  req
    .validate()
    .map_err(|report| {{Feature}}Error::Validation(validation_details(&report)))?;
  let {{feature}} = service.create({{create_call_args_from_req}}).await?;
  Ok((StatusCode::CREATED, Json({{feature}}.into())))
}

/// `GET /{{feature_plural}}/{id}` — fetches one {{feature}}.
///
/// # Errors
/// [`{{Feature}}Error::NotFound`] if no {{feature}} has that id;
/// [`{{Feature}}Error::Database`] if the query fails.
pub(crate) async fn get_{{feature}}(
  State(service): State<Shared{{Feature}}Service>,
  Path(id): Path<Uuid>,
) -> Result<Json<{{Feature}}Response>, {{Feature}}Error> {
  let {{feature}} = service.get(id).await?;
  Ok(Json({{feature}}.into()))
}

/// `GET /{{feature_plural}}` — lists {{feature_plural}} newest-first,
/// paginated by cursor.
///
/// # Errors
/// [`{{Feature}}Error::Validation`] if `limit` is out of range or `cursor`
/// does not decode; [`{{Feature}}Error::Database`] if the query fails.
pub(crate) async fn list_{{feature_plural}}(
  State(service): State<Shared{{Feature}}Service>,
  Query(params): Query<{{Feature}}ListParams>,
) -> Result<Json<PaginatedResponse<{{Feature}}Response>>, {{Feature}}Error> {
  params
    .validate()
    .map_err(|report| {{Feature}}Error::Validation(validation_details(&report)))?;
  let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
  let after = {{Feature}}Error::decode_cursor::<{{Feature}}Cursor>(params.cursor.as_deref())?
    .map(|cursor| (cursor.created_at, cursor.id));

  let rows = service.list(limit + 1, after).await?;
  let page = PaginatedResponse::from_rows(rows, limit, |{{feature}}| {{Feature}}Cursor {
    created_at: {{feature}}.created_at,
    id: {{feature}}.id,
  });

  Ok(Json(PaginatedResponse {
    data: page.data.into_iter().map({{Feature}}Response::from).collect(),
    next_cursor: page.next_cursor,
    has_more: page.has_more,
  }))
}

/// `DELETE /{{feature_plural}}/{id}` — removes a {{feature}}.
///
/// # Errors
/// [`{{Feature}}Error::NotFound`] if no {{feature}} had that id;
/// [`{{Feature}}Error::Database`] if the query fails.
pub(crate) async fn delete_{{feature}}(
  State(service): State<Shared{{Feature}}Service>,
  Path(id): Path<Uuid>,
) -> Result<StatusCode, {{Feature}}Error> {
  service.delete(id).await?;
  Ok(StatusCode::NO_CONTENT)
}
```

`{{Feature}}Service::get`/`delete` already return `{{Feature}}Error::NotFound`
directly (see `06-feature-service.md`) — the handler no longer needs its own
`.ok_or(...)`/`if ... else` branching the way `services/example`'s
`widget::handler` does calling the repository straight; that branching moved
into `service.rs` on purpose, since "does this id exist" is domain logic,
not an HTTP concern.

## Placeholders used

- `{{feature}}` / `{{Feature}}` / `{{feature_plural}}`
- `{{create_request_fields}}` — one `#[garde(...)] pub field: Type,` per
  extra field, with the same validation rules a garde-audit
  (`rust-quality` check #19) would expect
- `{{response_fields}}` / `{{response_field_mapping}}` — mirror
  `{{additional_fields}}` from `05-feature-domain-port.md`
- `{{create_call_args_from_req}}` — the `create` call's arguments, taken
  from `req.<field>` (e.g. `&req.name` or `&req.name, req.quantity`),
  matching `service.rs`'s `create` signature exactly

## Verify

```bash
cargo check -p {{service}}
cargo clippy -p {{service}} --no-deps --all-targets --all-features
```

Both should be clean once `09-feature-error.md` and `10-feature-wiring.md`
land (this file references `{{Feature}}Error::decode_cursor`, which needs
the `BadCursor` impl the next prompt adds). A clippy warning about an unused
`{{Feature}}Row` import means `{{response_field_mapping}}` didn't actually use
every field — fix the mapping, don't suppress the lint.
