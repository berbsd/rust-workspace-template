//! Keyset (cursor) pagination for list endpoints.
//!
//! Every paginated response in this workspace is a [`PaginatedResponse`],
//! and every one resumes from an opaque [`Cursor`] rather than an offset.
//!
//! # Why keyset and not `OFFSET`
//!
//! `OFFSET n` makes the database walk and discard `n` rows, so page 100 costs
//! a hundred times page 1. A keyset predicate compares against indexed columns
//! and jumps straight to the position, so every page costs the same.
//!
//! It is also the only form that stays correct under concurrent writes. With
//! `OFFSET`, a row inserted before the current position shifts every later row
//! down one, and the client silently re-reads a row it already saw — or misses
//! one, if a row was deleted. A cursor names a *position in the ordering*, not
//! a count, so inserts and deletes elsewhere cannot shift it.
//!
//! # How a page is built
//!
//! 1. Order by a key that is **unique overall** — a timestamp alone is not, and
//!    two rows sharing one are an unstable boundary a cursor cannot describe.
//!    The convention here is `(created_at DESC, id DESC)`, where the id breaks
//!    the tie.
//! 2. Ask for `limit + 1` rows. The extra row is the `has_more` signal, and it
//!    costs one row rather than a second `COUNT(*)` query over the whole set.
//! 3. Drop it, then encode the ordering columns of the **last returned** row as
//!    the next cursor.
//!
//! [`PaginatedResponse::from_rows`] does all three; a repository that fetched
//! `limit + 1` rows should hand them straight to it.
//!
//! # Resume with a row-value comparison
//!
//! For an all-descending key the predicate is a single row comparison:
//!
//! ```sql
//! WHERE (created_at, id) < ($1, $2)
//! ORDER BY created_at DESC, id DESC
//! ```
//!
//! Not the hand-expanded equivalent
//! (`created_at < $1 OR (created_at = $1 AND id < $2)`), for three reasons:
//!
//! - **It matches a composite index.** Postgres recognises a row comparison as
//!   a range over `(created_at, id)` and starts the scan at the cursor. The
//!   `OR` form is a disjunction; the planner typically turns it into a bitmap
//!   `OR` of two scans, or falls back to filtering.
//! - **It cannot be written subtly wrong.** The `OR` form has a well-known
//!   failure mode — `created_at <= $1 AND id < $2`, which drops every row whose
//!   timestamp differs from the cursor's. A row comparison has no room for that
//!   mistake.
//! - **It stays one expression as the key grows.** A third column doubles the
//!   `OR` form and adds one item to the row comparison.
//!
//! The form only works while every column sorts the same direction. Mixed
//! directions have no row-comparison spelling, which is why
//! [`PriorityKeysetCursor`] is documented as requiring the expansion —
//! see its docs for the shape.
//!
//! # Cursors are positions, never permissions
//!
//! A cursor is base64url-encoded JSON: readable with one command, and
//! rewritable by any client, since nothing signs it. Encode ordering columns
//! and nothing else. A resolved access decision cached in a cursor becomes an
//! authorization bypass the moment someone edits the query parameter, so every
//! page must re-derive the caller's access from their token.
//!
//! Base64 rather than a hash because a cursor must be *decodable* — the server
//! reads the position back out of it — and because it keeps the value opaque to
//! clients while staying inspectable during debugging.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

// ============================================================================
// Pagination
// ============================================================================

/// Page size applied when a client omits `limit`.
///
/// One value across the workspace, so a client that never sends `limit` gets
/// the same page size from every endpoint. A feature that diverges documents
/// why at the point of divergence.
pub const DEFAULT_LIMIT: i64 = 25;

/// Largest page a client may request.
///
/// A request above the cap is a `400` naming the cap — **never** silently
/// clamped. A clamped response is indistinguishable from a complete one, so a
/// client that asked for 500 and received 50 has no way to learn it is missing
/// rows.
pub const MAX_LIMIT: i64 = 50;

