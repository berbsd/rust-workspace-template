# Error Type Pattern

Verify every service's domain error enum follows the platform contract: `thiserror::Error` derive, garde-friendly variant, `ApiErrorMapping` implementation, and no data leakage through error formatting.

## Background

This workspace maps internal Rust errors to HTTP responses through the `ApiErrorMapping` trait (in `common-types`). Handlers return `Result<_, ServiceError>`, and the trait (via `impl_api_error_response!`) converts that into `(StatusCode, Json<ApiErrorBody>)` with structured details. Errors that don't follow the pattern produce inconsistent HTTP responses, leak internal state, or fail to compile when `req.validate(&())?` is used.

## What to find

Scan each service's `feature/*/error.rs` (or `*_error.rs` in the same crate). For each domain error enum:

### 1. Derive `thiserror::Error`

Required: `#[derive(Debug, thiserror::Error)]`. Flag enums that hand-roll `Display`/`Error` impls — `thiserror` is the workspace standard for service-layer errors. (`anyhow::Error` is fine for non-domain plumbing but must not appear in handler return types.)

### 2. Mandatory `Validation` variant

Every service error enum used by a handler that takes `Json<T>` or `Query<T>` must include:

```rust
#[error(transparent)]
Validation(#[from] garde::Report),
```

Without it, `req.validate(&())?` either fails to compile or forces ugly `.map_err(...)` calls. Flag enums missing this variant on services that have validating handlers.

### 3. `ApiErrorMapping` implementation

The enum must implement `ApiErrorMapping` with:

- `status_code(&self)` — every variant maps to an explicit `StatusCode`. No catch-all `_ => StatusCode::INTERNAL_SERVER_ERROR` unless every variant is genuinely a 500.
- `error_code(&self)` — stable string identifier per variant (e.g. `"VALIDATION_ERROR"`, `"NOT_FOUND"`, `"FORBIDDEN"`). Must be `SCREAMING_SNAKE_CASE` and unique within the enum.
- `details(&self)` — `Validation(report)` must call `common_types::validation_details(report)`; other variants return `None` unless they carry structured details intentionally.

Flag any variant whose `status_code` arm is missing, whose `error_code` is missing, duplicated, or formatted inconsistently.

### 4. Database variant must not leak sqlx details

Every service that touches Postgres has a `Database` variant. It must follow this exact shape:

```rust
/// Unrecoverable database error (500). Not exposed to clients.
#[error("an internal error occurred")]
Database(#[from] sqlx::Error),
```

Mandatory:

- `#[error("an internal error occurred")]` — the literal string sent to clients via the error response. **Never** `#[error("database error: {0}")]` or `#[error("{0}")]` — `sqlx::Error::Display` includes table names, column names, and parts of the query, all of which leak schema to API consumers.
- `#[from] sqlx::Error` — enables `?` propagation from repositories.
- `Debug` is preserved by the enum derive, so `#[instrument(err)]` still logs the full sqlx error server-side. The client sees the generic message; operators see everything.
- `status_code()` maps `Database(_)` → `INTERNAL_SERVER_ERROR`; `error_code()` returns `"INTERNAL_ERROR"`.

If a service uses a repository error wrapper instead of `sqlx::Error` directly (e.g. `RepoError`), the same rule applies at the service layer — the *service-level* `Database` variant has the generic `#[error("an internal error occurred")]` even if the repo-level variant is more descriptive (the repo type is internal and never reaches a response).

Also flag any `impl From<sqlx::Error> for ServiceError` block that calls `error!` / `warn!` / `info!` inside the `From` impl — that duplicates `#[instrument(err)]` and creates surprise logging at conversion time. The conversion should be a plain `Self::Database(err)`.

### 5. No data leakage in `#[error("...")]`

`#[error("...")]` strings flow into log lines and (via `Display`) potentially into client responses. They must not contain:

- Secret/credential values: `#[error("invalid token: {token}")]` — leaks the token. Use `#[error("invalid token")]` and skip the value.
- Raw third-party error bodies: `#[error("upstream: {0}")]` where `{0}` is a `reqwest::Error` from an authenticated call — may include URLs, headers, or response bodies.
- Database-specific identifiers: SQL state codes, table names, constraint names exposed verbatim.
- Long debug dumps: `#[error("{0:?}")]` on large structs.

Acceptable: stable opaque identifiers (`{user_id}`, `{project_id}` — UUIDs are fine) and short human-readable summaries.

### 6. Variant granularity

Flag these smells:

- Free-form `String`-carrying variants used as a dumping ground (e.g. `Other(String)`, `Internal(String)`). Each distinct failure mode should be its own variant.
- Variants that wrap `anyhow::Error` in a service-layer enum (defeats the typing).
- Multiple variants with the same `status_code` *and* `error_code` (callers can't distinguish them — collapse or differentiate the codes).

### 7. Consistency across services

After scanning all services, compare:

- Same `error_code` strings used for the same semantic meaning across services? (`NOT_FOUND` vs `RESOURCE_NOT_FOUND` vs `MISSING` — pick one and unify.)
- Same status code mappings for analogous variants? (Forbidden → 403 in every service, not 401 in some.)

## Verification

```bash
cargo check --workspace
cargo clippy --workspace --no-deps --all-targets
```

After fixes, every handler that uses `?` on `req.validate(&())` must compile without `map_err`.

## Report format

Group by service. For each finding, name the enum, variant (if any), file:line, and the rule violated. End with a cross-service drift summary if `error_code` strings or status mappings disagree.
