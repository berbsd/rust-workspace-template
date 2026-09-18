# OpenAPI Annotations (utoipa v5 + Scalar)

Adding `#[utoipa::path]`/`#[derive(ToSchema)]` annotations to a service that documents an
OpenAPI surface. This template doesn't enable OpenAPI by default (`services/example`
doesn't use it) — `common-types` supports it behind an `openapi` Cargo feature, so add it
the day a service actually needs a published spec, not preemptively.

There is no shared service framework here providing a `with_openapi()` builder or a
central security-scheme registry — wire utoipa and utoipa-scalar directly against axum, as
shown below.

## Relationship with doc comments

Doc comments are the single source of truth; utoipa annotations are additive, never a
replacement:

| Concern | Owned by | Format |
|---|---|---|
| Summary line (handlers) | This doc's "Handler Summary Convention" | `///` first line → OpenAPI `summary` |
| Description (handlers) | doc comment body | `///` body → OpenAPI `description` |
| Struct/field descriptions | doc comments | `///` → schema `description` |
| `# Errors` ↔ `responses(...)` | both, must agree | every error variant needs a status code entry and vice versa |
| OpenAPI examples | this doc | `#[schema(example)]`, request/response examples |
| Validation constraints | this doc | `#[schema(min_length, maximum, ...)]` |

Never contradict: if the doc comment says "returns `None` if not found," the response must
include a 404.

## Setup

```toml
# The service's own Cargo.toml
[dependencies]
common-types = { workspace = true, features = ["openapi"] }
utoipa       = { workspace = true, features = ["axum_extras", "uuid", "chrono"] }
utoipa-scalar = { workspace = true, features = ["axum"] }
```

**`utoipa` must be a direct dependency**, not just pulled in transitively through
`common-types`. Derive macros (`ToSchema`, `IntoParams`, `OpenApi`) generate code that
references `utoipa` by name — without a direct dependency, that code fails to resolve.

Shared types in `common-types` use conditional derives so they don't force the `utoipa`
dependency on a consumer that doesn't want it:

```rust
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiErrorBody { /* ... */ }
```

## Annotation Guide

### Types: `#[derive(ToSchema)]`

```rust
/// A widget as returned to API consumers.
#[derive(Debug, Serialize, ToSchema)]
pub struct WidgetResponse {
  /// Unique widget identifier.
  pub id: Uuid,
  /// Display name.
  #[schema(example = "Left flange")]
  pub name: String,
  /// When the widget was created.
  pub created_at: DateTime<Utc>,
}
```

- `///` provides the description (shared with rustdoc).
- `#[schema(example = ...)]` gives a realistic field example — never `"string"` or `0`.
- Add constraints that reflect real bounds: `min_length`, `max_length`, `minimum`,
  `maximum`, `pattern`.
- `#[schema(read_only)]` for response-only fields (`id`, `created_at`); `#[schema(write_only)]`
  for request-only fields (`password`).
- Serde attributes (`rename_all`, `rename`, `tag`, `skip`, `default`, `flatten`) are
  respected automatically.

### Query/Path Parameters: `#[derive(IntoParams)]`

```rust
/// `GET /widgets` query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListParams {
  /// Page size, 1–50 (default 25). Values above 50 return 400.
  pub limit: Option<i64>,
  /// Opaque cursor from a previous page's `next_cursor`.
  pub cursor: Option<String>,
}
```

`IntoParams` types go in `params(...)` on the handler annotation — **never** in
`components(schemas(...))`, which is only for `ToSchema` types. With the `axum_extras`
feature enabled, utoipa infers `parameter_in` from the extractor (`Query<T>`, `Path<T>`),
so `#[into_params(parameter_in = Query)]` is unnecessary.

### Handlers: `#[utoipa::path(...)]`

```rust
/// Retrieve a widget
///
/// Returns the widget with the given id.
///
/// # Errors
///
/// - [`WidgetError::NotFound`] — no widget has that id.
#[utoipa::path(
  get,
  path = "/widgets/{id}",
  tag = "widgets",
  responses(
    (status = 200, description = "Widget retrieved", body = WidgetResponse),
    (status = 404, description = "No widget with that id", body = common_types::ApiErrorBody),
  ),
)]
pub async fn get_widget(/* ... */) { /* ... */ }
```

