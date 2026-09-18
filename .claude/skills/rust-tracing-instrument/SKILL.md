---
name: rust-tracing-instrument
description: "Use when adding logging to Rust code, reviewing Rust code with verbose or manual info!/debug!/error! calls, migrating Rust logging to spans, or reducing logging bloat in Rust services. Triggers on \"tracing\", \"instrument\", \"structured logging\", \"span\", \"observability\", or logging in Rust — including when writing new Rust functions that need logging."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash
---

# Rust Tracing Instrument Skill

## Purpose

Replace verbose manual logging patterns (`info!`, `debug!`, `error!` scattered throughout function bodies) with idiomatic `#[tracing::instrument]` attributes and span-based context. The goal is to **reduce code bloat by 30-40%** while maintaining or improving observability.

## Companion Skills

### rust-documenter

Doc comments written by `rust-documenter` describe *what* a function does. `#[instrument]` describes *how it's observed*. They complement each other — doc comments explain contracts, spans capture runtime behavior. When both apply, write the doc comment first, then add `#[instrument]`.

### rust-quality

The `rust-quality` skill's Clone Analysis (#10) and Zero-Copy (#11) checks may conflict with instrument fields. After running quality checks that change function signatures or parameter types, re-verify that `skip(...)` and `fields(...)` still reference valid parameters.

## Core Principles

1. **Spans carry context, not log lines.** Function parameters, error chains, and return values belong on spans — not scattered through `info!`/`error!` calls.
2. **Errors propagate with `.context()`.** Add semantic meaning at each step; log the assembled chain once at the boundary with `err`.
3. **Manual logs are for business events only.** If it's not worth alerting on or auditing, it doesn't need a manual log line.
4. **Stay grounded in the workspace.** Only reference `tracing`, `tracing-subscriber`, and other crates actually in the project's `Cargo.toml`. Don't assume `opentelemetry`, `tracing-opentelemetry`, or specific subscriber layers exist unless they do.
5. **Compile after every change.** Every instrumentation change must leave the project compiling. Run `cargo check` after each modification.
6. **Respect existing patterns.** Match the project's existing tracing conventions — span naming, level choices, field formatting. Don't impose a different style.

---

## Workflow

### Step 1: Scan the project

Before instrumenting, understand the tracing setup:

1. Read `Cargo.toml` — check for `tracing`, `tracing-subscriber`, `tracing-opentelemetry`, `tracing-appender` and their feature flags
2. Find the subscriber setup — search for `tracing_subscriber::fmt`, `Registry::default()`, or `init_subscriber` to understand what layers are active (JSON? compact? pretty? OpenTelemetry?)
3. Identify existing conventions — grep for `#[instrument` and `#[tracing::instrument` to see what naming, level, and skip patterns the project already uses
4. Check error handling — is the project using `anyhow`, `thiserror`, `eyre`, or custom error types? This affects `err` behavior.
5. Note sensitive types — identify types containing secrets, tokens, or PII that must always be skipped

### Step 2: Instrument functions

Process files in the order that makes sense for the change scope:

- **New code**: Apply the templates below from the start
- **Migration**: Work module-by-module, starting from leaf functions (deepest call depth) and moving outward toward handlers/entrypoints. This avoids duplicate logging where an inner function logs and the outer function also logs the same context.

### Step 3: Verify

After instrumentation changes:

```bash
# Must compile — instrument macro errors are compile-time
cargo check

# Run tests — instrument changes should not break behavior
cargo test

# Run clippy — catches unused imports from removed log lines
cargo clippy --all-targets --all-features
```

---

## When Writing New Functions

Every async or sync function that performs meaningful work should use `#[instrument]` instead of manual entrance/exit/error logs.

### Default template for a new function:

```rust
#[tracing::instrument(skip(db), err)]
async fn create_user(db: &Pool, req: CreateUserRequest) -> Result<User> {
    let validated = validate_input(&req)?;
    let user = db.insert_user(validated).await
        .context("inserting user")?;
    Ok(user)
}
```

