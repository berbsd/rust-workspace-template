# Integration Tests

End-to-end HTTP tests against a real, throwaway Postgres — the same
`#[sqlx::test(migrations = "./migrations")]` pattern `services/example`
uses, driving the actual router (not a mock). `service.rs`'s own unit tests
(`06-feature-service.md`) already cover the domain rule without a database;
these prove the wire contract: status codes, JSON shape, the error envelope.

## Files

### 1. `services/{{service}}/tests/common.rs`

Identical to `services/example/tests/common.rs` — this file has no
service-specific content, copy it verbatim and only update the module doc
comment's service name:

```rust
//! Shared test harness for {{service}} service integration tests.
//!
//! Every test drives the real HTTP router — [`{{service_snake}}::router`] —
//! over an `#[sqlx::test(migrations = "./migrations")]`-provisioned
//! Postgres pool (needs a running local Postgres; see the workspace
//! `justfile`'s `db-ensure`/`pg-local` tasks and `DATABASE_URL`). Same
//! routes, same handler code, same behavior as production — there's no
//! auth middleware or other layer in this template to substitute a test
//! double for.

#![allow(dead_code, unreachable_pub, clippy::expect_used, clippy::unwrap_used)]

use axum::body::Body;
use serde_json::Value;

/// Generous ceiling for a test response body — every response in this suite
/// is a handful of JSON fields, so this only exists to turn a runaway
/// response into a clear panic instead of an unbounded read.
const MAX_TEST_BODY_BYTES: usize = 256 * 1024;

/// Reads a response body and parses it as JSON.
///
/// # Panics
///
/// Panics if the body cannot be read within [`MAX_TEST_BODY_BYTES`] — a
/// test-setup bug, not a condition a test should handle.
pub async fn read_json(resp: axum::http::Response<Body>) -> Value {
  let bytes = axum::body::to_bytes(resp.into_body(), MAX_TEST_BODY_BYTES)
    .await
    .expect("read body");
  serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}
```

### 2. `services/{{service}}/tests/{{feature}}_it.rs`

Harness plus the create/get/delete cases in full; the `list_{{feature_plural}}`
pagination cases (newest-first ordering, cursor resume, malformed-cursor
`400`, out-of-range-limit `400`) follow the exact shape of
`services/example/tests/widget_it.rs`'s four `list_widgets_*` tests — read
that file and adapt it rather than re-deriving the pagination assertions
from scratch; the keyset-cursor contract (`rust-quality` check #28) is
identical across every feature in this workspace.

```rust
//! Integration tests for the {{feature}} CRUD endpoints — `POST
//! /{{feature_plural}}`, `GET /{{feature_plural}}/{id}`, `GET
//! /{{feature_plural}}`, `DELETE /{{feature_plural}}/{id}`.
//!
//! Drives the real router ([`{{service_snake}}::router`]) over a
//! `#[sqlx::test(migrations = "./migrations")]`-provisioned Postgres pool —
//! see `tests/common.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

mod common;

use axum::{
  Router,
  body::Body,
  http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt as _;

/// The path every {{feature}} route hangs off.
const {{FEATURE_PLURAL_CONST}}_PATH: &str = "/{{feature_plural}}";

fn request(
  method: &str,
  uri: &str,
  body: Option<&serde_json::Value>,
) -> Request<Body> {
  let builder = Request::builder().method(method).uri(uri);
  match body {
    | Some(b) => builder
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(serde_json::to_vec(b).expect("serialize body")))
      .expect("build request"),
    | None => builder.body(Body::empty()).expect("build request"),
  }
}

fn router(pool: PgPool) -> Router {
  {{service_snake}}::router(pool)
}

/// A `POST /{{feature_plural}}` with a valid body returns `201` and the
/// created {{feature}}, with a server-minted id and `created_at`.
#[sqlx::test(migrations = "./migrations")]
async fn create_{{feature}}_returns_201_with_the_created_{{feature}}(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request(
      "POST",
      {{FEATURE_PLURAL_CONST}}_PATH,
      Some(&json!({{create_request_body_example}})),
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::CREATED);
  let body = common::read_json(resp).await;
  {{create_response_field_assertions}}
  assert!(body["id"].is_string(), "id must be present: {body}");
  assert!(
    body["created_at"].is_string(),
    "created_at must be present: {body}"
  );
}