- The handler's `///` doc comment supplies `summary` (first line) and `description` (rest)
  — see "Handler Summary Convention" below.
- `path` must match the **full route the client calls**, including any gateway prefix this
  service is nested under — not just the bare route registered in the local router.
- `responses(...)` must list **every** status code the handler can actually return.
  Use the shared error body type (`common_types::ApiErrorBody`) on all error responses.
- `operation_id` defaults to the function name; SDK generators key on it, so keep handler
  names unique per service and pin `operation_id` explicitly only to preserve stability
  across a rename (see "Deprecation" below).
- If the route requires authentication, add `security(("<scheme>" = []))` matching what
  the route *actually enforces* — never declare a scheme nothing validates. There's no
  shared `SecurityAddon` here; register the scheme once, wherever this service's auth
  middleware lives, and keep every handler's `security(...)` in sync with it by hand.
  `rust-quality`'s prompt 26 (`26-openapi-security-accuracy.md`) is the audit for this,
  conditional on the workspace actually having an auth layer.

### Request bodies, query params, and form-encoded bodies

```rust
#[utoipa::path(
  post,
  path = "/widgets",
  request_body(content = CreateWidgetRequest, description = "The widget to create"),
  responses((status = 201, description = "Widget created", body = WidgetResponse)),
)]
pub async fn create_widget(/* ... */) { /* ... */ }

#[utoipa::path(
  get,
  path = "/widgets",
  params(ListParams),
  responses((status = 200, description = "A page of widgets", body = PaginatedWidgetResponse)),
)]
pub async fn list_widgets(/* ... */) { /* ... */ }
```

Form-encoded bodies (`axum::Form<T>`) annotate identically to JSON bodies — `T` still
derives `ToSchema` and is referenced the same way in `request_body(content = T)`.

### OpenApi root: `#[derive(OpenApi)]`

Handler paths must be **fully qualified** when the handler lives in a different module
than `ApiDoc`:

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  info(title = "{{PROJECT_NAME}} — example service", version = "0.1.0"),
  paths(
    crate::feature::widget::handler::create_widget,
    crate::feature::widget::handler::get_widget,
    crate::feature::widget::handler::list_widgets,
  ),
  components(schemas(
    crate::feature::widget::model::WidgetResponse,
    crate::feature::widget::model::CreateWidgetRequest,
    common_types::ApiErrorBody,
  )),
  tags((name = "widgets", description = "Widget CRUD")),
)]
struct ApiDoc;
```

- `paths(...)` — fully qualified paths to `#[utoipa::path]`-annotated functions.
- `components(schemas(...))` — every `ToSchema` type. `IntoParams` types do **not** go
  here; they register automatically via `params(...)` on the handler.
- `tags(...)` — groups endpoints in the Scalar sidebar.

### Wiring it into the router (plain axum + Scalar, no framework)

```rust
use axum::Router;
use utoipa::OpenApi as _;
use utoipa_scalar::{Scalar, Servable as _};

pub fn router(pool: PgPool) -> Router {
  Router::new()
    .merge(feature::widget::router(pool))
    .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
}
```

