# `#[tracing::instrument]` Attribute Reference

Complete reference for the `tracing::instrument` proc-macro attribute from the `tracing-attributes` crate (re-exported via `tracing`).

---

## Basic Syntax

```rust
#[tracing::instrument]
fn my_function(param: &str) -> Result<()> { /* ... */ }
```

Creates a span named `my_function` at `INFO` level with `param` recorded as a field.

---

## Attribute Parameters

### `name`

Override the span name (defaults to the function name).

```rust
#[tracing::instrument(name = "user_store::get")]
async fn get(&self, id: Uuid) -> Result<User> { /* ... */ }
```

**When to use:**
- Trait impl methods where the function name is generic (`get`, `new`, `run`)
- Disambiguating overloaded names across modules
- When the function name doesn't reflect the logical operation

### `level`

Set the span's verbosity level. Defaults to `INFO`.

```rust
#[tracing::instrument(level = "debug")]
fn internal_helper(data: &[u8]) -> usize { /* ... */ }
```

Valid values: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`

Also accepts `tracing::Level` constants:
```rust
#[tracing::instrument(level = tracing::Level::DEBUG)]
```

### `target`

Override the span's target (defaults to the module path). Used for subscriber filtering.

```rust
#[tracing::instrument(target = "database")]
async fn query_users(db: &Pool) -> Result<Vec<User>> { /* ... */ }
```

Useful when you want to filter spans by logical subsystem rather than Rust module path:
```rust
// In subscriber config: filter database spans specifically
// RUST_LOG=database=debug,info
```

### `skip`

Exclude specific parameters from the span's recorded fields.

```rust
#[tracing::instrument(skip(db, password))]
async fn login(db: &Pool, username: &str, password: &str) -> Result<Token> { /* ... */ }
```

The skipped parameters are not recorded at all — they don't appear in logs.

**Always skip:**
- Types that don't implement `Debug` (compilation error otherwise)
- Connection pools, HTTP clients, large state objects
- Sensitive data: passwords, tokens, API keys, PII

### `skip_all`

Exclude all parameters. Combine with `fields(...)` to selectively expose specific values.

```rust
#[tracing::instrument(skip_all, fields(user_id = %user_id))]
async fn update_profile(db: &Pool, user_id: Uuid, body: LargePayload) -> Result<()> { /* ... */ }
```

**When to use:**
- Most parameters are non-displayable or sensitive
- Function has many parameters and only 1-2 are diagnostically useful
- Handling request bodies or other large types

### `fields`

Add custom fields to the span. Fields can reference parameters, constants, or be declared empty for later recording.

```rust
#[tracing::instrument(
    skip(db),
    fields(
        user_id = %req.user_id,          // Display formatting
        request_type = ?req.kind,         // Debug formatting
        email = tracing::field::Empty,    // Filled later
        operation = "create_user",        // Constant string
        attempt_count = 0,               // Constant number
    )
)]
```

#### Field formatting prefixes

| Prefix | Trait | Output | Use for |
|--------|-------|--------|---------|
| `%` | `Display` | Clean, human-readable | IDs, strings, numbers, types with good Display |
| `?` | `Debug` | Full structure dump | Complex types when structure matters for debugging |
| (none) | `Value` | Native value | Strings, numbers, bools — recorded as-is |

#### Recording empty fields later

```rust
#[tracing::instrument(fields(result_count = tracing::field::Empty))]
async fn search(query: &str) -> Result<Vec<Item>> {
    let results = db.search(query).await?;
    tracing::Span::current().record("result_count", results.len());
    Ok(results)
}
```

### `err`

Log the error when the function returns `Err`. Default level: `ERROR`.

```rust
// Default: logs at ERROR with Debug formatting
#[tracing::instrument(err)]

// Custom level
#[tracing::instrument(err(level = "warn"))]

// Display formatting (cleaner for thiserror types)
#[tracing::instrument(err(Display))]

// Both custom level and formatting
#[tracing::instrument(err(level = "warn", Display))]
```

**How it works:** When the function returns `Err(e)`, the span records an event with the error value. The error is formatted using the specified trait (`Debug` by default, `Display` if specified).

**Interaction with `?` operator:** `err` fires on the final `Err` return from the function, not on each `?`. So if you have:
```rust
#[tracing::instrument(err)]
async fn process() -> Result<()> {
    step_one()?;   // if this fails, err fires with step_one's error
    step_two()?;   // if this fails, err fires with step_two's error
    Ok(())
}
```

### `ret`

Log the return value when the function returns `Ok` (or any value for non-Result functions). Default level: `INFO`.

```rust
// Default: logs at INFO with Debug formatting
#[tracing::instrument(ret)]

// Custom level (recommended — return values are usually debug-level)
#[tracing::instrument(ret(level = "debug"))]