This single attribute replaces what would otherwise be 4-6 manual log lines (entrance log, error log per operation, debug-on-success).

---

## Migration Patterns

When encountering existing code with manual logging, apply these transformations in order:

### Pattern 1: Remove entrance logs → use `#[instrument]`

BEFORE:
```rust
async fn get_user(db: &Pool, user_id: Uuid) -> Result<User> {
    info!("fetching user {}", user_id);
    // ...
}
```

AFTER:
```rust
#[tracing::instrument(skip(db))]
async fn get_user(db: &Pool, user_id: Uuid) -> Result<User> {
    // ...
}
```

All non-skipped parameters are automatically captured as span fields.

### Pattern 2: Remove `.map_err()` error logging → use `err` flag

BEFORE:
```rust
let user = db.fetch_user(user_id).await.map_err(|e| {
    error!("failed to fetch user {}: {}", user_id, e);
    e
})?;
```

AFTER:
```rust
#[tracing::instrument(skip(db), err)]
async fn get_user(db: &Pool, user_id: Uuid) -> Result<User> {
    let user = db.fetch_user(user_id).await
        .context("fetching user")?;
    // ...
}
```

The `err` flag logs any `Err` return at ERROR level automatically. The `user_id` is already on the span. Use `.context("description")` (from `anyhow`) to add semantic meaning to error chains instead of logging at each step.

### Pattern 3: Remove debug-on-success logs → remove entirely or use `ret`

BEFORE:
```rust
let user = db.fetch_user(user_id).await?;
debug!("found user: {:?}", user.email);
```

AFTER — **prefer removing entirely** (if the caller proceeds, the call succeeded):
```rust
let user = db.fetch_user(user_id).await
    .context("fetching user")?;
```

AFTER — only if the return value is diagnostically important:
```rust
#[tracing::instrument(skip(db), err, ret(level = "debug"))]
```

### Pattern 4: Mid-function computed values → use `fields` with `Empty`

BEFORE:
```rust
let user = db.fetch_user(user_id).await?;
debug!("user email: {}", user.email);
```

AFTER:
```rust
#[tracing::instrument(skip(db), err, fields(email = tracing::field::Empty))]
async fn get_user(db: &Pool, user_id: Uuid) -> Result<User> {
    let user = db.fetch_user(user_id).await?;
    tracing::Span::current().record("email", user.email.as_str());
    // ...
}
```

Only use this when the value is genuinely useful for debugging/filtering. Do not record values just because they exist.

### Pattern 5: Replace error-logging `.map_err()` chains → `.context()` propagation

BEFORE:
```rust
let config = load_config().map_err(|e| {
    error!("failed to load config: {}", e);
    AppError::Config(e)
})?;
let db = connect_db(&config).map_err(|e| {
    error!("failed to connect to db: {}", e);
    AppError::Database(e)
})?;
```

AFTER:
```rust
#[tracing::instrument(err)]
async fn initialize(config_path: &str) -> Result<AppState> {
    let config = load_config(config_path)
        .context("loading config")?;
    let db = connect_db(&config).await
        .context("connecting to database")?;
    Ok(AppState { config, db })
}
```

The `err` attribute logs the full chain once at the boundary: `"connecting to database: connection refused"`.

---

## What to Skip

Always `skip` parameters that:
- Don't implement `Debug` or `Display` (connection pools, HTTP clients, large state)
- Contain sensitive data (passwords, tokens, API keys)
- Are large structs that would bloat log output

```rust
// Skip non-displayable and sensitive params, selectively expose what matters
#[tracing::instrument(
    skip(db, auth_client),
    fields(user_id = %req.user_id, action = %req.action)
)]
```

Use `skip_all` when most params should be hidden, then selectively add with `fields(...)`:

```rust
#[tracing::instrument(skip_all, fields(user_id = %user_id))]
```

---

