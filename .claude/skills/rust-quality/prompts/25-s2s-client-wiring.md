# Service-to-Service Client Wiring

Cross-reference how each service constructs its `*-client` dependencies
(`auth-client`, `widget-client`, `order-client`, `account-client`,
`invite-client`, …) in `main.rs` against the canonical pattern. Flag clients
built without the standard HTTP layer (request-id, tracing, GCP OIDC), without
a dedicated `*_http` config, or without a fail-fast audience guard.

## Why

A `*-client` calls another service's OIDC-gated `/internal/*` endpoints. Three
things must hold or the call silently breaks or goes dark:

1. **OIDC audience** — the outbound HTTP client must attach a GCP OIDC token
   whose audience is the *callee's* Cloud Run URL, or the callee's
   `internal_auth_layer` returns `401` on every request.
2. **Observability** — the client must carry request-id propagation and
   tracing, so the hop appears in distributed traces and logs correlate across
   services. A client built from raw config without these is invisible.
3. **Fail-fast** — the audience (`cloud_run_url`) must be *required* at
   startup. A missing env var should refuse to boot, not silently produce a
   no-OIDC client that 401s in production.

## Canonical pattern

In `services/<name>/src/main.rs` (reference: `services/example/src/main.rs` for
the auth + widget clients; any service wiring an order + account client
follows the identical shape):

```rust
let <svc>_audience = config
  .<svc>_client
  .cloud_run_url
  .as_ref()
  .ok_or("<SVC>_CLIENT__CLOUD_RUN_URL is required for service-to-service auth")?;
let <svc>_http = HttpClientBuilder::with_config(config.<svc>_client_http)
  .with_request_id()
  .with_tracing()
  .with_gcp_oidc(<svc>_audience)
  .build()?;
let <svc>_client = Arc::new(
  <svc>_client::<Svc>ClientBuilder::from_config(&config.<svc>_client)
    .with_http_client(<svc>_http)
    .build()
    .map_err(|e| format!("failed to build <svc> client: {e}"))?,
);
```

**Config (`config.rs`)** — TWO fields per client:
- `<svc>_client: <Svc>ClientConfig` (carries `base_url` + `cloud_run_url`).
- a *dedicated* `<svc>_client_http: HttpClientConfig` with `#[serde(default)]
  #[garde(dive)]`. Do **not** reuse another client's `*_http` config, and do
  **not** reuse the JWKS-validator `auth_http` for the `auth-client`.

**Infra (`infra/gcp/environments/*/main.tf`)** — the service's `env_vars`
merge sets:
- `{ for k, v in local.http_defaults : "<SVC>_CLIENT_HTTP__${k}" => v }`
- `<SVC>_CLIENT__BASE_URL = module.<svc>_service.uri`
- `<SVC>_CLIENT__CLOUD_RUN_URL = format(local.cloud_run_base_url, "<svc>")`

## Anti-patterns (each is a finding)

- **`.with_http_config(...)` on the client builder** instead of building the
  HTTP client explicitly and passing `.with_http_client(...)`. The client
  builder's internal config path attaches OIDC *only if* `cloud_run_url` is
  set, but it does **not** add `with_request_id`/`with_tracing` — so the call
  loses request correlation and traces — and there is no fail-fast guard.
- **Reusing `auth_http`** (the JWKS-fetch client used by JWT validation) for
  the `auth-client`. They are distinct clients; the auth-client needs its own
  `auth_client_http`.
- **Missing `.ok_or(...)` audience guard** → a silent no-OIDC client that 401s
  at runtime instead of refusing to start.
- **Building the HTTP client without `.with_request_id()` / `.with_tracing()`.**
- **Wrong audience** — `with_gcp_oidc` passed the caller's own URL instead of
  the callee's.

## Workflow

1. Enumerate `*-client` path deps in each `services/<name>/Cargo.toml`.
2. For each, find its construction in `main.rs`. Confirm: explicit
   `HttpClientBuilder` + `with_request_id` + `with_tracing` +
   `with_gcp_oidc(<callee>_audience)` + `.with_http_client(...)`, with the
   audience resolved via `.ok_or(...)`.
3. Confirm `config.rs` declares both `<svc>_client` and a dedicated
   `<svc>_client_http`.
4. Confirm infra sets `<SVC>_CLIENT_HTTP__*`, `<SVC>_CLIENT__BASE_URL`, and
   `<SVC>_CLIENT__CLOUD_RUN_URL` in every environment.
5. Flag any deviation; cross-check against `services/example` as the reference.

## Fixing

Rewrite to the canonical pattern; add the dedicated `*_http` config field and
the infra env mapping. Then:

```bash
cargo build -p <service> --all-targets --all-features
cargo clippy -p <service> --all-targets --all-features
# in the infra repo, per environment:
terraform fmt && terraform validate
```

Do **not** `terraform apply` — that is a gated human step.

## Report format

Per service, a row per `*-client` with ✅/❌ on: explicit HTTP client,
`request_id`, `tracing`, `gcp_oidc`, correct audience, dedicated `*_http`
config, startup audience guard, and the three infra env vars. End with any
client appearing inconsistent across 2+ services (a shared gap worth a helper
in `service-builder`/`http-client`).
