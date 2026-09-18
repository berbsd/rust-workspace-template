//! Shared test harness for example service integration tests.
//!
//! Every test drives the real HTTP router — [`example::router`] — over an
//! `#[sqlx::test(migrations = "./migrations")]`-provisioned Postgres pool
//! (needs a running local Postgres; see the workspace `justfile`'s
//! `db-ensure`/`pg-local` tasks and `DATABASE_URL`). Same routes, same
//! handler code, same behavior as production — there's no auth middleware
//! or other layer in this template to substitute a test double for.

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
