# Span Strategy: When and How to Instrument

Decision guide for choosing the right instrumentation approach for each function type. Modeled after the async/unsafe documentation decision framework.

---

## Decision Framework

For each function, ask:

1. **Does it cross a boundary?** — Network, filesystem, database, external service, IPC
2. **Does it mutate significant state?** — State machines, cache invalidation, permission changes
3. **Is failure meaningful?** — Would an operator need to know this specific function failed?
4. **Is it called per-request?** — Or is it called many times per request?

- **Yes to 1, 2, or 3** → Instrument with `#[instrument]`
- **Yes to 4 but no to 1-3** → Instrument at `level = "debug"` or skip entirely
- **No to all** → Do not instrument

---

## By Function Category

### HTTP/gRPC Handlers

**Instrument:** Yes, but check for existing middleware spans first.

```rust
// If tower-http TraceLayer is already active, the request span exists.
// Add #[instrument] only for handler-specific fields:
#[tracing::instrument(skip_all, fields(user_id))]
async fn get_user(
    State(db): State<Pool>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    // ...
}

// Without TraceLayer, instrument normally:
#[tracing::instrument(skip(db), err)]
async fn get_user(db: &Pool, user_id: Uuid) -> Result<User> { /* ... */ }
```

**Level:** INFO (default) — one span per incoming request.

### Database Operations

**Instrument:** Yes — these are boundary-crossing I/O.

```rust
#[tracing::instrument(skip(pool), err)]
async fn fetch_user(pool: &PgPool, user_id: Uuid) -> Result<User> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
        .context("fetching user by id")
}
```

**Level:** INFO for primary queries called once per request. DEBUG for secondary/supplementary queries.

**Skip:** Always skip the connection pool. Consider `fields(table = "users")` if the function name doesn't convey the table.

### External Service Calls

**Instrument:** Always — external calls are the most important spans for debugging latency and failures.

```rust
#[tracing::instrument(
    skip(client),
    fields(service = "payment_gateway", endpoint = "/charge"),
    err,
)]
async fn charge_payment(
    client: &PaymentClient,
    order_id: Uuid,
    amount: Decimal,
) -> Result<Receipt> {
    client.post("/charge")
        .json(&ChargeRequest { order_id, amount })
        .send()
        .await
        .context("charging payment gateway")
}
```

**Level:** INFO — always want visibility into external calls.

### Background Tasks / Workers

**Instrument:** Yes — especially important for async tasks that outlive request scopes.

```rust
#[tracing::instrument(skip_all, fields(worker_id = %id, queue = %queue_name))]
async fn run_worker(id: u32, queue_name: &str, pool: &Pool) -> Result<()> {
    loop {
        let job = fetch_next_job(pool, queue_name).await?;
        process_job(pool, &job).await
            .context("processing job")?;
    }
}
```

**Level:** INFO for the worker span itself. Individual job processing can be DEBUG if high-volume.

### Spawned Tasks

**Instrument:** Use `.instrument()` or `.in_current_span()` — not `#[instrument]`.

```rust
use tracing::Instrument;

// New span for independent work
let span = tracing::info_span!("email_notification", user_id = %user_id);
tokio::spawn(
    async move {
        send_notification(user_id).await;
    }
    .instrument(span)
);

// Inherit current span for dependent work
tokio::spawn(
    async move {
        update_cache(key, value).await;
    }
    .in_current_span()
);
```

### Validation / Parsing Functions

**Instrument:** Only if the validation is complex and can fail in non-obvious ways.

```rust
// Skip: simple validation, failure is clear from the error
fn validate_email(email: &str) -> Result<Email> {
    Email::parse(email).context("invalid email format")
}

// Instrument: complex multi-step validation with external lookups
#[tracing::instrument(skip(db), level = "debug", err)]
async fn validate_order(db: &Pool, order: &CreateOrder) -> Result<ValidatedOrder> {
    let user = db.fetch_user(order.user_id).await.context("checking user exists")?;
    let inventory = check_inventory(db, &order.items).await.context("checking inventory")?;
    // ... multiple validation steps
    Ok(ValidatedOrder { user, inventory, .. })
}
```

### Data Transformation / Pure Functions

**Instrument:** No — unless the transformation is expensive and you need to measure its duration.

```rust
// Do NOT instrument
fn to_response(user: User, prefs: Preferences) -> UserResponse {
    UserResponse {
        name: user.name,
        theme: prefs.theme,
    }
}

// Do NOT instrument — hot path
fn calculate_hash(data: &[u8]) -> u64 {
    // called per-packet
}

// Maybe instrument at TRACE if you need duration measurement
#[tracing::instrument(level = "trace", skip_all, fields(input_size = data.len()))]
fn compress(data: &[u8]) -> Vec<u8> {
    // expensive but pure
}
```

### Startup / Initialization

**Instrument:** Yes — startup failures are critical to diagnose.

```rust
#[tracing::instrument(err)]
async fn initialize_app(config_path: &str) -> Result<AppState> {
    let config = load_config(config_path).context("loading config")?;
    let db = connect_database(&config.database_url).await.context("connecting to database")?;
    let cache = connect_cache(&config.redis_url).await.context("connecting to cache")?;
    Ok(AppState { config, db, cache })
}
```

