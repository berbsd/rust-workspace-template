# Internal Route Isolation

Audit every service's router composition to ensure internal (service-to-service) routes are never reachable without S2S authentication. Verify the three-layer defense: integration test, CI lint, and Cloud Armor edge rule.

## Why this is its own check

A single internal router merged without its `internal_auth_layer` silently exposes endpoints meant for service-to-service traffic to the public internet — without authentication. The check is mechanical, the failure mode is severe, and the fix touches code, tooling, and infrastructure. Treat it as a standalone audit, not a sub-bullet.

## Route categories

Every route in every service belongs to **exactly one** category:

| Category    | Auth layer                                             | Examples                     |
|-------------|--------------------------------------------------------|------------------------------|
| public      | none                                                   | `/health`, `/ready`, `/docs` |
| protected   | `auth_layer(validator, svc)` (user JWT)                | `/widgets`, `/profile/me`    |
| admin       | `admin_auth_layer(validator, svc)` (fused JWT + role)  | `/admin/users`               |
| internal    | `internal_auth_layer(oidc_validator)` (GCP OIDC, S2S)  | `/internal/members`          |

All four tiers share one listener (Cloud Run exposes a single container port);
isolation is middleware (per-tier auth layers) plus the Cloud Armor edge rule,
not separate ports.

## Rules

1. **Internal routes live on a dedicated internal router with `internal_auth_layer(oidc_validator)` applied via `route_layer` before it is merged** — never merged bare onto the public/protected router (the layer-after-merge ordering mistake leaves them unprotected).
2. **Internal route paths always contain `/internal/`** — both as a marker and so the Cloud Armor rule matches them.
3. **OIDC is the isolation mechanism.** `internal_auth_layer` validates the GCP-signed S2S token (audience-scoped per service); a service-wide `internal_path_guard` (applied by `Service::build()`) fails closed on any `/internal/`-path response the layer did not stamp.
4. **Cloud Armor blocks `/internal/` at the edge** as defense-in-depth — any external request whose path matches gets `deny(404)` before reaching the service.
5. **Admin routes are layered with the fused `admin_auth_layer(validator, service)`** — one call doing JWT validation + anonymous rejection + `admin` role check, so the stack cannot be half-applied. Never raw `auth_layer` alone, and never the legacy two-layer `admin_layer()` + `auth_layer()` stacking on an `/admin/` path. Composition shape (merge **before** `route_layer` — reversed, merged routes arrive after the layer and are silently unprotected):

   ```rust
   let admin = Router::new()
     .merge(crate::feature::admin::router::routes_admin())
     .route_layer(admin_auth_layer(platform_validator, "auth"));
   // merged into the module router (routes carry the /admin prefix)
   ```

   The mount-without-layer mistake fails closed at runtime: `admin_auth_layer` stamps an `AdminLayerVerified` response extension, and the service-wide `admin_path_guard` (applied by `Service::build()`) replaces any `/admin`-path response lacking it with a 403 + ERROR log — every mount shape covered. The inverse mistake (`admin_auth_layer` on a non-`/admin` path) logs at ERROR but serves. Residual to audit by hand: an admin-purpose route whose path lacks `/admin` entirely.
6. **Admin route paths always contain `/admin/`** — the path is the marker this audit and reviewers key on.

## What to check

### A. Source-level audit (every service)

For each `services/*/src/main.rs` (or wherever the router is composed):

1. Find every router built — public, protected, admin, internal.
2. Confirm the internal router has `internal_auth_layer(oidc_validator)` applied via `route_layer` **before** any merge, and is **not** merged bare (`public.merge(internal_api)` / `protected.merge(internal_api)` without the layer).
3. Confirm every route declared inside the internal router has `/internal/` in its path. A route at `/api/<svc>/v1/members` declared on the internal router but missing the `/internal/` segment escapes the path guard and bypasses Cloud Armor — flag it.
4. Confirm no route declared on the public/protected/admin routers contains `/internal/` in its path. That would create an external-facing endpoint that the Cloud Armor rule will (correctly) block — the route exists in code but is unreachable, which is dead code at best and confusing at worst.
5. Confirm every `/admin/...` route travels through `admin_auth_layer` (route_layer on the admin router before merging). The runtime `admin_path_guard` fails closed on mount-without-layer mistakes; what it cannot catch — and this audit exists to close — is an admin-purpose route whose path lacks `/admin/` entirely, or raw `auth_layer` stacking standing in for the fused layer.

### B. Integration test presence

For each service that has internal routes, verify a test exists that:

- Builds the internal routes behind the **real** `internal_auth_layer` (mock
  OIDC validator — see `services/example/tests/membership_handlers.rs` for the
  exemplar shape).
- Probes one or more `/internal/...` paths via `tower::ServiceExt::oneshot`
  **without** a valid bearer.
- Asserts the response is `401 UNAUTHORIZED` (and the happy path succeeds
  with the mock-validated token).

Suggested location: `services/<name>/src/server/tests.rs` or `services/<name>/tests/route_isolation_it.rs`.

Flag services with internal routes but no such test — that's the layer that catches new mistakes at `cargo test` time.

### C. Justfile lint presence

Confirm the workspace `justfile` has an `audit-internal` recipe that fails if
any service declares `/internal/` routes without applying
`internal_auth_layer` somewhere in the same crate (this is what the recipe
checks today — route declaration and layer application live in different
files, so the check is crate-scoped).

It should be wired into `just check` (or run as part of CI).

Flag if the recipe is missing, not wired into `just check`, or has been weakened (e.g. matching only some services).

### D. Cloud Armor rule presence (defense in depth)

Outside the Rust workspace — if the repo includes Terraform / GCP config (`gcp/`, `terraform/`, `.tf` files), confirm a Cloud Armor rule exists with:

- Action: `deny(404)` (not 403 — 403 would reveal the existence of internal routes).
- Match: path matches `/internal/`.
- Priority: high (e.g. 100), so it evaluates before any allow rules.

If infrastructure config isn't in this repo, note it as out-of-scope for this audit but mention that the rule must exist somewhere.

## Verification

After fixes:

```bash
just audit-internal
just test-unit          # exercises the integration test added in B
cargo check --workspace
```

All three must pass.

## Report format

Group by service. For each service, report each of A/B/C as PASS/FAIL with a short reason. End with a workspace-level summary for D (Cloud Armor) and the justfile recipe.

```
services/example       A:PASS  B:FAIL  C:n/a   — missing route_isolation test
services/orders        A:FAIL  B:PASS  C:n/a   — internal route at /members lacks /internal/ prefix (file:line)

Workspace:
  C: PASS — audit-internal recipe present, wired into `just check`
  D: PASS — Cloud Armor rule at gcp/security_policy.tf priority 100
```
