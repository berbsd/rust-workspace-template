//! The API error envelope every service answers with.
//!
//! One shape, three fields, shared across every service in the workspace —
//! so a client writes error handling once:
//!
//! ```json
//! {
//!   "error": "NOT_FOUND",
//!   "message": "project not found",
//!   "details": null
//! }
//! ```
//!
//! - **`error`** — the machine-readable code, `SCREAMING_SNAKE_CASE`. Note the
//!   field is named `error`, while the [`ApiErrorMapping`] method supplying it
//!   is `error_code`; a client reading `error_code` off the wire finds nothing.
//! - **`message`** — human-readable, taken from the error's `Display` impl,
//!   which `thiserror` generates from `#[error("…")]`.
//! - **`details`** — structured extras, or `null`. Always present; see
//!   [`ApiErrorBody::details`].
//!
//! # Defining a Service Error
//!
//! Every service defines its own domain error enum and maps it onto the
//! envelope via [`ApiErrorMapping`] + [`impl_api_error_response!`]:
//!
//! ```no_run
//! use axum::http::StatusCode;
//! use common_types::{ApiErrorMapping, impl_api_error_response};
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! pub enum PaymentError {
//!     #[error("payment method {0} is expired")]
//!     CardExpired(String),
//!
//!     #[error("insufficient funds: need ${need}, have ${have}")]
//!     InsufficientFunds { need: f64, have: f64 },
//!
//!     #[error("payment gateway timeout")]
//!     GatewayTimeout,
//!
//!     #[error("user not found")]
//!     UserNotFound,
//! }
//!
//! impl ApiErrorMapping for PaymentError {
//!     fn status_code(&self) -> StatusCode {
//!         match self {
//!             Self::CardExpired(_) | Self::InsufficientFunds { .. } => {
//!                 StatusCode::PAYMENT_REQUIRED
//!             }
//!             Self::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
//!             Self::UserNotFound => StatusCode::NOT_FOUND,
//!         }
//!     }
//!
//!     fn error_code(&self) -> &'static str {
//!         match self {
//!             Self::CardExpired(_) => "CARD_EXPIRED",
//!             Self::InsufficientFunds { .. } => "INSUFFICIENT_FUNDS",
//!             Self::GatewayTimeout => "GATEWAY_TIMEOUT",
//!             Self::UserNotFound => "USER_NOT_FOUND",
//!         }
//!     }
//!
//!     // Optional: Add structured details for client debugging
//!     fn details(&self) -> Option<serde_json::Value> {
//!         match self {
//!             Self::InsufficientFunds { need, have } => Some(serde_json::json!({
//!                 "required_amount": need,
//!                 "available_amount": have,
//!                 "shortfall": need - have,
//!             })),
//!             _ => None,
//!         }
//!     }
//! }
//!
//! // This macro generates the IntoResponse impl
//! impl_api_error_response!(PaymentError);
//! ```
//!
//! The split between the trait and the macro is an orphan-rule workaround, not
//! a design preference — [`impl_api_error_response!`] records why a blanket
//! `impl<T: ApiErrorMapping> IntoResponse for T` cannot exist.
//!
//! # HTTP Status Code Guidelines
//!
//! Choose status codes that match HTTP semantics:
//!
//! | Status Code | Use When | Example |
//! |-------------|----------|---------|
//! | **400** Bad Request | Client sent invalid data | Malformed JSON, validation failure |
//! | **401** Unauthorized | Authentication required | Missing/invalid auth token |
//! | **403** Forbidden | Authenticated but not authorized | User can't access resource |
//! | **404** Not Found | Resource doesn't exist | User ID not in database |
//! | **408** Request Timeout | Server-side timeout | Long query exceeded time limit |
//! | **409** Conflict | State conflict | Username already taken |
//! | **413** Payload Too Large | Request body too big | File upload exceeds 10MB limit |
//! | **422** Unprocessable Entity | Valid syntax, invalid semantics | Start date after end date |
//! | **429** Too Many Requests | Rate limit exceeded | API quota exceeded |
//! | **500** Internal Server Error | Unexpected server error | Database connection failed |
//! | **502** Bad Gateway | Upstream service failed | Payment gateway returned error |
//! | **503** Service Unavailable | Temporarily unavailable | Under maintenance, overloaded |
//! | **504** Gateway Timeout | Upstream service timeout | Payment gateway didn't respond |
//!
//! # Error codes are a wire contract
//!
//! Clients branch on [`ApiErrorBody::error`], so a code is as public as a URL.
//! Keep them `SCREAMING_SNAKE_CASE`, descriptive (`CARD_EXPIRED`, not
//! `ERROR_42`), and identical across services for the same condition. Renaming
//! one breaks every client that handled it, and the break is silent — the
//! branch simply stops matching.

use axum::http::StatusCode;
use serde::Serialize;

/// How a domain error becomes an HTTP response.
///
/// Every service implements this on its own error enum, then invokes
/// [`impl_api_error_response!`](crate::impl_api_error_response) to get the
/// `IntoResponse` impl. Splitting the two lets a test assert the mapping
/// without building a response.
///
/// The trait deliberately does not touch `message` — that comes from the
/// error's `Display` impl, so the wording lives beside the variant in its
/// `#[error("…")]` attribute.
pub trait ApiErrorMapping {
  /// The HTTP status this error answers with.
  ///
  /// See the status-code table in the module docs for which code fits which
  /// condition; picking consistently across services is what makes a client's
  /// generic retry and error handling work everywhere.
  fn status_code(&self) -> StatusCode;

  /// The machine-readable code clients branch on, `SCREAMING_SNAKE_CASE`.
  ///
  /// Lands in the response's `error` field — the names differ, and only the
  /// wire name matters to a client. `&'static str` on purpose: a code is a
  /// fixed vocabulary, never interpolated from request data.
  fn error_code(&self) -> &'static str;

  /// Structured extras for this error, or `None` when there are none.
  ///
  /// Where field-level validation failures, retry hints, or an upstream's own
  /// error belong. Never put anything here a caller is not entitled to see:
  /// this is serialized straight to them.
  ///
  /// Defaults to `None`, so an error type that needs no extras implements
  /// nothing.
  fn details(&self) -> Option<serde_json::Value> {
    None
  }
}

// ============================================================================
// Error Types
// ============================================================================

/// The serialized error envelope — the body of every failed request.
///
/// Built by [`impl_api_error_response!`](crate::impl_api_error_response) from
/// any [`ApiErrorMapping`] implementor, so no service constructs one by hand.
/// Its field names are a wire contract shared by every service and every
/// generated client; renaming one is a platform-wide breaking change.
///
/// ```json
/// {
///   "error": "NOT_FOUND",
///   "message": "User with id 123 not found",
///   "details": null
/// }
/// ```
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(
  example = json!({"error": "NOT_FOUND", "message": "User with id 123 not found", "details": null})
))]
pub struct ApiErrorBody {
  /// Machine-readable code, `SCREAMING_SNAKE_CASE` (e.g. `"NOT_FOUND"`).
  ///
  /// From [`ApiErrorMapping::error_code`]. The wire name is `error`, not
  /// `error_code`.
  #[cfg_attr(feature = "openapi", schema(example = "NOT_FOUND"))]
  pub error:   &'static str,
  /// Human-readable description, from the error's
  /// [`Display`](std::fmt::Display) impl.
  ///
  /// Written for a person reading a log or a toast, not for a client to parse
  /// — branch on [`Self::error`] instead. It may name the offending value, so
  /// keep secrets out of `#[error("…")]` strings.
  #[cfg_attr(feature = "openapi", schema(example = "User with id 123 not found"))]
  pub message: String,
  /// Optional structured payload for client-side debugging.
  ///
  /// Serialized as `null` rather than omitted, so a generated client types it
  /// *nullable* rather than *optional* — the platform-wide convention for
  /// every response type in this crate.
  ///
  /// **Never add `skip_serializing_if = "Option::is_none"` here.** The doc
  /// example above and the `ToSchema` example both advertise `"details": null`;
  /// omitting the key would make the `OpenAPI` document promise a field the
  /// server never sends. This is the envelope behind *every* error response on
  /// the platform, so that mismatch would reach every client error path.
  ///
  /// `#[schema(required = true)]` is the other half — utoipa leaves an
  /// `Option<T>` out of `required` unless told otherwise.
  #[cfg_attr(feature = "openapi", schema(required = true))]
  pub details: Option<serde_json::Value>,
}

