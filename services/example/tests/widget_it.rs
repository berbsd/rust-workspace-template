//! Integration tests for the widget CRUD endpoints — `POST /widgets`,
//! `GET /widgets/{id}`, `GET /widgets`, `DELETE /widgets/{id}`.
//!
//! Drives the real router ([`example::router`]) over a
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

/// The path every widget route hangs off.
const WIDGETS_PATH: &str = "/widgets";

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
  example::router(pool)
}

/// A `POST /widgets` with a valid body returns `201` and the created widget,
/// with a server-minted id and `created_at`.
#[sqlx::test(migrations = "./migrations")]
async fn create_widget_returns_201_with_the_created_widget(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request(
      "POST",
      WIDGETS_PATH,
      Some(&json!({ "name": "Left flange" })),
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::CREATED);
  let body = common::read_json(resp).await;
  assert_eq!(body["name"], "Left flange");
  assert!(body["id"].is_string(), "id must be present: {body}");
  assert!(
    body["created_at"].is_string(),
    "created_at must be present: {body}"
  );
}

/// An empty `name` fails garde validation before any repository call, and
/// surfaces as the workspace's standard validation envelope.
#[sqlx::test(migrations = "./migrations")]
async fn create_widget_rejects_an_empty_name(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request("POST", WIDGETS_PATH, Some(&json!({ "name": "" }))))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  let body = common::read_json(resp).await;
  assert_eq!(body["error"], "VALIDATION_ERROR");
}

/// `GET /widgets/{id}` returns the exact widget a prior create produced.
#[sqlx::test(migrations = "./migrations")]
async fn get_widget_returns_the_created_widget(pool: PgPool) {
  let router = router(pool);

  let create_resp = router
    .clone()
    .oneshot(request(
      "POST",
      WIDGETS_PATH,
      Some(&json!({ "name": "Right flange" })),
    ))
    .await
    .expect("create succeeds");
  let created = common::read_json(create_resp).await;
  let id = created["id"].as_str().expect("id is a string");

  let get_resp = router
    .oneshot(request("GET", &format!("{WIDGETS_PATH}/{id}"), None))
    .await
    .expect("get succeeds");

  assert_eq!(get_resp.status(), StatusCode::OK);
  let body = common::read_json(get_resp).await;
  assert_eq!(body["id"], id);
  assert_eq!(body["name"], "Right flange");
}

/// An id no widget was ever created with returns `404` with the standard
/// not-found envelope.
#[sqlx::test(migrations = "./migrations")]
async fn get_widget_returns_404_for_an_unknown_id(pool: PgPool) {
  let router = router(pool);
  let unknown_id = uuid::Uuid::now_v7();

  let resp = router
    .oneshot(request(
      "GET",
      &format!("{WIDGETS_PATH}/{unknown_id}"),
      None,
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  let body = common::read_json(resp).await;
  assert_eq!(body["error"], "NOT_FOUND");
}

/// `GET /widgets` orders results newest-first and reports the last page
/// correctly (`has_more: false`, no cursor) when everything fits in one
/// page.
#[sqlx::test(migrations = "./migrations")]
async fn list_widgets_returns_newest_first(pool: PgPool) {
  let router = router(pool);

  for name in ["first", "second", "third"] {
    router
      .clone()
      .oneshot(request(
        "POST",
        WIDGETS_PATH,
        Some(&json!({ "name": name })),
      ))
      .await
      .expect("create succeeds");
  }

  let resp = router
    .oneshot(request("GET", WIDGETS_PATH, None))
    .await
    .expect("list succeeds");

  assert_eq!(resp.status(), StatusCode::OK);
  let body = common::read_json(resp).await;
  let names: Vec<&str> = body["data"]
    .as_array()
    .expect("data is an array")
    .iter()
    .map(|w| w["name"].as_str().expect("name is a string"))
    .collect();
  assert_eq!(
    names,
    vec!["third", "second", "first"],
    "widgets must be ordered newest-first"
  );
  assert_eq!(body["has_more"], false);
  assert!(body["next_cursor"].is_null());
}

/// A page smaller than the full result set reports `has_more: true` and a
/// cursor that resumes exactly where the first page left off.
#[sqlx::test(migrations = "./migrations")]
async fn list_widgets_paginates_with_a_cursor(pool: PgPool) {
  let router = router(pool);

  for name in ["a", "b", "c"] {
    router
      .clone()
      .oneshot(request(
        "POST",
        WIDGETS_PATH,
        Some(&json!({ "name": name })),
      ))
      .await
      .expect("create succeeds");
  }

  let first_page = router
    .clone()
    .oneshot(request("GET", &format!("{WIDGETS_PATH}?limit=2"), None))
    .await
    .expect("list succeeds");
  assert_eq!(first_page.status(), StatusCode::OK);
  let first_body = common::read_json(first_page).await;
  assert_eq!(
    first_body["data"].as_array().expect("array").len(),
    2,
    "first page holds the requested limit"
  );
  assert_eq!(first_body["has_more"], true);
  let cursor = first_body["next_cursor"]
    .as_str()
    .expect("cursor present on a full page")
    .to_string();

  let second_page = router
    .oneshot(request(
      "GET",
      &format!("{WIDGETS_PATH}?limit=2&cursor={cursor}"),
      None,
    ))
    .await
    .expect("list succeeds");
  assert_eq!(second_page.status(), StatusCode::OK);
  let second_body = common::read_json(second_page).await;
  assert_eq!(
    second_body["data"].as_array().expect("array").len(),
    1,
    "second page holds the one remaining widget"
  );
  assert_eq!(second_body["has_more"], false);
  assert!(second_body["next_cursor"].is_null());
}

/// A malformed cursor is rejected as a `400`, never treated as "start over".
#[sqlx::test(migrations = "./migrations")]
async fn list_widgets_rejects_an_undecodable_cursor(pool: PgPool) {
  let router = router(pool);

  let resp = router
    .oneshot(request(
      "GET",
      &format!("{WIDGETS_PATH}?cursor=not-a-valid-cursor"),
      None,
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
  let body = common::read_json(resp).await;
  assert_eq!(body["error"], "VALIDATION_ERROR");
}

/// `DELETE /widgets/{id}` removes the widget; a second delete then `404`s.
#[sqlx::test(migrations = "./migrations")]
async fn delete_widget_removes_it(pool: PgPool) {
  let router = router(pool);

  let create_resp = router
    .clone()
    .oneshot(request(
      "POST",
      WIDGETS_PATH,
      Some(&json!({ "name": "Disposable" })),
    ))
    .await
    .expect("create succeeds");
  let created = common::read_json(create_resp).await;
  let id = created["id"].as_str().expect("id is a string").to_owned();

  let delete_resp = router
    .clone()
    .oneshot(request("DELETE", &format!("{WIDGETS_PATH}/{id}"), None))
    .await
    .expect("delete succeeds");
  assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

  let get_resp = router
    .oneshot(request("GET", &format!("{WIDGETS_PATH}/{id}"), None))
    .await
    .expect("get succeeds");
  assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

/// Deleting an id no widget was ever created with is a `404`, not a `204`.
#[sqlx::test(migrations = "./migrations")]
async fn delete_widget_returns_404_for_an_unknown_id(pool: PgPool) {
  let router = router(pool);
  let unknown_id = uuid::Uuid::now_v7();

  let resp = router
    .oneshot(request(
      "DELETE",
      &format!("{WIDGETS_PATH}/{unknown_id}"),
      None,
    ))
    .await
    .expect("request succeeds");

  assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
