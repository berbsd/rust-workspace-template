# Service Skeleton Consistency

Verify that `main.rs`, `config.rs`, and the metrics module follow the same shape across every service. Domain logic and routes will differ — the bootstrap, configuration, and metric-registration scaffolding must not.

## Why

Skeleton drift is the leading cause of bugs that exist in one service but silently don't in another: a missing tracing init layer, a different config-loading helper, a metrics counter registered in one service and not in others. Auditing periodically keeps services interchangeable from an operational perspective.

## Scope

For every directory under `services/`, compare:

- `services/<name>/src/main.rs`
- `services/<name>/src/config.rs`
- `services/<name>/src/metrics.rs` (or `src/metrics/mod.rs` / wherever metric definitions live)

If a service is missing one of these files, that's the first finding.

## Workflow

### 1. Pick a reference service

Identify the service with the most recently-touched, most complete skeleton. That becomes the reference shape. (Don't invent a new ideal — use what already works in the codebase.)

### 2. Diff each other service against the reference

For each of the three files, compare structurally — not line-for-line. The aim is shape parity, not character parity.

#### `main.rs` checklist

The bootstrap sequence should be identical in order and helpers used. In this template's
own shape (`services/example`), that's:

1. `#[tokio::main]` signature.
2. Tracing/subscriber initialization — same crate, same layers, same env-filter source.
3. Config read the same way — same env vars, same defaults, same fallback order.
4. Postgres pool construction and `sqlx::migrate!` — same options (`max_connections`, etc.).
5. Router construction via the crate's own `router(pool)` function, never assembled inline
   in `main`.
6. Shutdown signal wiring — same source (`tokio::signal::ctrl_c`, SIGTERM handling, etc.).
7. `axum::serve(...).await?` returns `()` from `main`.

If the workspace grows a shared bootstrap crate later, update this list to match it —
whatever the actual convention is, every service's `main.rs` should follow it identically.
Flag deviations: a service initializing tracing differently, building the router inline
instead of via its own `router(pool)` function, or skipping graceful shutdown wiring.

#### `config.rs` checklist

- Root struct named `Config`, derives `Clone, Debug, Deserialize, Validate`.
- Every leaf field carries an `/// Env: `VAR` (default … | required)` line, and every nested-config field an `/// Env prefix:` line — flag missing lines, and flag any surviving `# Environment Variables` table on the struct, which this convention replaces (a table on the struct outlives the fields it describes; a `///` line on each field can't drift from it).
- Fields use `#[serde(rename = "...")]` to define env var prefixes consistently with other services — whatever prefix convention this workspace's `Config` types already use (e.g. `service`, `postgres`, and any other shared sub-config).
- Shared sub-configs (`ServiceConfig`, `PostgresConfig`, `HttpClientConfig`, etc.) come from the workspace crates listed in CLAUDE.md — not redefined locally.
- Every field carries `#[garde(...)]` (or `#[garde(skip)]` with justification) — no silent omissions.
- No service-specific override of `Default` for shared types — defaults live in the owning crate.

Flag any locally redefined struct that has the same shape as a workspace-shared one.

#### Metrics module checklist

- Same crate used for metrics across services (`metrics`, `prometheus`, etc. — only one).
- Metric names use the same prefix convention (e.g. `<service>_<noun>_<unit>` — read the reference and match).
- Counter / histogram / gauge registration happens at startup in one place, not scattered.
- Same set of "platform" metrics present in every service (request counts, error counts, latency histograms — whatever the reference defines).
- Custom domain metrics are clearly separated from platform metrics (in submodules or with a comment fence).
- Histogram buckets follow the workspace standard (don't redefine bucket sets per service).

Flag services missing platform metrics, using a different prefix scheme, or registering metrics inline in handlers instead of in the metrics module.

### 3. Report drift, don't auto-fix the reference

Skeleton consistency fixes touch every service. Don't apply them silently — produce the drift report, then ask the user which direction to converge:

- "Bring all services up to the reference shape" (most common — propagate the best version).
- "Update the reference to match the others" (rare — only if the reference is what's actually wrong).
- "Extract the shared shape into `common-types` or a new helper crate" (when the duplication itself is the smell).

## Verification

After fixes:

```bash
cargo check --workspace
cargo clippy --workspace --no-deps --all-targets
cargo test --workspace
```

Run each affected service locally (`just run <name>`) to confirm startup logs, metrics endpoint, and config loading still work.

## Report format

Produce a matrix view — services × skeleton concerns:

```
              | main.rs       | config.rs                | metrics.rs
example       | reference     | reference                | reference
service-b     | tracing init  | redefines PostgresConfig | wrong prefix
service-c     | no shutdown   | OK                       | missing platform metrics
```

Then list each cell's drift in detail with file:line. End with a recommendation on convergence direction.