## What to Keep as Manual Logs

After applying instrument patterns, the **only** manual log lines remaining should be genuine business events that aren't captured by enter/exit/error spans:

```rust
#[tracing::instrument(skip(db), err)]
async fn process_order(db: &Pool, order: Order) -> Result<Receipt> {
    let receipt = db.charge(&order).await
        .context("charging order")?;

    // KEEP: This is a meaningful business event worth logging explicitly
    info!(
        order_id = %order.id,
        amount = %receipt.amount,
        currency = %receipt.currency,
        "order processed successfully"
    );

    Ok(receipt)
}
```

**Guidelines for keeping manual logs:**
- Payment/financial events
- Security-relevant actions (login, permission changes)
- State machine transitions
- Rate limit triggers or circuit breaker events
- Anything you'd want to alert on in production

**Remove manual logs for:**
- Function entry ("starting X...")
- Operation success ("X completed")
- Error logging that duplicates `err` flag behavior
- Debug logging that just prints a variable value

---

## Instrument Attribute Reference

| Attribute | Effect |
|-----------|--------|
| `#[instrument]` | Span with function name + all args |
| `skip(a, b)` | Exclude specific parameters |
| `skip_all` | Exclude all parameters |
| `err` | Log `Err` returns at ERROR |
| `err(level = "warn")` | Log errors at custom level |
| `ret` | Log `Ok` returns at INFO |
| `ret(level = "debug")` | Log success at DEBUG |
| `fields(k = %v)` | Add custom span fields (`%` = Display, `?` = Debug) |
| `fields(k = Empty)` | Declare field, record later with `Span::current().record()` |
| `name = "custom"` | Override span name |
| `level = "debug"` | Set span level (default INFO) |
| `target = "module"` | Set span target for filtering |

---

## Nested Spans

Instrumented functions called from other instrumented functions automatically nest spans. This provides the call chain context for free:

```
ERROR get_user{user_id=550e8400}:fetch_profile{user_id=550e8400}: return=Err(profile query: connection timeout)
```

No manual context threading needed. The span hierarchy gives you the full trace.

---

## Error Handling Integration

This skill pairs with `thiserror`/`anyhow` error handling:

- Use `anyhow::Context` (`.context("description")`) to add meaning to errors at each step
- Use `#[instrument(err)]` to log the assembled error chain once at the function boundary
- Do NOT log errors at intermediate steps — let them propagate with context

```rust
use anyhow::Context;

#[tracing::instrument(skip(db), err)]
async fn get_user_profile(db: &Pool, user_id: Uuid) -> Result<FullProfile> {
    let user = db.fetch_user(user_id).await
        .context("fetching user")?;
    let prefs = db.fetch_preferences(user_id).await
        .context("fetching preferences")?;
    let avatar = storage.get_avatar(user_id).await
        .context("loading avatar")?;
    Ok(FullProfile { user, prefs, avatar })
}
// On error, logs: "loading avatar: S3 timeout" with user_id on the span
```

### `err` Formatting: Display vs Debug

By default, `err` formats the error with `Debug`. Control this explicitly:

```rust
// Default — uses Debug (shows full error chain structure)
#[tracing::instrument(err)]

// Use Display — cleaner output for user-facing error types
#[tracing::instrument(err(Display))]

// Use Debug — explicit, same as default but signals intent
#[tracing::instrument(err(Debug))]
```

**Guideline:** Use `err(Display)` when the error type has a good `Display` impl (e.g., `thiserror` errors). Use `err` (Debug, the default) for `anyhow::Error` where the debug representation includes the full chain.

---

## When NOT to Instrument

Not every function benefits from `#[instrument]`. Skip it for:

### Trivial getters and field accessors

```rust
// Do NOT instrument — adds noise, no diagnostic value
pub fn name(&self) -> &str {
    &self.name
}
```

### Pure computation with no I/O

