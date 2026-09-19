# Service-to-Service Client Wiring

Skip this check entirely if this workspace has only one service, or if its
services don't call each other over HTTP — `services/example` in this
template has no peer service to call, so this check has nothing to inspect
here yet. Once a second service exists and the first grows a typed client for
it, apply this check to how that client is constructed in `main.rs`.

Cross-reference how each service constructs its `*-client` dependencies in
`main.rs` against a single canonical pattern. Flag clients built without the
standard HTTP layer (request-id, tracing, service-to-service auth), without a
dedicated `*_http` config, or without a fail-fast guard on required config.

## Why

A `*-client` calls another service's internally-gated endpoints. Three things
must hold or the call silently breaks or goes dark:

1. **Service-to-service auth** — the outbound HTTP client must attach
   whatever credential the callee's internal-auth layer expects (a signed
   token scoped to the callee's identity, mTLS, a shared secret, etc.), or
   every request is rejected.
2. **Observability** — the client must carry request-id propagation and
   tracing, so the hop appears in distributed traces and logs correlate
   across services. A client built from raw config without these is
   invisible.
3. **Fail-fast** — whatever identifies the callee (its base URL, its expected
   audience, its credential) must be *required* at startup. A missing env var
   should refuse to boot, not silently produce a client that fails on first
   use in production.

## Canonical pattern

In `services/<name>/src/main.rs`, look for one consistent shape across every
`*-client` construction:

```rust
let <svc>_audience = config
  .<svc>_client
  .callee_identity
  .as_ref()
  .ok_or("<SVC>_CLIENT__CALLEE_IDENTITY is required for service-to-service auth")?;
let <svc>_http = HttpClientBuilder::with_config(config.<svc>_client_http)
  .with_request_id()
  .with_tracing()
  .with_service_auth(<svc>_audience)
  .build()?;
let <svc>_client = Arc::new(
  <svc>_client::<Svc>ClientBuilder::from_config(&config.<svc>_client)
    .with_http_client(<svc>_http)
    .build()
    .map_err(|e| format!("failed to build <svc> client: {e}"))?,
);
```

**Config (`config.rs`)** — TWO fields per client:
- `<svc>_client: <Svc>ClientConfig` (carries the callee's `base_url` plus
  whatever identifies it to the auth mechanism in use).
- a *dedicated* `<svc>_client_http: HttpClientConfig` with `#[serde(default)]
  #[garde(dive)]`. Do **not** reuse another client's `*_http` config, and do
  **not** reuse a validator client's HTTP config (e.g. a JWKS-fetch client)
  for an outbound service client — they are distinct clients with distinct
  lifecycles.

**Infra (if this workspace provisions services via infra-as-code)** — the
service's environment should set the callee's base URL and auth identity from
the same source the infra module uses to name/deploy the callee, so the two
never drift independently.

## Anti-patterns (each is a finding)

- **A config-driven shortcut on the client builder** that only conditionally
  attaches auth (e.g. "if this field is set") instead of building the HTTP
  client explicitly and passing it in — that shortcut path typically skips
  `with_request_id`/`with_tracing` too, and has no fail-fast guard.
- **Reusing another client's HTTP config** (e.g. a JWKS-fetch client's config)
  for an unrelated outbound client. They are distinct clients; each needs its
  own `*_client_http`.
- **Missing `.ok_or(...)` guard** on the callee's identity/audience → a client
  that silently fails at runtime instead of refusing to start.
- **Building the HTTP client without `.with_request_id()` / `.with_tracing()`.**
- **Wrong identity** — the client authenticates as itself instead of
  asserting the callee's expected audience/identity.

## Workflow

1. Enumerate `*-client` path deps in each `services/<name>/Cargo.toml`.
2. For each, find its construction in `main.rs`. Confirm: explicit
   `HttpClientBuilder` + `with_request_id` + `with_tracing` + the
   service-to-service auth call + `.with_http_client(...)`, with the callee's
   identity resolved via `.ok_or(...)`.
3. Confirm `config.rs` declares both `<svc>_client` and a dedicated
   `<svc>_client_http`.
4. If infra-as-code exists, confirm it sets the client's HTTP defaults, base
   URL, and auth identity per environment from the same source of truth as
   the callee's own deployment.
5. Flag any deviation; cross-check against whichever service in the workspace
   already has this wired correctly, as the reference.

## Fixing

Rewrite to the canonical pattern; add the dedicated `*_http` config field and
the infra env mapping if applicable. Then:

```bash
cargo build -p <service> --all-targets --all-features
cargo clippy -p <service> --all-targets --all-features
```

If infra-as-code changed, validate it (e.g. `terraform fmt && terraform
validate`) but do **not** apply it — that is a gated human step.

## Report format

Per service, a row per `*-client` with ✅/❌ on: explicit HTTP client,
`request_id`, `tracing`, service-to-service auth, correct callee identity,
dedicated `*_http` config, startup guard, and any infra env vars this
workspace uses for the wiring. End with any client appearing inconsistent
across 2+ services (a shared gap worth extracting into a helper, if this
workspace has a shared HTTP-client crate — or worth creating one if the
duplication has crossed two services).
