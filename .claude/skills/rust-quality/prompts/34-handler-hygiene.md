# Handler Hygiene

Verify HTTP handlers stay thin — extract, validate, delegate, respond — and that
responses never leak internal state. This is the handler-boundary half of code quality;
input validation (#19), error-enum structure (#13), and pagination mechanics (#28) each
own their own slice and are not repeated here.

## Why

A handler that reaches past its own layer — querying the database directly, calling a
third-party API, branching on business state — duplicates logic the service/repository
layer should own once, and makes the handler untestable without the full stack. A response
type that leaks a raw database row or an internal field is a data exposure the moment a
column is added upstream, not a decision anyone made about that field going out.

## Workflow

Scan `services/*/src/feature/*/handler.rs` (and any `handler_internal.rs`, if this
workspace has an internal/S2S tier). Read each handler function fully before flagging
anything — most of these checks need the complete body, not a line-by-line grep match.

### 1. Business logic in the handler

Handlers extract, validate, delegate to the repository/service layer, and respond —
nothing else. Flag, inside a handler function body:

- Conditional branching on domain state (not input validation or error handling).
- Loops or `.map()`/`.filter()` transforming business data.
- More than one repository/service call in sequence (orchestration belongs one layer down).
- Arithmetic, aggregation, or string manipulation on domain data.

Do **not** flag: input extraction/validation, a single delegate call plus response
mapping, `?` error propagation, or a simple `if` choosing between two status codes.

### 2. Direct repository access from the handler

Search handler files for `sqlx::query`/`sqlx::query_as`, `.fetch_one`/`.fetch_optional`/
`.fetch_all`/`.execute`, `PgPool`/`Pool<Postgres>`, or a direct reference to a
`*Repository` type. A handler that reaches storage without going through
`repository.rs`'s trait is a finding.

### 3. Direct third-party calls from the handler

Search handler files for `reqwest::`/`hyper::`/`Client::new()`, cloud SDK calls, message
queue publishing, or any adapter/client-module reference. Calls to the injected
repository/service via `State(...)` are correct delegation and not a finding; everything
else that reaches an external system directly from the handler is.

### 4. No raw database row in the response

A handler's return type must never be the type its repository's `sqlx::FromRow`/`query_as`
produces. Check the `T` in `Result<Json<T>, _>`: flag if `T` derives `FromRow`, ends in
`Row`, or is defined in `repository.rs`. The wire response is always a distinct type (see
`services/example/src/feature/widget/model.rs`'s `Widget` → `WidgetResponse` split) — a
table column added later should not silently become an API field because nothing marked
the boundary.

### 5. No internal data leakage in the response

Flag response types (success **and** error) that include: secrets/credentials (password
hashes, API keys, tokens), internal-only ids not relevant to the consumer, implementation
markers (`deleted_at` leaking soft-delete, `internal_*`/`raw_*`/`_`-prefixed fields),
database internals (`xmin`, `ctid`), or stack traces/file paths/line numbers. For error
responses specifically, check that `ApiErrorMapping` implementations don't leak a raw
`sqlx::Error`/third-party error body — prompt #13 owns the error-enum-level version of
this; here, check whether a *handler* bypasses the enum and constructs something leaky
inline.

### 6. Correct HTTP status codes and error mapping

| Operation | Correct status |
|---|---|
| `POST` that creates a resource | `201 Created` — not bare `Ok(Json(response))`, which defaults to `200` |
| `POST` that triggers an action, no resource created | `200`/`202` |
| `DELETE`, no body | `204 No Content` |
| `DELETE`, body with updated state (soft-delete) | `200` |
| `PUT`/`PATCH` | `200` |

Also flag handlers that return a raw `StatusCode`/`axum::response::Response` for an error
case, or construct a `Json` error body by hand, instead of returning the domain error type
through `?` — that's an ad-hoc response bypassing the shared `ApiErrorBody` envelope (see
prompt #13).

### 7. Pagination struct naming

A list endpoint's query-params struct must be named `{Resource}ListParams` — not `*Query`,
`*PaginationParams`, `*Filters`, or `*Options`. Prefix `My` for a caller-scoped list
(`MyWidgetListParams`), `Admin` for an admin-only one. Required fields: `cursor:
Option<String>` and `limit: Option<i64>`, plus any resource-specific filters. (This is
about the struct's *name and shape*; prompt #28 owns whether the cursor mechanics
underneath it are correct.)

## Fixing

For each finding, pick exactly one:

1. **Move the logic down a layer** — business logic into the service/repository call, not
   the handler.
2. **Add the missing indirection** — route a direct DB/third-party call through
   `repository.rs`'s trait.
3. **Split response from row** — give the type a dedicated `{X}Response` struct and a
   `From<Row>` (or equivalent) mapping.
4. **Remove the leaking field**, or move it behind whatever internal-only surface this
   workspace has (if any) — never the public response.
5. **Fix the status code** — `(StatusCode::CREATED, Json(response))` for create, `204` for
   bodyless delete.
6. **Route the error through the domain error enum** with `?`, deleting the ad-hoc
   response construction.
7. **Rename the struct** to `{Resource}ListParams` and add any missing `cursor`/`limit`
   fields.

## Verification

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
cargo nextest run --workspace --all-features
```

## Report format

Group by handler file. For each finding: file:line, the handler function name, which rule
(#1–#7) it violates, and the fix applied. End with a per-service count of findings by rule
number.