```rust
// Do NOT instrument — called thousands of times, no side effects
fn calculate_checksum(data: &[u8]) -> u32 {
    // ...
}
```

### Hot loops and high-frequency calls

```rust
// Do NOT instrument — span creation overhead matters at high call rates
fn process_packet(packet: &Packet) -> Result<Frame> {
    // called per-packet at 10k+/sec
}
```

### Closures and short-lived iterators

`#[instrument]` cannot be applied to closures. Use `tracing::info_span!` inline only if the closure does meaningful I/O:

```rust
// Only if the closure does I/O worth tracing
let results: Vec<_> = items
    .iter()
    .map(|item| {
        let _span = tracing::info_span!("process_item", item_id = %item.id).entered();
        transform(item)
    })
    .collect();
```

### Decision rule

> Instrument functions that **cross a boundary**: network, filesystem, database, external service, or significant state mutation. Skip functions that are pure data transformation within a single process.

---

## Async Considerations

### `#[instrument]` on async functions

When applied to an `async fn`, `#[instrument]` creates a span that lives for the **entire lifetime of the returned future** — not just until the function body starts. This means the span is active across `.await` points, which is the correct behavior for tracing async work.

### Manual span attachment with `.instrument()`

For futures that aren't direct `async fn` return values (e.g., spawned tasks), use the `Instrument` trait:

```rust
use tracing::Instrument;

let span = tracing::info_span!("background_sync", user_id = %user_id);
tokio::spawn(
    async move {
        sync_user_data(user_id).await;
    }
    .instrument(span)
);
```

Without `.instrument()`, the spawned task loses the parent span context. This is the most common source of "orphaned" spans in async code.

### `in_current_span()` shorthand

When you want a spawned task to inherit the current span rather than creating a new one:

```rust
use tracing::Instrument;

tokio::spawn(
    async move { process(data).await }
        .in_current_span()
);
```

---

## Level Selection Guide

| Level | Use for | Example |
|-------|---------|---------|
| `ERROR` | Failures that need operator attention | Default for `err` flag |
| `WARN` | Degraded operation, recoverable issues | `err(level = "warn")` for expected failures (cache miss fallback) |
| `INFO` | Default span level. Request handling, service operations | `#[instrument]` (default) |
| `DEBUG` | Internal operations useful during development | `level = "debug"` for helper functions, internal plumbing |
| `TRACE` | Very high-frequency, fine-grained diagnostics | `level = "trace"` for per-iteration, per-packet work |