/// {{invalid_create_description}} fails garde validation before any
/// repository call, and surfaces as the workspace's standard validation
/// envelope.
#[sqlx::test(migrations = "./migrations")]
async fn create_{{feature}}_rejects_invalid_input(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request(
      "POST",
      {{FEATURE_PLURAL_CONST}}_PATH,
      Some(&json!({{invalid_create_request_body}})),
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  let body = common::read_json(resp).await;
  assert_eq!(body["error"], "VALIDATION_ERROR");
}

/// `GET /{{feature_plural}}/{id}` returns the exact {{feature}} a prior
/// create produced.
#[sqlx::test(migrations = "./migrations")]
async fn get_{{feature}}_returns_the_created_{{feature}}(pool: PgPool) {
  let router = router(pool);

  let create_resp = router
    .clone()
    .oneshot(request(
      "POST",
      {{FEATURE_PLURAL_CONST}}_PATH,
      Some(&json!({{create_request_body_example}})),
    ))
    .await
    .expect("request succeeds");
  let created = common::read_json(create_resp).await;
  let id = created["id"].as_str().expect("id is a string");

  let resp = router
    .oneshot(request(
      "GET",
      &format!("/{{feature_plural}}/{id}"),
      None,
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::OK);
  let body = common::read_json(resp).await;
  assert_eq!(body["id"], created["id"]);
}

/// An id no {{feature}} was ever created with returns `404` with the
/// standard not-found envelope.
#[sqlx::test(migrations = "./migrations")]
async fn get_{{feature}}_returns_404_for_an_unknown_id(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request(
      "GET",
      &format!("/{{feature_plural}}/{}", uuid::Uuid::now_v7()),
      None,
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  let body = common::read_json(resp).await;
  assert_eq!(body["error"], "NOT_FOUND");
}

/// `DELETE /{{feature_plural}}/{id}` removes the {{feature}}; a second
/// delete then `404`s.
#[sqlx::test(migrations = "./migrations")]
async fn delete_{{feature}}_removes_it(pool: PgPool) {
  let router = router(pool);

  let create_resp = router
    .clone()
    .oneshot(request(
      "POST",
      {{FEATURE_PLURAL_CONST}}_PATH,
      Some(&json!({{create_request_body_example}})),
    ))
    .await
    .expect("request succeeds");
  let created = common::read_json(create_resp).await;
  let id = created["id"].as_str().expect("id is a string");

  let delete_resp = router
    .clone()
    .oneshot(request("DELETE", &format!("/{{feature_plural}}/{id}"), None))
    .await
    .expect("request succeeds");
  assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

  let second_delete = router
    .oneshot(request("DELETE", &format!("/{{feature_plural}}/{id}"), None))
    .await
    .expect("request succeeds");
  assert_eq!(second_delete.status(), StatusCode::NOT_FOUND);
}

// list_{{feature_plural}}_* tests: port services/example/tests/widget_it.rs's
// list_widgets_returns_newest_first, list_widgets_paginates_with_a_cursor,
// list_widgets_rejects_an_undecodable_cursor, and
// list_widgets_rejects_an_out_of_range_limit here, substituting
// {{feature_plural}}/{{feature}} and the request-body shape.
```

## Placeholders used

- `{{service}}` / `{{service_snake}}` / `{{feature}}` / `{{feature_plural}}`
- `{{FEATURE_PLURAL_CONST}}` — `SCREAMING_SNAKE_CASE` of `{{feature_plural}}`
- `{{create_request_body_example}}` — a realistic `json!({...})` literal
  matching `Create{{Feature}}Request`'s fields
- `{{create_response_field_assertions}}` — one `assert_eq!(body["field"],
  ...)` per field the create response should echo
- `{{invalid_create_description}}` / `{{invalid_create_request_body}}` — a
  request that fails exactly one garde rule (an empty required string, an
  out-of-range number) — pick whichever field has the domain rule from
  `06-feature-service.md`, since that's the rule most likely to regress

## Verify

```bash
just db-ensure
cargo nextest run -p {{service}} --all-features
cargo test --doc -p {{service}} --all-features
```

Every test must run — `0 skipped` in nextest's summary line
(`rust-quality` check #33 applies to generated code too). A test that needs
`#[ignore]` to pass means the feature isn't actually wired correctly yet;
fix the wiring, don't skip the test.