**Level:** INFO — startup happens once, always worth seeing.

### Trait Implementations

**Instrument:** On the `impl`, not the trait definition.

```rust
impl UserRepository for PgUserRepository {
    #[tracing::instrument(
        name = "UserRepository::find_by_email",
        skip(self),
        err,
    )]
    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        // ...
    }
}
```

Use `name = "Trait::method"` to make spans readable when multiple impls exist.

### Closures and Iterators

**Instrument:** Cannot use `#[instrument]`. Use manual spans only if the closure does I/O.

```rust
// Skip: pure transformation
let names: Vec<_> = users.iter().map(|u| u.name.clone()).collect();

// Manual span: closure does I/O
let results: Vec<Result<Response>> = urls.iter().map(|url| {
    let _span = tracing::info_span!("fetch_url", url = %url).entered();
    client.get(url).send()
}).collect();

// Better: use a proper async function and instrument that
#[tracing::instrument(skip(client), err)]
async fn fetch_url(client: &Client, url: &str) -> Result<Response> {
    client.get(url).send().await.context("fetching url")
}
```

### Middleware / Tower Layers

**Instrument:** Usually the middleware itself creates spans. Don't double-span.

```rust
// tower-http TraceLayer already creates request spans.
// Only add custom middleware spans for cross-cutting concerns:
use tower::Layer;

// Example: rate limiting middleware with its own span
let _span = tracing::info_span!(
    "rate_limit",
    client_ip = %ip,
    remaining = tracing::field::Empty,
).entered();
// ... check rate limit
tracing::Span::current().record("remaining", remaining);
```

---

## Level Selection Decision Tree

```
Is it an error/failure?
  → ERROR (default for `err`)
  → WARN if it's expected/recoverable (cache miss, rate limit hit)

Is it a boundary-crossing call? (DB, network, filesystem)
  → INFO (default)

Is it called once per request?
  → INFO

Is it called multiple times per request?
  → DEBUG

Is it called in a tight loop?
  → TRACE (or don't instrument at all)

Is it startup/shutdown?
  → INFO
```

---

## Error Logging Placement

### The Boundary Rule

Log errors at the **outermost boundary** where the error becomes actionable, not at every intermediate step.

```
Handler (log here with err)       ← ERROR logged once, with full span context
  └── Service layer (propagate)   ← .context("doing X"), no err flag
       └── Repository (propagate) ← .context("querying Y"), no err flag
            └── SQL query fails   ← the original error
```

```rust
// Handler: the boundary — use err
#[tracing::instrument(skip_all, fields(user_id), err)]
async fn get_user_handler(...) -> Result<Json<User>, AppError> { ... }

// Service: propagates — no err
#[tracing::instrument(skip(db))]
async fn get_user_profile(db: &Pool, user_id: Uuid) -> Result<Profile> {
    let user = fetch_user(db, user_id).await.context("fetching user")?;
    let prefs = fetch_prefs(db, user_id).await.context("fetching preferences")?;
    Ok(Profile { user, prefs })
}

// Repository: propagates — no err
#[tracing::instrument(skip(pool))]
async fn fetch_user(pool: &PgPool, id: Uuid) -> Result<User> {
    sqlx::query_as!(...)
        .fetch_one(pool)
        .await
        .context("querying users table")
}
```

The resulting error log contains the full chain:
```
ERROR get_user_handler{user_id=abc}: fetching preferences: querying preferences table: connection refused
```

### Exceptions to the Boundary Rule

Use `err` on inner functions when:
- The function is also called directly (not only through the handler)
- The function handles retries and you want to log each attempt
- The error is silently recovered and the caller never sees it

---

## Field Strategy

### What to put on spans (structured fields)

Fields on spans are **indexed and searchable** in observability backends (Datadog, Grafana, Jaeger). Use them for values you'll filter or group by:

- User IDs, request IDs, trace IDs
- Resource identifiers (order_id, product_id)
- Operation types or categories
- Counts and sizes (batch_size, result_count)

### What to put in messages (unstructured text)

Messages are the human-readable part of events. Use them for:

- Business event descriptions ("order processed", "payment charged")
- Error context that doesn't need indexing

### What to skip entirely

- Large payloads (request/response bodies)
- Redundant context (parent span already has it)
- Internal implementation details (intermediate variable values)
- Anything sensitive (PII, credentials, tokens)

---

## Performance Notes

Span creation has measurable overhead. In microbenchmarks:
- Creating and entering a span: ~200-500ns
- Recording a field: ~50-100ns
- Disabled span (filtered out by subscriber): ~10-20ns

**Guidelines:**
- Functions called < 1000 times/sec: instrument freely at INFO
- Functions called 1000-100k times/sec: instrument at DEBUG/TRACE (disabled in production)
- Functions called > 100k times/sec: do not instrument, or use `tracing::Span::current().is_disabled()` to gate expensive field computation

When in doubt, instrument and measure. The subscriber's level filter eliminates overhead for disabled spans almost entirely.