/// The envelope every paginated list endpoint returns.
///
/// One page of `T`, plus what a client needs to ask for the next one:
///
/// ```json
/// {
///   "data": [...],
///   "next_cursor": "eyJ0IjoiMjAyNC0wMS0xNVQxMDozMDowMFoiLCJpIjoiMDE5NDJiZmQifQ",
///   "has_more": true
/// }
/// ```
///
/// `next_cursor` is a [`Cursor`] — base64url-encoded JSON holding the ordering
/// columns of the last item, e.g.
/// `{"created_at":"2024-01-15T10:30:00Z","id":"01942bfd"}`. Clients treat it as
/// opaque and echo it back unchanged.
///
/// `has_more` and a non-null `next_cursor` always agree; a client may branch on
/// either. Both are present because `has_more` reads more directly at a call
/// site that only wants to know whether to show a "load more" control.
///
/// Build one with [`Self::from_rows`] from a `limit + 1` query rather than
/// assembling the fields by hand — that is where the extra-row and cursor logic
/// lives.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginatedResponse<T> {
  /// This page's items, in the endpoint's sort order. Empty on an empty
  /// result — never `null`.
  pub data:        Vec<T>,
  /// Opaque cursor for fetching the next page (`None` if no more pages).
  ///
  /// This is a base64-encoded representation of the last item's ordering
  /// columns. Pass this as the `cursor` parameter in the next request to
  /// fetch the next page.
  ///
  /// Serialized as `null` on the last page rather than omitted, and marked
  /// `required` so a generated client types it *nullable* rather than
  /// *optional*. Both halves are load-bearing — dropping
  /// `skip_serializing_if` alone does nothing to the schema; utoipa still
  /// omits an `Option<T>` from `required` unless told otherwise.
  ///
  /// This is the envelope every paginated response in this workspace wraps,
  /// so it's the single widest lever on client-side optionality if it drifts.
  #[cfg_attr(feature = "openapi", schema(required = true))]
  #[cfg_attr(
    feature = "openapi",
    schema(example = "eyJ0IjoiMjAyNC0wMS0xNVQxMDozMDowMFoiLCJpIjoiMDE5NDJiZmQifQ")
  )]
  pub next_cursor: Option<String>,
  /// Whether a further page exists. Always the same answer as
  /// `next_cursor.is_some()`.
  pub has_more:    bool,
}

impl<T> PaginatedResponse<T> {
  /// Assembles a page from parts already computed by the caller.
  ///
  /// Keeping `next_cursor` and `has_more` consistent is the caller's job here.
  /// Prefer [`Self::from_rows`], which derives both from the row set and cannot
  /// disagree with itself.
  #[must_use]
  pub fn new(
    data: Vec<T>,
    next_cursor: Option<String>,
    has_more: bool,
  ) -> Self {
    Self { data, next_cursor, has_more }
  }

  /// Builds a page from the rows of a `LIMIT limit + 1` query.
  ///
  /// The canonical constructor. Drops the extra row if it came back, sets
  /// `has_more` accordingly, and mints the cursor from the last **remaining**
  /// row via `cursor_fn` — the last row the client actually receives, which is
  /// where the next page must resume.
  ///
  /// A negative or nonsensical `limit` degrades to zero rather than panicking,
  /// which yields an empty page and `has_more` set whenever any row was
  /// fetched.
  #[must_use]
  pub fn from_rows<C: Cursor>(
    mut rows: Vec<T>,
    limit: i64,
    cursor_fn: impl Fn(&T) -> C,
  ) -> Self {
    let limit_usize = usize::try_from(limit).unwrap_or(0);
    let has_more = rows.len() > limit_usize;
    if has_more {
      rows.pop();
    }

    let next_cursor = if has_more {
      rows.last().map(|r| cursor_fn(r).encode())
    } else {
      None
    };

    Self { data: rows, next_cursor, has_more }
  }