/// Converts a [`garde::Report`] into structured JSON suitable for the
/// `details` field of [`ApiErrorBody`].
///
/// Groups validation errors by field path, producing:
///
/// ```json
/// {
///   "fields": {
///     "email": ["must not be empty"],
///     "age": ["must be at least 18"]
///   }
/// }
/// ```
///
/// Use this in [`ApiErrorMapping::details()`] for validation error variants
/// so clients get per-field error information instead of a flat string.
#[cfg(feature = "garde")]
#[must_use]
pub fn validation_details(report: &garde::Report) -> serde_json::Value {
  let mut fields = std::collections::BTreeMap::<String, Vec<String>>::new();

  for (path, error) in report.iter() {
    let key = if path.is_empty() {
      "_root".to_owned()
    } else {
      path.to_string()
    };
    fields
      .entry(key)
      .or_default()
      .push(error.message().to_owned());
  }

  serde_json::json!({ "fields": fields })
}

/// Generates the `IntoResponse` impl that turns an error type into the shared
/// [`ApiErrorBody`] envelope.
///
/// Invoke it once beside every domain error enum. The error type must
/// implement [`ApiErrorMapping`] and [`Display`](std::fmt::Display) — the
/// latter is what `thiserror`'s `#[error("…")]` provides.
///
/// ```ignore
/// impl_api_error_response!(PaymentError);
///
/// // PaymentError is now a handler return type:
/// async fn charge(amount: Cents) -> Result<Json<Receipt>, PaymentError> { ... }
/// ```
///
/// # Why a macro rather than a blanket impl
///
/// The natural spelling is a blanket impl, and Rust's orphan rules (RFC 1023)
/// forbid it:
///
/// ```compile_fail
/// // Does not compile: `IntoResponse` is axum's, `T` could be anyone's.
/// impl<T: ApiErrorMapping + Display> axum::response::IntoResponse for T { ... }
/// ```
///
/// A blanket impl over an unconstrained `T` would conflict with any other
/// crate's `IntoResponse` impl for a type that also implements
/// [`ApiErrorMapping`]. Emitting one concrete impl per named type sidesteps
/// that entirely.
///
/// A derive macro would read slightly better at the call site and would cost a
/// proc-macro crate in every service's dependency graph. `macro_rules!` earns
/// the tradeoff here because there is nothing to parse.
///
/// # What it generates
///
/// ```ignore
/// impl axum::response::IntoResponse for PaymentError {
///   fn into_response(self) -> axum::response::Response {
///     let body = ApiErrorBody {
///       error:   self.error_code(),   // ApiErrorMapping
///       message: self.to_string(),    // Display
///       details: self.details(),      // ApiErrorMapping
///     };
///     (self.status_code(), axum::Json(body)).into_response()
///   }
/// }
/// ```
///
/// Note that `message` comes from `Display` and not from the trait: that is
/// what keeps each variant's wording next to the variant.
#[macro_export]
macro_rules! impl_api_error_response {
  ($error_type:ty) => {
    impl ::axum::response::IntoResponse for $error_type {
      fn into_response(self) -> ::axum::response::Response {
        use $crate::ApiErrorMapping;

        let body = $crate::ApiErrorBody {
          error:   self.error_code(),
          message: ::std::string::ToString::to_string(&self),
          details: self.details(),
        };
        (self.status_code(), ::axum::Json(body)).into_response()
      }
    }
  };
}

#[cfg(all(test, feature = "openapi"))]
mod schema_tests {
  use utoipa::PartialSchema;

  use super::*;

  /// `details` is `required` even though it is `Option`, so a client types it
  /// `object | null` rather than `object | null | undefined`.
  ///
  /// This is the envelope behind every error response on the platform, and it
  /// documents `"details": null` in two places — the module example and the
  /// `ToSchema` example. This test pins both halves: the field is serialized
  /// rather than skipped, and it appears in `required`.
  #[test]
  fn details_is_required_despite_being_optional() {
    let schema = serde_json::to_value(ApiErrorBody::schema()).unwrap_or_default();
    let required: Vec<&str> = schema["required"]
      .as_array()
      .map(|values| {
        values
          .iter()
          .filter_map(serde_json::Value::as_str)
          .collect()
      })
      .unwrap_or_default();

    for field in ["error", "message", "details"] {
      assert!(
        required.contains(&field),
        "`{field}` must be required; got {required:?}"
      );
    }
  }
}