`Scalar::with_url(path, spec)` returns a router serving both the interactive docs at
`path` and the raw spec at `{path}/openapi.json`. For custom Scalar JS configuration
(hiding the sidebar, disabling the client picker, etc.) beyond what `Scalar` exposes
directly, `Scalar::new(spec).to_html()` renders the HTML yourself, which you can then
serve from a handler with a custom `<script>` configuration block — see
[utoipa-scalar's docs](https://docs.rs/utoipa-scalar) for the `custom_html` method.

**If this service is nested under a path prefix by a host** (see the main `SKILL.md`'s
"Service anatomy" / this workspace's `hosts/example-host`), mount the docs under that same
prefix — a spec reachable at `/docs` when the actual API is nested at `/example/*` links
to routes that 404.

## Handler Summary Convention (Stripe-style)

Handler `///` summary lines appear in the Scalar sidebar — short, scannable labels, not
sentences.

| Rule | Example |
|---|---|
| Imperative verb, not third-person | `Create` not `Creates` |
| Always include an article | `a`/`an` for singular, `all` for lists |
| Lowercase nouns | `a widget` not `a Widget` |
| **Except acronyms/proper nouns**, which keep their casing | `Create an API key`, `Validate a JWT` |
| No trailing period | `Create a widget` not `Create a widget.` |
| No implementation details | `List all widgets` not `List all widgets with pagination` |
| No HTTP method/path echoing | the spec already shows `GET /widgets` |
| Aim for ≤40 characters | a guideline, not a hard limit — don't mangle a longer summary to fit |

### Verb mapping by HTTP method

| Method | Pattern | Examples |
|---|---|---|
| `POST` (create) | `Create a {noun}` | `Create a widget` |
| `GET` (single) | `Retrieve a {noun}` | `Retrieve a widget` |
| `GET` (list) | `List all {nouns}` | `List all widgets` |
| `PATCH`/`PUT` | `Update a {noun}` | `Update a widget` |
| `DELETE` | `Delete a {noun}` | `Delete a widget` |
| `POST` (action, non-CRUD) | `{Verb} a {noun}` | `Submit a report`, `Approve a request` |

### Doc comment structure

```rust
/// Retrieve a widget
///
/// Returns the widget with the given id.
///
/// # Errors
///
/// - [`WidgetError::NotFound`] — no widget has that id.
#[utoipa::path(get, path = "/widgets/{id}", ...)]
pub async fn get_widget(/* ... */) { /* ... */ }
```

The first `///` line becomes the OpenAPI `summary`; everything after the blank `///` line
becomes the `description`.

## Prose style

| Slot | Register | Example |
|---|---|---|
| Summary (`///` first line) | Stripe-style label — imperative, ≤40 chars, no period | `List all widgets` |
| `description = "..."` strings (responses, params, request_body, tags) | Sentence-case fragment, no trailing period | `A page of widget summaries` |
| Description bodies (handler `///` body, field/type `///`) | Full sentences with periods | `Returns the widget with the given id.` |

- American English throughout.
- Acronyms and proper nouns keep their casing everywhere — `JWT`, `OAuth`.
- Second person for the caller's own actions/credentials/quota ("your request"); neutral
  voice for system behavior.
- One term per concept per service — pick the caller-facing word and reuse it in every
  summary, description, and field doc.
- Field `///` docs keep the full-sentence rustdoc convention (trailing period) even though
  they land in the spec — the no-period rule is for *labels* (summaries, `description =
  "..."` strings), not prose.

## Status-code conventions

| Operation | Status | Note |
|---|---|---|
| `POST` that creates | `201` | never `200` — bare `Ok(Json(x))` defaults to 200, which is wrong here |
| `DELETE` with no body | `204` | no response body |
| `PATCH` partial update | `200` | request schema is all-`Option`, description says omitted fields are unchanged |
| List with `limit` over the cap | `400` | document the cap in the description — **never** silently clamp |

Documented codes drift because they live in three places nothing reconciles: the handler's
return expression, the error enum's `status_code()` mapping, and `responses(...)`. When
auditing, derive the actual status set from the first two and diff it against the third.

## Documenting pagination

List endpoints use `common_types::{PaginatedResponse, KeysetCursor}` (see `services/example`
for the working pattern):

- The `{Resource}ListParams` struct derives `IntoParams`, with `///` on both fields —
  document the `limit` contract explicitly: `/// Page size, 1–50 (default 25). Values above
  50 return 400.`
- The response is `PaginatedResponse<T>`. Register the concrete instantiation as a
  nameable schema via `#[aliases(WidgetListResponse = PaginatedResponse<WidgetResponse>)]`
  so generators emit a named type instead of an anonymous inline object.
- Document `next_cursor` semantics once, on the response type, not per endpoint.

## Deprecation

- Add `deprecated` to `#[utoipa::path]` — Scalar renders a strikethrough badge.
- Pin `operation_id` explicitly so existing consumers keep working while a replacement
  handler takes the natural name.
- State the replacement as the first description line after the summary: `Deprecated: use
  \`PUT /widgets/{id}\` instead.`
- If the handler emits a `Deprecation` header, document it:
  `headers(("Deprecation" = String, description = "Always true on this endpoint"))`.

## Operation-level examples

Field-level `#[schema(example)]` is the baseline; add operation-level examples when the
composite would mislead — payloads with variants, mutually exclusive fields, or
state-dependent responses:

```rust
#[utoipa::path(
  post,
  path = "/widgets",
  request_body(
    content = CreateWidgetRequest,
    examples(
      ("minimal" = (summary = "Minimal widget", value = json!({"name": "Left flange"}))),
    ),
  ),
  responses(
    (status = 201, description = "Widget created", body = WidgetResponse,
      example = json!({"id": "0195...", "name": "Left flange", "created_at": "2026-01-01T00:00:00Z"})),
  ),
)]
```

- Error examples use the wire envelope `{ "error", "message", "details"? }` — the JSON
  field is `error` (from `error_code()`), not `error_code`.
- Examples are static JSON baked in at compile time — keep them synthetic-but-realistic,
  and reuse the same invented entities across related endpoints.
- Paginated responses are the main place a per-field composite fails: give list endpoints
  an operation-level response example with a plausible `next_cursor` and `has_more: true`.

## Common pitfalls

1. **`PartialSchema` not implemented** — `ToSchema` requires `PartialSchema` as a
   supertrait in utoipa v5. Implementing `ToSchema` by hand means implementing
   `PartialSchema` too; the compiler error names the missing bound.
2. **`utoipa` not a direct dependency** — derive macros reference `utoipa` by name; a
   transitive dependency isn't enough.
3. **`__path_handler_name` not found** — paths in `paths(...)` must be fully qualified
   when the handler lives in a different module (`crate::feature::widget::handler::get_widget`,
   not just `get_widget`).
4. **`IntoParams` types in `schemas()`** — they don't go there; they register via
   `params(...)` on the handler and cause a compile error if also listed in `schemas()`.
5. **Blank page at `/docs`** — if this service is nested under a gateway/host prefix,
   mount the docs router under that same prefix.
6. **`Http::bearer_format` is not a method** — in utoipa v5, `Http` has public fields, not
   fluent setters: `http.bearer_format = Some("JWT".to_string())`.
7. **`unexpected_cfgs` warnings** — a `cfg(feature = "openapi")` block expanded by a macro
   in a crate that doesn't define that feature triggers this; `#[allow(unexpected_cfgs)]`
   on the block.
8. **Forgetting to register** — a `#[utoipa::path]`-annotated handler not listed in
   `paths(...)` never appears in the spec; a `ToSchema` type not in
   `components(schemas(...))` gets inlined instead of referenced.
9. **Serde/schema attribute conflicts** — when both `#[serde(rename_all)]` and
   `#[schema(rename_all)]` exist, serde wins. Use only one.
10. **Recursive types** — `#[schema(no_recursion)]` on one edge breaks the infinite loop.
11. **`# Errors` / `responses` drift** — after changing error handling, update both; they
    must stay in sync.
12. **Overriding doc comments with `description`** — avoid `#[schema(description = "...")]`
    unless the OpenAPI description genuinely must differ from the rustdoc one; prefer
    improving the doc comment to serve both.
13. **`Scalar::new()` vs `Scalar::with_url()`** — `new()` takes just the spec (for the
    standalone `to_html()` path); `with_url(path, spec)` sets both the route path and the
    spec for direct axum mounting.

## Sync validation checklist

- [ ] Every handler has both a doc comment and `#[utoipa::path]`.
- [ ] First doc-comment line follows the Stripe-style convention.
- [ ] `# Errors` variants match `responses(...)` status codes.
- [ ] `ToSchema` types have `///` on every field.
- [ ] OpenAPI examples are valid instances of their schemas.
- [ ] All handlers are in `#[derive(OpenApi)]`'s `paths(...)`, fully qualified.
- [ ] All `ToSchema` types are in `components(schemas(...))`; `IntoParams` types are not.
- [ ] `path = "..."` uses the full route the client actually calls.
- [ ] `utoipa` is a direct dependency in the service's `Cargo.toml`.
- [ ] POST-create documents 201; DELETE documents 204; list endpoints document the limit
      cap as 400 (never clamping).
- [ ] Deprecated operations carry `deprecated`, a pinned `operation_id`, and the
      replacement path in the description.
- [ ] If this service has auth, every `security(...)` matches what the route actually
      enforces (`rust-quality` prompt 26).
