# Structured Logging Compliance

Verify all logging follows this workspace's structured-logging contract: `tracing` macros with key-value fields (not interpolated strings), `#[tracing::instrument]` on boundary-crossing functions, no secrets in fields, and event constants in the `<type>.<action>.<status>` shape.

## Background

This workspace uses `tracing`, typically with a JSON-formatting layer (`tracing-subscriber`'s built-in formatter, or a platform-specific one) so a log aggregator can filter, query, and alert on structured fields — `event`, `user_id`, etc. String-interpolated messages (`info!("user {} did {}", id, action)`) defeat that pipeline because the values aren't queryable.

## What to find

Scan all `*.rs` files in `crates/` and `services/`. Skip `target/` and generated code.

### 1. String interpolation in log macros

Flag any `info!`, `warn!`, `error!`, `debug!`, `trace!` call where dynamic values are embedded in the format string instead of carried as fields:

```rust
// Bad
info!("user {user_id} created widget {widget_id}");
error!("failed to connect: {e}");

// Good
info!(%user_id, %widget_id, "user created widget");
error!(error = %e, "failed to connect");
```

The exception: a fully static message string with no variable substitution is fine (`info!("service ready")`). Anything with `{...}` substitution should move to fields.

### 2. Missing `#[instrument]` on boundary functions

Functions that cross an I/O boundary (HTTP handler, DB query, external API call, queue publish) should carry `#[tracing::instrument(...)]`. Flag any such function without one.

Required attributes for handlers:

- `skip_all` (or explicit `skip(state, ...)`) — never let request bodies be auto-captured.
- `err` — so failures are recorded on the span.
- `fields(...)` — selectively surface non-sensitive identifiers (`%user_id`, `%widget_id`).

Flag handlers with bare `#[instrument]` (no `skip_all` / `skip(...)`) — auto-capture of `Json<T>` parameters can leak credentials into logs.

#### Service-method shape table

Service layer methods follow these exact shapes — flag deviations:

| Method type                           | Attribute                                                         |
|---------------------------------------|-------------------------------------------------------------------|
| Mutation (create/update/delete) on a typed request | `#[instrument(skip(self, req), err)]`                |
| Query / read by ID                    | `#[instrument(skip(self), err)]` (let scalar IDs onto the span)   |
| Sensitive inputs (token, password, key, JWT claims) | `#[instrument(skip_all, fields(user_id = %claims.sub), err)]` — selectively expose safe identifiers |
| Infallible / best-effort fire-and-forget | `#[instrument(skip_all)]` (no `err`)                           |

`self` is always skipped — it carries the full service struct (repos, clients, config) and would flood logs. `err` is always present on `Result`-returning methods.

Do **not** instrument: `new()` / constructors, sync getters returning a field with no I/O, pure functions, private helpers (the parent span covers them).

### 3. Sensitive data in fields or messages

Flag any of the following appearing in `fields(...)`, in interpolated message strings, or in `#[error("...")]` formatters that get logged via `err`:

- `token`, `access_token`, `refresh_token`, `id_token`, `bearer`, `authorization`
- `password`, `passwd`, `secret`, `api_key`, `apikey`
- `code` (when in OAuth context — authorization codes are credentials)
- `signing_key`, `private_key`, `hmac`
- Full request/response bodies (`?req`, `%req`, `?body`, `?response`)
- Email addresses unless explicitly required (use `user_id` instead)

When the field name suggests a secret, the variant must use `skip(...)` not `fields(name = %name)`.

### 4. Manual logs that duplicate `#[instrument]`

Flag manual log lines that the instrument macro already emits:

- `info!("starting X")` / `info!("entered Y")` at the top of an instrumented function.
- `error!("X failed: {e}")` followed by `Err(e)` — `err` flag already records this.
- `debug!("returning {result:?}")` at the end of a function — use `ret(level = "debug")` if genuinely needed.

Keep manual logs only for *business events* (payments, auth state transitions, rate-limit triggers, queue publishes).

### 5. Event constant format

Where the project uses an `event` field to label structured events, the value must follow the `<type>.<action>.<status>` convention, e.g. `service.lifecycle.ready`, `job.process.failed`, `user.sync.success`.

Flag event values that:

- Use a different separator (`/`, `-`, `:`) instead of `.`.
- Don't have three segments.
- Use mixed case (must be lowercase dotted).
- Are not declared as a `pub const` or `&'static str` constant — string literals scattered through call sites drift.

### 6. Cross-service consistency

After scanning, compare event constants across services. Flag the same semantic event using different names (`auth.token.success` in one service, `auth.token.granted` in another). Suggest unification.

## Verification

```bash
cargo check --workspace
cargo clippy --workspace --no-deps --all-targets
cargo nextest run --workspace --all-features
```

If a fix changes a log call's shape, also check log-based dashboards or alerts that may reference the old field/event name and surface them in the report — don't silently break observability.

## Report format

Group by file. For each finding, give file:line, the offending macro call (one line), the rule violated, and the suggested replacement. End with a per-service summary of event-constant drift if any was found.