// Display formatting
#[tracing::instrument(ret(Display))]
```

**Warning:** `ret` uses `Debug` by default. For types with many fields, this creates very large log lines. Prefer `ret(level = "debug")` at minimum.

### `err` and `ret` combined

```rust
#[tracing::instrument(err, ret(level = "debug"))]
async fn process(input: &str) -> Result<Output> { /* ... */ }
```

- On `Ok(value)`: logs the value at DEBUG
- On `Err(error)`: logs the error at ERROR

---

## Full Example: All Attributes Together

```rust
#[tracing::instrument(
    name = "order_service::process_payment",
    level = "info",
    target = "payments",
    skip(db, payment_client),
    fields(
        order_id = %order.id,
        amount = %order.total,
        currency = %order.currency,
        payment_method = tracing::field::Empty,
    ),
    err(Display),
    ret(level = "debug"),
)]
async fn process_payment(
    db: &Pool,
    payment_client: &PaymentClient,
    order: &Order,
) -> Result<PaymentReceipt> {
    let method = determine_payment_method(order).await?;
    tracing::Span::current().record("payment_method", tracing::field::display(&method));

    let receipt = payment_client
        .charge(order, &method)
        .await
        .context("charging payment")?;

    Ok(receipt)
}
```

---

## Manual Span Creation

For cases where `#[instrument]` doesn't apply (closures, conditionals, spawned tasks).

### `info_span!` / `debug_span!` / `trace_span!`

```rust
let span = tracing::info_span!("process_batch", batch_size = items.len());
let _guard = span.enter();
// ... work happens within the span
// span closes when _guard is dropped
```

### Async-compatible entry with `.entered()`

```rust
// For sync code or code that won't cross .await points
let _guard = tracing::info_span!("sync_work", key = %key).entered();
```

**Warning:** Do NOT hold `.entered()` guards across `.await` points. Use `.instrument()` instead.

### `.instrument()` for futures

```rust
use tracing::Instrument;

let span = tracing::info_span!("background_task", task_id = %id);
tokio::spawn(
    async move {
        do_work().await;
    }
    .instrument(span)
);
```

### `.in_current_span()` shorthand

```rust
use tracing::Instrument;

// Spawned task inherits the current span as parent
tokio::spawn(
    async move { process(data).await }
        .in_current_span()
);
```

---

## Span Events

Events are point-in-time occurrences within a span. Use sparingly — only for genuine business events.

```rust
// Structured events with fields
tracing::info!(order_id = %id, amount = %total, "payment processed");
tracing::warn!(retries = attempts, "retrying after transient failure");
tracing::error!(error = %e, "unrecoverable failure");

// Event with explicit target
tracing::info!(target: "audit", user_id = %uid, action = "login", "user authenticated");

// Debug-level events (filtered out in production)
tracing::debug!(cache_hit = true, key = %key, "cache lookup");
```

### Structured field syntax

```rust
// Named fields with formatting
tracing::info!(
    user_id = %user.id,        // Display
    request = ?req,             // Debug
    latency_ms = elapsed,      // Value (no prefix for primitives)
    "request completed"        // Message is always the last argument
);
```

---

## `Span::current()` Recipes

### Record a field declared as Empty

```rust
tracing::Span::current().record("email", user.email.as_str());
```

### Check if the current span is disabled (performance optimization)

```rust
if tracing::Span::current().is_disabled() {
    // Skip expensive field computation
    return;
}
// ... compute expensive diagnostic value
tracing::Span::current().record("expensive_field", computed_value);
```

### Add a follow-from relationship

```rust
let processing_span = tracing::info_span!("process_message");
processing_span.follows_from(producer_span);
```

---

## Common Type Patterns

### Types that need `skip`

| Type | Why | Pattern |
|------|-----|---------|
| `&Pool` / `PgPool` | No useful Debug, large | `skip(db)` |
| `&Client` (reqwest, etc.) | No useful Debug | `skip(client)` |
| `&State<T>` / `AppState` | Usually large, may contain secrets | `skip(state)` or `skip_all` |
| `Json<T>` / `Body` | Large request payloads | `skip(body)`, selectively expose with `fields` |
| `String` passwords/tokens | Sensitive | `skip(password)` |
| `impl Stream` / `impl Future` | Not Debug | `skip(stream)` |
| `&[u8]` large buffers | Huge Debug output | `skip(buffer)`, maybe `fields(buffer_len = buffer.len())` |

### Types that are fine to include

| Type | Notes |
|------|-------|
| `Uuid` | Concise Display and Debug |
| `i32`, `u64`, etc. | Primitive, always fine |
| `&str`, `String` (non-sensitive) | Fine for IDs, names, paths |
| `bool` | Trivial |
| Small enums | If Debug is concise |

---

## Compilation Requirements

- All non-skipped parameters must implement `Debug` (or the function won't compile)
- `err` requires the error type to implement `Debug` (or `Display` if `err(Display)`)
- `ret` requires the return type to implement `Debug` (or `Display` if `ret(Display)`)
- For `Result<T, E>`: `err` needs `E: Debug`, `ret` needs `T: Debug`