**Rule of thumb:** If the function is called once per user request, `INFO`. If called many times per request, `DEBUG`. If called in a tight loop, `TRACE` (or don't instrument at all).

---

## Advanced Patterns

### Trait method instrumentation

Instrument on the **impl**, not the trait definition. Trait definitions can't have `#[instrument]` because they don't have bodies:

```rust
trait UserStore {
    async fn get_user(&self, id: Uuid) -> Result<User>;
}

impl UserStore for PgUserStore {
    #[tracing::instrument(skip(self), err)]
    async fn get_user(&self, id: Uuid) -> Result<User> {
        // ...
    }
}
```

Use `name = "UserStore::get_user"` if you want the span to reference the trait rather than the concrete impl:

```rust
#[tracing::instrument(name = "UserStore::get_user", skip(self), err)]
```

### Custom span names for disambiguation

When the function name alone is ambiguous (common with `new`, `run`, `process`):

```rust
#[tracing::instrument(name = "order_processor::run", skip_all)]
async fn run(&self) -> Result<()> { /* ... */ }
```

### Axum handler instrumentation

Axum handlers are already wrapped in a span by tower-http's `TraceLayer`. Avoid double-spanning:

```rust
// If using tower_http::trace::TraceLayer, the request span already exists.
// Instrument the handler only if you need ADDITIONAL fields beyond
// what TraceLayer provides (method, uri, status).
#[tracing::instrument(skip_all, fields(user_id))]
async fn get_user_handler(
    State(db): State<Pool>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let user = db.fetch_user(user_id).await
        .context("fetching user")?;
    Ok(Json(user))
}
```

### Field formatting: `%` vs `?`

```rust
#[tracing::instrument(
    fields(
        user_id = %user_id,   // %: Display — clean output: "550e8400-..."
        request = ?request,    // ?: Debug — full struct dump: CreateUserRequest { name: "...", ... }
    )
)]
```

**Prefer `%` (Display)** for IDs, strings, and types with concise Display impls. Use `?` (Debug) only when the full structure is diagnostically needed.

---

## Common Pitfalls

1. **Forgetting `skip` on non-Debug types** — Compilation fails if a parameter doesn't implement `Debug` and isn't skipped. Always `skip` connection pools, HTTP clients, and state handles.

2. **Double logging errors** — If an inner function has `err` AND the outer function also has `err`, the same error is logged twice. Prefer `err` only at the boundary (handler or entrypoint).

3. **Logging sensitive data** — Parameters are captured by default. Always `skip` passwords, tokens, API keys, and PII. Use `skip_all` + selective `fields` for functions handling credentials.

4. **Over-instrumenting** — Instrumenting every function buries meaningful spans in noise. Follow the boundary-crossing rule.

5. **Missing `Instrument` on spawned tasks** — `tokio::spawn` creates a new span context. Without `.instrument(span)` or `.in_current_span()`, the task's spans become orphaned roots.

6. **`ret` on large types** — `ret` uses `Debug` formatting by default. On a type with many fields, this creates enormous log lines. Use `ret(level = "debug")` at minimum, or skip it entirely.

7. **Redundant field names** — `#[instrument]` already captures function parameters by name. Don't re-add them in `fields(...)`:
    ```rust
    // Bad: user_id is already captured from the parameter
    #[tracing::instrument(fields(user_id = %user_id))]
    async fn get_user(user_id: Uuid) -> Result<User> { /* ... */ }

    // Good: just let instrument capture it automatically
    #[tracing::instrument(err)]
    async fn get_user(user_id: Uuid) -> Result<User> { /* ... */ }
    ```

---

## Checklist When Reviewing/Writing Code

### Instrumentation

- [ ] Does the function have `#[instrument]` if it crosses a boundary (I/O, network, DB)?
- [ ] Are trivial getters, pure computations, and hot loops left uninstrumented?
- [ ] Is the span level appropriate? (INFO for per-request, DEBUG for internal, TRACE for high-frequency)
- [ ] Does the span name avoid ambiguity? (Use `name = "..."` for `new`/`run`/`process`)

### Parameter handling

- [ ] Are non-Debug/non-Display params in `skip(...)`?
- [ ] Are sensitive params (passwords, tokens, PII) in `skip(...)`?
- [ ] Are large structs skipped or selectively exposed via `fields(...)`?
- [ ] Is `%` (Display) preferred over `?` (Debug) for IDs and short values?

### Error handling

- [ ] Is `err` used at the boundary, not on every nested function?
- [ ] Are `.map_err()` blocks that only log removed in favor of `err` + `.context()`?
- [ ] Is `err(Display)` used for types with good Display impls?
- [ ] Do error messages use `.context()` for semantic meaning?

### Log line cleanup

- [ ] Are entrance logs (`info!("starting...")`) removed?
- [ ] Are debug-on-success logs removed (or `ret(level = "debug")` only if valuable)?
- [ ] Are remaining manual logs genuine business events, not ceremony?

### Async

- [ ] Do spawned tasks use `.instrument(span)` or `.in_current_span()`?
- [ ] Is the project's existing subscriber setup understood (don't add layers that aren't there)?

### Verification

- [ ] Does `cargo check` pass?
- [ ] Does `cargo test` pass?
- [ ] Does `cargo clippy --all-targets --all-features` pass?

See `references/tracing-attributes.md` for the full attribute reference and `references/span-strategy.md` for the decision guide on when and how to create spans.