  /// Returns a page with no items and nothing more to fetch.
  ///
  /// For endpoints that can answer without querying — an access gate that
  /// resolves to an empty set, or a filter that excludes everything. `200` with
  /// this body, never a `404`: "no matching rows" is a successful answer.
  #[must_use]
  pub fn empty() -> Self {
    Self {
      data:        Vec::new(),
      next_cursor: None,
      has_more:    false,
    }
  }
}

/// A decoded pagination position: the ordering columns of the last row of a
/// page.
///
/// A marker trait — [`encode`](Self::encode) and [`decode`](Self::decode) are
/// provided, so an implementation is `impl Cursor for MyCursor {}` over a
/// `Serialize + Deserialize` struct. Most list endpoints need no bespoke type
/// at all and use [`KeysetCursor`].
///
/// # One cursor type per `ORDER BY`
///
/// The fields must be exactly the ordering columns, in the same order. That is
/// not a style rule: the cursor's only job is to name a position in *this*
/// ordering, so a cursor whose fields disagree with the `ORDER BY` describes a
/// position in a sequence the query never produces, and the page it resumes
/// skips or repeats rows.
///
/// Two endpoints ordering differently therefore need two cursor types, even
/// over the same table. Reusing one is not caught by the compiler, because both
/// decode from the same base64 string — the mismatch shows up as missing rows.
///
/// A cursor minted under one ordering and replayed against another fails to
/// decode into the new shape and surfaces as a `400`, which is the intended
/// outcome: the position is meaningless, and silently restarting from the top
/// would be worse. See [`BadCursor::decode_cursor`].
///
/// # Example
///
/// ```ignore
/// // Query: ORDER BY score DESC, id DESC
/// #[derive(Serialize, Deserialize)]
/// struct SearchCursor {
///   score: f32,
///   id:    String,
/// }
/// impl Cursor for SearchCursor {}
///
/// // Resume: WHERE (score, id) < ($1, $2) ORDER BY score DESC, id DESC
/// let after = MyError::decode_cursor::<SearchCursor>(params.cursor.as_deref())?;
/// ```
pub trait Cursor: Serialize + DeserializeOwned + Sized {
  /// Encodes this position as the opaque string a client sends back.
  ///
  /// base64url without padding, so the value drops into a query parameter
  /// unescaped. Serialization cannot realistically fail for a cursor — these
  /// are flat structs of primitives — and an empty string on failure would
  /// decode as a `400` on the next request rather than resuming somewhere
  /// wrong.
  fn encode(&self) -> String {
    let json = serde_json::to_string(self).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(json.as_bytes())
  }

  /// Decodes a position from a client-supplied cursor string.
  ///
  /// Services rarely call this directly — [`BadCursor::decode_cursor`] wraps it
  /// with the mapping onto the service's own `400`.
  ///
  /// # Errors
  ///
  /// - [`CursorError::InvalidFormat`] — not valid base64url.
  /// - [`CursorError::InvalidData`] — decodes, but not into `Self`. Usually a
  ///   cursor minted for a different ordering; see the trait docs.
  fn decode(cursor: &str) -> Result<Self, CursorError> {
    let bytes = URL_SAFE_NO_PAD
      .decode(cursor)
      .map_err(|e| CursorError::InvalidFormat(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|e| CursorError::InvalidData(e.to_string()))
  }
}

/// The default cursor: `ORDER BY created_at DESC, id DESC`.
///
/// Covers nearly every list endpoint — reach for this before defining a
/// cursor type of your own. The `id` is not decoration: `created_at` alone
/// is not unique, and a tie at the page boundary is a position a cursor
/// cannot name.
///
/// Both columns sort the same direction, so the resume predicate is the single
/// row comparison `WHERE (created_at, id) < ($1, $2)`. See the module docs for
/// why that form rather than the `OR`-expanded one.
///
/// Generic over the id type only; the wire shape is
/// `{"created_at": "…", "id": "…"}` whichever it is, so switching a
/// `KeysetCursor<Uuid>` to a `KeysetCursor<String>` changes nothing a client
/// can observe.
///
/// # Example
///
/// ```
/// use chrono::Utc;
/// use common_types::{Cursor, KeysetCursor};
/// use uuid::Uuid;
///
/// type WidgetCursor = KeysetCursor<Uuid>;
/// let cursor = WidgetCursor { created_at: Utc::now(), id: Uuid::now_v7() };
/// let encoded = cursor.encode();
/// let decoded = WidgetCursor::decode(&encoded).expect("round-trip");
/// assert_eq!(cursor.id, decoded.id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysetCursor<Id> {
  /// Timestamp of the last item on the previous page.
  pub created_at: DateTime<Utc>,
  /// Identifier of the last item on the previous page.
  pub id:         Id,
}

