# Missing Metrics

Cross-reference each service's `metrics.rs` against its actual boundary functions. Flag operations that should be observable but aren't, plus naming drift across services.

## Why

Metrics are the cheapest way to catch a regression: a counter or histogram that doesn't fire for the new code path is a silent failure. This template's intended pattern is one `metrics.rs` per service that registers every metric the service emits, plus standard names so dashboards and alerts can be written generically — but as of this writing `services/example` has not adopted it, so the first run of this check on an unmodified template produces exactly one finding (see the Report format example below), not a multi-service drift report.

## Conventions (recap)

From the workspace naming contract:

- Metric names are prefixed with the **singular crate name** — `project_create_total`, not `projects_create_total`.
- Counters: `{prefix}_{operation}_total` for success, `{prefix}_{operation}_failed_total` (or matching) for failure.
- Histograms: `{prefix}_{operation}_duration_seconds`.
- All registration lives in `services/<name>/src/metrics.rs` (or `metrics/mod.rs`). Inline registration scattered through handlers/services is a finding.
- No metrics crate is wired into this template yet. If the workspace has picked one, every service must use the same one — flag any service pulling in a second (`prometheus`, `opentelemetry`, hand-rolled atomics in handlers) once a first choice exists. If none is chosen yet, choosing one is out of scope for this check; flag it as a prerequisite instead of picking on the workspace's behalf.

## Workflow

### Step 1: Build the per-service metric inventory

For each `services/<name>/`:

1. Read `metrics.rs`. Extract every metric name registered (counters, histograms, gauges).
2. Note the prefix used. Confirm it matches the singular crate name.
3. Note any helper functions (e.g. `record_create_success(...)`) and which metrics they touch.

If a service is missing `metrics.rs` entirely, that's the first finding — fall back to grepping for `metrics::counter!` / `metrics::histogram!` macro calls inline (which is itself a finding under the centralization rule).

### Step 2: Identify boundary operations that should be measured

For each service, enumerate the operations that warrant metrics. The general standard is:

| Surface                           | Counter (success)                   | Counter (failure)                          | Histogram                          |
|-----------------------------------|--------------------------------------|--------------------------------------------|------------------------------------|
| HTTP handler                      | `{prefix}_request_total{path,method,status}` (often a tower layer, shared across handlers) | — same metric, status label captures failure | `{prefix}_request_duration_seconds` |
| Service mutation (create/update/delete) | `{prefix}_{operation}_total`   | `{prefix}_{operation}_failed_total{reason}` | `{prefix}_{operation}_duration_seconds` (when latency matters) |
| External client call (HTTP, gRPC, object storage, message queue publish) | `{prefix}_{client}_request_total` | `{prefix}_{client}_request_failed_total{reason}` | `{prefix}_{client}_request_duration_seconds` |
| Message queue subscriber          | `{prefix}_event_received_total{event}` | `{prefix}_event_failed_total{event,reason}` | `{prefix}_event_duration_seconds{event}` |
| Background job (cleanup, sync)    | `{prefix}_{job}_run_total`           | `{prefix}_{job}_run_failed_total`          | `{prefix}_{job}_duration_seconds` |
| Cache lookup                      | gauge or counter `{prefix}_cache_hit_total` / `_miss_total` | — | — |

If a shared crate already provides one of these (e.g. HTTP request metrics via a tower layer), don't duplicate it; instead confirm the layer is wired into the service's router. This template has no such shared crate today — each service wires its own metrics directly (see AGENTS.md).

### Step 3: Cross-reference

For each operation in Step 2:

- Does the corresponding metric name exist in the service's `metrics.rs`?
- Does the operation's code site actually *increment* the counter / observe the histogram? Registration without recording is dead.
- Does the failure path also record? Common bug: the success path increments `..._total`, the failure path `?`-propagates without touching the failure counter.

### Step 4: Flag drift across services

After per-service findings, compare:

- Same operation named differently across services? (`auth_token_total` vs `auth_token_minted_total` for what is semantically the same event.) Suggest unification.
- Histogram bucket overrides — if one service customises buckets and the rest use defaults, dashboards become per-service. Either standardise the override into the workspace metrics helper or remove the local override.
- Label cardinality — a counter labelled by `user_id` will explode the metric store. Flag any label that is not bounded (`user_id`, `project_id`, free-form error strings). Use bounded labels: `status`, `reason`, `event_type`, `path` (path templates, not URLs with IDs).

### Step 5: Consult related skills

- If this workspace uses structured-event constants (`<type>.<action>.<status>`), each `.failed` event in that taxonomy should have a matching failure counter. Flag mismatches.
- Prompt #14 (structured logging) covers tracing instrumentation on handlers; if a handler is instrumented but the metric is missing, the gap is here, not there.
- Skill #15 (skeleton consistency) flags drift in *what* metrics module looks like; this skill flags drift in *which metrics exist*. They complement each other — do not duplicate findings.

## Fixing

For each missing metric:

1. **Register** in the service's `metrics.rs` with the correct prefix and type.
2. **Record** at the operation's call site — increment the counter on the relevant path; observe the histogram around the I/O call.
3. **Verify** failure paths also record by reading the function end-to-end, not just the happy path.
4. **Update** the per-service env-vars / metrics doc if one exists, so operators know the new metric is available.

For naming drift across services, propose the canonical name and apply it consistently — but only after user confirmation, since metric renames break existing dashboards and alerts.

## Verification

```bash
cargo check --workspace
cargo clippy --workspace --no-deps --all-targets
just run <service>  # then curl http://localhost:PORT/metrics and grep for the new names
```

The Prometheus `/metrics` endpoint must show the registered metric names — if a metric is registered but never recorded, it won't appear (Prometheus omits zero-valued counters that haven't been touched).

## Report format

Per-service table:

```
services/example
  Registered metrics:    none — no metrics.rs (or metrics/mod.rs) exists
  Finding: create/list/delete handlers in feature/widget/ have no counters or
    histograms; the service is currently unobservable. Establishing metrics.rs
    is a prerequisite for the rest of this check, not something to fix inline
    here — flag it and stop for this service.
```

Once a service has an established `metrics.rs`, per-service findings and
cross-service drift look like this (illustrative — substitute the workspace's
actual service/metric names):

```
services/widget-service
  Registered metrics:    widget_created_total, widget_validated_total, widget_request_duration_seconds
  Missing:
    - widget_deleted_total          (delete handler at handler/delete.rs:45 has no counter)
    - widget_upstream_request_failed_total (client failures unobserved)
  Naming drift:
    - "widget_created_total" vs "create_widget_total" elsewhere — pick a canonical form
```

End with a workspace summary listing operations missing metrics in 2+ services (likely a gap worth solving once, in a shared crate, if the workspace has grown one — see AGENTS.md's guidance on when a shared crate is warranted).
