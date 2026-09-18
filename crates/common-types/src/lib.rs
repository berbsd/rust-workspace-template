//! The workspace's shared wire vocabulary.
//!
//! Every service serializes types defined here, so this crate is API
//! surface, not merely a utility library: renaming a field or changing an
//! optionality convention here changes the generated client for every
//! service at once.
//!
//! Deliberately minimal — two things every HTTP service needs, and nothing
//! else. No auth types, no typed ids, no validation framework: those are
//! real, useful patterns, but they belong to a project's own choices about
//! identity and access control, not to a starter template.
//!
//! What it owns:
//!
//! - **The API error envelope** — [`ApiErrorBody`], produced from any domain
//!   error via [`ApiErrorMapping`] + [`impl_api_error_response!`].
//! - **Pagination** — [`PaginatedResponse`] and the keyset [`Cursor`] trait.
//!
//! # The error envelope
//!
//! Every error response is the same three-field object. The machine-readable
//! code travels in a field named **`error`** — not `error_code`, which is the
//! name of the [`ApiErrorMapping`] *method* that supplies it:
//!
//! ```json
//! { "error": "NOT_FOUND", "message": "widget not found", "details": null }
//! ```
//!
//! # Optional response fields are nullable, never omitted
//!
//! An `Option` field on a response type serializes as `null` and is marked
//! `required` in the emitted schema (when the `openapi` feature is enabled).
//! A generated client then types it *nullable* (`T | null`) rather than
//! *optional* (`T | null | undefined`).
//!
//! **Do not add `skip_serializing_if = "Option::is_none"` to a response type
//! in this crate, and do not drop a `#[schema(required = true)]`.** Both
//! halves are needed and neither is visible from the other: omitting the key
//! breaks the wire, while utoipa leaves an `Option<T>` out of `required`
//! regardless of how it serializes. [`ApiErrorBody::details`] carries the
//! full reasoning and a `schema_tests` guard.
//!
//! # Features
//!
//! | Feature | Enables |
//! |---|---|
//! | `garde` | [`validation_details`], converting a garde report into the error envelope's `details` field |
//! | `openapi` | `utoipa` schema derives on both wire types |

// Tests index fixture collections and parsed payloads directly; an out-of-range
// index is the intended failure signal there, not a panic to defend against.
#![cfg_attr(test, allow(clippy::indexing_slicing))]

/// Fails the build if an `OpenAPI` `info(version = ...)` literal drifts from
/// the crate's `CARGO_PKG_VERSION`.
///
/// The version is restated as a literal in the `#[openapi(info(...))]`
/// block, so this guard pins it to the package version. Place it next to the
/// `ApiDoc`:
///
/// ```ignore
/// common_types::assert_openapi_version!("0.1.0");
/// // ...
/// #[openapi(info(version = "0.1.0", ...))]
/// ```
#[macro_export]
macro_rules! assert_openapi_version {
  ($lit:literal) => {
    const _: () = {
      const fn bytes_eq(
        a: &[u8],
        b: &[u8],
      ) -> bool {
        match (a.split_first(), b.split_first()) {
          | (None, None) => true,
          | (Some((x, ra)), Some((y, rb))) if *x == *y => bytes_eq(ra, rb),
          | _ => false,
        }
      }
      assert!(
        bytes_eq($lit.as_bytes(), env!("CARGO_PKG_VERSION").as_bytes()),
        "OpenAPI info version literal drifted from CARGO_PKG_VERSION"
      );
    };
  };
}

mod errors;
mod pagination;

#[cfg(feature = "garde")]
pub use errors::validation_details;
pub use errors::{ApiErrorBody, ApiErrorMapping};
pub use pagination::{
  BadCursor, Cursor, CursorError, DEFAULT_LIMIT, KeysetCursor, MAX_LIMIT, PaginatedResponse,
  PriorityKeysetCursor,
};