impl<Id> Cursor for KeysetCursor<Id> where Id: Serialize + DeserializeOwned + Sized {}

/// Cursor for priority-first listings:
/// `ORDER BY priority ASC, created_at DESC, id DESC`.
///
/// The one place the row-comparison rule above does not apply. `priority`
/// sorts ascending while the other two sort descending, and a row comparison
/// can only express one direction for the whole tuple — so this cursor
/// **must** be expanded by hand:
///
/// ```sql
/// WHERE priority > $1
///    OR (priority = $1 AND (created_at, id) < ($2, $3))
/// ```
///
/// Write it exactly that way. The inner comparison stays a row comparison
/// because `created_at` and `id` do agree in direction; expanding that half as
/// well reintroduces the `created_at <= $2 AND id < $3` mistake for no benefit.
///
/// Generic over the typed id, so clients still see one opaque token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityKeysetCursor<Id> {
  /// Priority of the last item on the previous page (1..=5, lower first).
  pub priority:   i16,
  /// Timestamp of the last item on the previous page.
  pub created_at: DateTime<Utc>,
  /// Identifier of the last item on the previous page.
  pub id:         Id,
}

impl<Id> Cursor for PriorityKeysetCursor<Id> where Id: Serialize + DeserializeOwned + Sized {}

/// Why a client-supplied cursor could not be read.
///
/// Always a `400`: the value came from the request. Services map it through
/// [`BadCursor::bad_cursor`] rather than matching on it, since the two variants
/// call for the same response.
#[derive(Debug, Error)]
pub enum CursorError {
  /// Not valid base64url — typically a truncated or hand-edited value.
  #[error("invalid cursor format: {0}")]
  InvalidFormat(String),
  /// Valid base64url, but the JSON inside is not this cursor's shape. Most
  /// often a cursor minted under a different ordering.
  #[error("invalid cursor data: {0}")]
  InvalidData(String),
}

/// The seam that lets one cursor-decoding helper serve every service.
///
/// Each service owns its own error enum, so a shared free function cannot
/// return one. Implementing this trait names the service's existing `400`
/// variant once, and [`Self::decode_cursor`] then works everywhere.
///
/// Implement it against the `400` the service already has rather than adding
/// a variant: adopting the helper should change no wire behaviour.
pub trait BadCursor: Sized {
  /// Builds this service's `400` for a cursor that would not decode.
  ///
  /// `message` already reads as a complete sentence; wrap it rather than
  /// reformatting, so the wording stays identical across services.
  fn bad_cursor(message: String) -> Self;

