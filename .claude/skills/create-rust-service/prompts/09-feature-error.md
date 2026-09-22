# Feature: `error.rs`

The feature's error vocabulary — shared between `service.rs` (which returns
it) and `adapter/http.rs` (which is the only place that turns it into an
HTTP response). Stays at the feature root rather than inside `adapter/`: it
is not an adapter-specific concern, it's what the whole feature can fail
with. Follows `rust-quality` check #13's contract exactly:
`thiserror` + `ApiErrorMapping` + `impl_api_error_response!`, no leakage of
the underlying `sqlx::Error` into `Display`.

## File

### `services/{{service}}/src/feature/{{feature}}/error.rs`

```rust
//! The {{feature}} feature's own error type, mapped onto the shared
//! [`common_types::ApiErrorBody`] envelope.

use axum::http::StatusCode;
use common_types::{ApiErrorMapping, BadCursor, impl_api_error_response};

/// Everything that can go wrong handling a {{feature}} request.
#[derive(Debug, thiserror::Error)]
pub(crate) enum {{Feature}}Error {
  /// No {{feature}} exists with the requested id.
  #[error("{{feature}} not found")]
  NotFound,
  /// The request body or query parameters failed validation, or a domain
  /// rule in `service.rs` rejected the input. Carries structured per-field
  /// details for [`ApiErrorMapping::details`].
  #[error("validation failed")]
  Validation(serde_json::Value),
  /// The database rejected or failed to serve the query. The `Display`
  /// text is what reaches the client via `ApiErrorBody::message` — never
  /// the raw `sqlx::Error`, which includes table/column names and query
  /// fragments.
  #[error("an internal error occurred")]
  Database(#[from] sqlx::Error),
}

impl ApiErrorMapping for {{Feature}}Error {
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

impl BadCursor for {{Feature}}Error {
  fn bad_cursor(message: String) -> Self {
    Self::Validation(serde_json::json!({ "cursor": [message] }))
  }
}

impl_api_error_response!({{Feature}}Error);
```

No placeholders beyond `{{feature}}`/`{{Feature}}` — this file is a template
in the literal sense; every generated feature's `error.rs` should read
identically apart from the name, the same way `rust-quality` check #13
expects every service's error enums to share one shape. If the feature's
domain rules need a second `Validation`-shaped variant (rare — most features
have exactly one validation bucket, differentiated by the `serde_json::Value`
payload's field keys, not by a second enum variant), add it here and update
both `match` arms in `ApiErrorMapping` together — never let a new variant
silently fall through the exhaustive match to fail the build instead of
being deliberately mapped.

## Placeholders used

- `{{feature}}` / `{{Feature}}`

## Verify

```bash
cargo check -p {{service}}
```

`error.rs` depends only on `common-types`/`axum`/`thiserror`/`serde_json`,
all already in `Cargo.toml` from `01-crate-scaffold.md` — this file should
compile clean in isolation the moment it's written, independent of whether
`service.rs`/`adapter/` exist yet.
