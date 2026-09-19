# Internal Route Isolation

Audit every service's router composition to ensure internal (service-to-service) routes are never reachable without S2S authentication. Verify the layered defense: an auth middleware applied before merge, an integration test proving it, a lint catching regressions, and (if the deployment has one) an edge/WAF rule as defense-in-depth.

This check applies to workspaces with more than one auth tier — public, user-authenticated, admin, and/or internal service-to-service routes. `services/example` in this template is a single public CRUD resource with no auth layering at all, so this check is a no-op here; the shapes below are illustrative for when the workspace grows a protected/admin/internal tier. Substitute this workspace's actual middleware names, route conventions, and deployment platform for the placeholders below.

## Why this is its own check

A single internal router merged without its internal-auth middleware silently exposes endpoints meant for service-to-service traffic to the public internet — without authentication. The check is mechanical, the failure mode is severe, and the fix touches code, tooling, and (if applicable) infrastructure. Treat it as a standalone audit, not a sub-bullet.

## Route categories

Once a workspace has more than one tier, every route in every service belongs to **exactly one** category — illustrative example:

| Category    | Auth layer                                                  | Examples                     |
|-------------|---------------------------------------------------------------|-------------------------------|
| public      | none                                                          | `/health`, `/ready`, `/docs` |
| protected   | a user-auth layer (validates a user token)                    | `/widgets`, `/profile/me`    |
| admin       | a fused auth+role layer (validates a user token + admin role) | `/admin/users`               |
| internal    | an S2S-auth layer (validates a signed service-to-service token) | `/internal/members`        |

If every tier is served behind one listener, isolation between tiers is middleware
(per-tier auth layers) plus an edge rule if the deployment platform offers one —
not separate ports.

## Rules

1. **Internal routes live on a dedicated internal router with the internal-auth layer applied via `route_layer` before it is merged** — never merged bare onto the public/protected router (the layer-after-merge ordering mistake leaves them unprotected).
2. **Internal route paths carry a consistent marker (e.g. `/internal/`)** — both for the source audit below and so an edge rule, if one exists, can match them.
3. **The S2S token is the isolation mechanism.** The internal-auth layer validates it; a service-wide fallback guard that fails closed on any internal-path response the layer didn't stamp is a good defense-in-depth pattern if this workspace's framework supports it.
4. **An edge rule blocks the internal path marker at the edge, if the deployment platform has one** — defense-in-depth, not the primary control.
5. **Admin routes are layered with a single fused auth+role layer** — one call doing token validation + anonymous rejection + role check, so the stack cannot be half-applied. Never a bare user-auth layer alone, and never two separate layers stacked in place of one fused layer. Composition shape (merge **before** `route_layer` — reversed, merged routes arrive after the layer and are silently unprotected):

   ```rust
   let admin = Router::new()
     .merge(crate::feature::admin::router::routes_admin())
     .route_layer(admin_auth_layer(validator, "auth"));
   // merged into the module router (routes carry the /admin prefix)
   ```

   If this workspace's framework supports a response-extension marker plus a service-wide guard that fails closed on any admin-path response lacking it, that closes the mount-without-layer mistake at runtime — the audit below exists for what such a guard can't catch: an admin-purpose route whose path lacks the marker entirely, or a wrong-tier layer on the wrong path.
6. **Admin route paths carry a consistent marker (e.g. `/admin/`)** — the path is what this audit and reviewers key on.

## What to check

### A. Source-level audit (every service that has more than a public tier)

For each `services/*/src/main.rs` (or wherever the router is composed):

1. Find every router built — public, protected, admin, internal — whichever tiers this service actually has.
2. Confirm the internal router has its internal-auth layer applied via `route_layer` **before** any merge, and is **not** merged bare (`public.merge(internal_api)` / `protected.merge(internal_api)` without the layer).
3. Confirm every route declared inside the internal router carries the internal path marker. A route missing that segment escapes any path guard and bypasses an edge rule if one exists — flag it.
4. Confirm no route declared on the public/protected/admin routers carries the internal path marker. That would create an external-facing endpoint an edge rule would (correctly) block — the route exists in code but is unreachable, which is dead code at best and confusing at worst.
5. Confirm every admin route travels through the fused admin-auth layer (route_layer on the admin router before merging). What a runtime guard can't catch — and this audit exists to close — is an admin-purpose route whose path lacks the admin marker entirely, or a bare user-auth layer standing in for the fused layer.

### B. Integration test presence

For each service that has internal routes, verify a test exists that:

- Builds the internal routes behind the **real** internal-auth layer (with a mock token validator).
- Probes one or more internal-path routes via `tower::ServiceExt::oneshot`
  **without** a valid credential.
- Asserts the response is `401 UNAUTHORIZED` (and the happy path succeeds
  with a mock-validated credential).

Suggested location: `services/<name>/src/server/tests.rs` or `services/<name>/tests/route_isolation_it.rs`.

Flag services with internal routes but no such test — that's the layer that catches new mistakes at `cargo test` time.

### C. Lint/recipe presence

If this workspace has a `justfile` (or CI) recipe meant to fail when a service declares an internal route without applying the internal-auth layer, confirm it's still correct and wired into `just check` (or CI). If no such recipe exists yet but the workspace has more than one auth tier, that absence is itself a finding worth raising — not something to invent a recipe name for here.

Flag if the recipe is missing, not wired into `just check`, or has been weakened (e.g. matching only some services).

### D. Edge/WAF rule presence (defense in depth, if applicable)

Outside the Rust workspace — if the repo includes infra-as-code (Terraform or similar) for a deployment platform with edge/WAF rules, confirm a rule exists that:

- Denies with a response that doesn't reveal the existence of internal routes (e.g. a plain 404, not a 403).
- Matches the internal path marker.
- Has high priority, so it evaluates before any allow rules.

If infrastructure config isn't in this repo, note it as out-of-scope for this audit but mention that the rule must exist somewhere if the workspace relies on it as a layer of defense.

## Verification

After fixes, run this workspace's equivalent of:

```bash
just check               # or whichever recipe wires in the C lint, if one exists
just test-unit           # exercises the integration test added in B
cargo check --workspace
```

All must pass.

## Report format

Group by service. For each service, report each of A/B/C as PASS/FAIL with a short reason. End with a workspace-level summary for D (edge rule, if applicable) and the lint recipe.

```
services/<name>       A:PASS  B:FAIL  C:n/a   — missing route_isolation test
services/<other>      A:FAIL  B:PASS  C:n/a   — internal route at /members lacks internal-path marker (file:line)

Workspace:
  C: PASS — internal-route lint present, wired into `just check`
  D: PASS — edge rule present in infra config, high priority
```