  /// Decodes an optional cursor, mapping failure onto this service's `400`.
  ///
  /// The single definition of how a bad cursor is reported. Every list endpoint
  /// calls this rather than hand-rolling the decode, so the message
  /// (`"invalid cursor: {e}"`), the status, and the layer it happens at are the
  /// same everywhere. Hand-rolled versions drift: dropping the underlying error
  /// loses the only clue to *why* a client's cursor failed.
  ///
  /// A cursor minted for a different ordering decodes into the wrong shape and
  /// arrives here as a `400`. That is intended, not a fallback — the position
  /// is meaningless once the ordering changed, and restarting from the top
  /// silently would skip or repeat rows.
  ///
  /// An associated function rather than a free one because `?` inserts a `From`
  /// conversion, which leaves a free `decode_cursor(raw)?` with no way to infer
  /// its error type. Naming the error — `NoteError::decode_cursor(raw)?` — is
  /// what makes one definition usable from every service.
  ///
  /// # Errors
  ///
  /// [`Self::bad_cursor`] when `raw` is present and does not decode into `C`.
  /// `None` is not an error: it means "start at the beginning".
  fn decode_cursor<C: Cursor>(raw: Option<&str>) -> Result<Option<C>, Self> {
    raw
      .map(C::decode)
      .transpose()
      .map_err(|e| Self::bad_cursor(format!("invalid cursor: {e}")))
  }

  /// [`Self::decode_cursor`] for a cursor already known to be present.
  ///
  /// Separate rather than reached through the optional form, because the
  /// callers that need it have *unwrapped* the `Option` to get here — a sort
  /// key parsed out of the cursor's envelope, or an arm of a match on which
  /// ordering it encodes. Handing them `Option` back so they can immediately
  /// `expect` it would trade a compile-time fact for a runtime one.
  ///
  /// # Errors
  ///
  /// [`Self::bad_cursor`] when `raw` does not decode into `C`.
  fn decode_present_cursor<C: Cursor>(raw: &str) -> Result<C, Self> {
    C::decode(raw).map_err(|e| Self::bad_cursor(format!("invalid cursor: {e}")))
  }
}

#[cfg(all(test, feature = "openapi"))]
mod schema_tests {
  use utoipa::PartialSchema;

  use super::*;

  /// `next_cursor` is `required` even though it is `Option`, so a generated
  /// client types it `string | null` rather than `string | null | undefined`.
  ///
  /// This envelope wraps every paginated response in this workspace, which
  /// makes it the single widest lever on client optionality — and the easiest
  /// place for the `required` half to be dropped unnoticed, since the field
  /// serializes correctly either way.
  #[test]
  fn next_cursor_is_required_despite_being_optional() {
    let schema = serde_json::to_value(PaginatedResponse::<String>::schema()).unwrap_or_default();
    let required: Vec<&str> = schema["required"]
      .as_array()
      .map(|values| {
        values
          .iter()
          .filter_map(serde_json::Value::as_str)
          .collect()
      })
      .unwrap_or_default();

    for field in ["data", "next_cursor", "has_more"] {
      assert!(
        required.contains(&field),
        "`{field}` must be required; got {required:?}"
      );
    }
  }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::cast_possible_wrap)]
mod tests {
  use serde::Deserialize;

  use super::*;

  // Cursor pagination tests

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  struct TestCursor {
    timestamp: i64,
    id:        String,
  }

  impl Cursor for TestCursor {}

  #[test]
  fn cursor_encode_decode_roundtrip() {
    let cursor = TestCursor {
      timestamp: 1_705_312_200,
      id:        "abc123".to_string(),
    };
    let encoded = cursor.encode();
    let decoded = TestCursor::decode(&encoded).unwrap();
    assert_eq!(cursor, decoded);
  }

  #[test]
  fn cursor_decode_invalid_base64() {
    let result = TestCursor::decode("not!valid!base64!!!");
    assert!(matches!(result, Err(CursorError::InvalidFormat(_))));
  }

  #[test]
  fn priority_keyset_cursor_roundtrip() {
    use chrono::Utc;
    use uuid::Uuid;

    type PC = PriorityKeysetCursor<Uuid>;
    let cursor = PC {
      priority:   2,
      created_at: Utc::now(),
      id:         Uuid::now_v7(),
    };
    let encoded = cursor.encode();
    let decoded = PC::decode(&encoded).unwrap();
    assert_eq!(cursor.priority, decoded.priority);
    assert_eq!(cursor.created_at, decoded.created_at);
    assert_eq!(cursor.id, decoded.id);
  }

  #[test]
  fn cursor_decode_invalid_json() {
    // Valid base64 but not valid JSON for TestCursor
    let encoded = URL_SAFE_NO_PAD.encode(b"not json");
    let result = TestCursor::decode(&encoded);
    assert!(matches!(result, Err(CursorError::InvalidData(_))));
  }

  #[test]
  fn cursor_paginated_response_with_more() {
    let response: PaginatedResponse<i32> =
      PaginatedResponse::new(vec![1, 2, 3], Some("cursor123".to_string()), true);

    assert_eq!(response.data, vec![1, 2, 3]);
    assert_eq!(response.next_cursor, Some("cursor123".to_string()));
    assert!(response.has_more);
  }

  #[test]
  fn cursor_paginated_response_last_page() {
    let response: PaginatedResponse<i32> = PaginatedResponse::new(vec![1, 2], None, false);

    assert_eq!(response.data, vec![1, 2]);
    assert!(response.next_cursor.is_none());
    assert!(!response.has_more);
  }

  #[test]
  fn cursor_paginated_response_empty() {
    let response: PaginatedResponse<i32> = PaginatedResponse::empty();

    assert!(response.data.is_empty());
    assert!(response.next_cursor.is_none());
    assert!(!response.has_more);
  }

  // from_rows tests

  #[derive(Debug, Clone)]
  struct Row {
    ts: i64,
    id: String,
  }

  #[derive(Debug, Serialize, Deserialize)]
  struct RowCursor {
    ts: i64,
    id: String,
  }

  impl Cursor for RowCursor {}

  fn make_rows(n: usize) -> Vec<Row> {
    (0..n)
      .map(|i| Row { ts: 1000 + i as i64, id: format!("r{i}") })
      .collect()
  }

  #[test]
  fn from_rows_has_more_pops_extra() {
    let rows = make_rows(11); // limit=10, fetched 11 → has_more
    let resp = PaginatedResponse::from_rows(rows, 10, |r| RowCursor { ts: r.ts, id: r.id.clone() });

    assert_eq!(resp.data.len(), 10);
    assert!(resp.has_more);
    assert!(resp.next_cursor.is_some());

    // cursor encodes last remaining row (index 9)
    let decoded = RowCursor::decode(resp.next_cursor.as_ref().unwrap()).unwrap();
    assert_eq!(decoded.ts, 1009);
    assert_eq!(decoded.id, "r9");
  }

  #[test]
  fn from_rows_exact_limit_no_more() {
    let rows = make_rows(10); // limit=10, fetched 10 → no more
    let resp = PaginatedResponse::from_rows(rows, 10, |r| RowCursor { ts: r.ts, id: r.id.clone() });

    assert_eq!(resp.data.len(), 10);
    assert!(!resp.has_more);
    assert!(resp.next_cursor.is_none());
  }

  #[test]
  fn from_rows_empty() {
    let rows: Vec<Row> = vec![];
    let resp = PaginatedResponse::from_rows(rows, 10, |r| RowCursor { ts: r.ts, id: r.id.clone() });

    assert!(resp.data.is_empty());
    assert!(!resp.has_more);
    assert!(resp.next_cursor.is_none());
  }

  #[test]
  fn from_rows_under_limit() {
    let rows = make_rows(3); // limit=10, fetched 3 → no more
    let resp = PaginatedResponse::from_rows(rows, 10, |r| RowCursor { ts: r.ts, id: r.id.clone() });

    assert_eq!(resp.data.len(), 3);
    assert!(!resp.has_more);
    assert!(resp.next_cursor.is_none());
  }
}
