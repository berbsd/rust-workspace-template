# OpenAPI Security Accuracy

Audit every `#[utoipa::path]` `security(...)` declaration against what the route **actually enforces and accepts** on the wire. The OpenAPI spec is the external contract: a security entry the route does not honor is a documented lie, and an enforcement path the spec omits is an undocumented capability. Both are findings.

## Why

Security declarations drift independently of the code that enforces auth, because they live in three places that nothing reconciles:

1. the handler's `security(...)` attribute (the claim),
2. the router's middleware stack (`auth_layer`, `route_layer`) (one enforcement point),
3. in-handler extractors and gates (`resolve_api_key`, `require_*`) (the other enforcement point).

A public catalog service can ship exactly this bug: read endpoints declare `("bearer_jwt" = [])` and the `info()` docs promise "authenticated users bypass the per-IP cap". In reality the read routes have **no** `auth_layer`, and the `Option<AuthUser>` extractor never reads `Authorization` — it reads request extensions (only populated by `auth_layer`) or falls back to forwarded `X-Auth-*` headers. The documented JWT mode cannot work — and worse, the authorization gate keys a full rate-limit bypass on `auth.is_some()`, reachable by any caller forging two `X-Auth-*` headers. A spec/capability audit catches both halves: the phantom scheme and the forgeable-input bypass.

## What to Scan

For each service with `with_openapi(...)`:

- every handler with `#[utoipa::path]` — collect its `security(...)` entries (`()`, `("bearer_jwt" = [])`, `("api_key" = [])`, …)
- the router assembly (`server.rs` / `router.rs`) — which routes get `auth_layer(...)` / `route_layer(...)`, which deliberately don't
- handler signatures — `AuthUser`, `Option<AuthUser>`, `ValidatedToken`, custom principal resolvers (`resolve_api_key`, header extractors)
- the service's `SecurityAddon` (or equivalent `Modify` impl) — which schemes are registered
- the `info(description = ...)` auth section — the prose contract

## Verification Rules

For each handler, reconcile claim vs enforcement:

1. **`bearer_jwt` declared ⇒ a JWT is actually validated on that route.** Valid evidence: `auth_layer(validator, ...)` on the route, or an extractor that itself cryptographically validates the `Authorization` token (`ValidatedToken`). **Not** valid evidence: an `AuthUser` / `Option<AuthUser>` extractor alone — on a route without `auth_layer` it reads extensions (absent) or forwarded `X-Auth-*` headers (client-forgeable unless the ingress provably strips them). If the only path to "authenticated" is forwarded headers, the scheme is phantom: remove it from `security(...)` or wire real validation — ask the user which matches product intent.
2. **`api_key` declared ⇒ the handler resolves and validates a key** (e.g. `resolve_api_key` → validator → scope/quota gates) and the failure mode is a documented 401/403/429.
3. **`()` (anonymous) declared ⇒ the route genuinely serves unauthenticated callers** (possibly quota-limited). If every request is rejected without credentials, drop `()`.
4. **No undeclared modes.** If the route enforces or accepts an auth mode (key, JWT, OIDC service-to-service) that `security(...)` omits, add it.
5. **Authorization decisions never key on forgeable input.** Flag any grant/bypass/quota decision derived from an optional extractor or forwarded header on a route without a validating layer (`if auth.is_some() { bypass }` is a real class of auth-bypass bug — an optional extractor silently downgrades to "trust the caller" instead of rejecting). Cross-reference the trust-model rules in `21-security.md`.
6. **Schemes registered ⇔ schemes referenced.** Every scheme named in any `security(...)` exists in the `SecurityAddon`; every registered scheme is referenced by at least one surviving handler (the SDK/spec pipeline may prune, but the source should not carry orphans).
7. **Error responses match the auth model.** JWT-required endpoints document `401` (and `403` where valid-but-insufficient identities are rejected). Anonymous-plus-key endpoints' `401` remediation text must match the product ("use an API key") — don't tell callers to do something that doesn't lift the restriction (a `401` that says "sign in" when signing in grants nothing is exactly this bug).
8. **Prose matches the table.** The `info(description)` auth section lists exactly the modes that survive rules 1–4 — no more, no fewer.

## Output

Per service, a table: endpoint → declared schemes → enforcement evidence (file:line of layer/extractor/gate) → verdict (`match` / `phantom scheme` / `undeclared mode` / `forgeable bypass`). Fix `phantom scheme` and prose drift directly (spec-only change); treat `forgeable bypass` and `undeclared mode` as behavior findings — state the fix and its risk, and confirm product intent with the user before changing enforcement code.

Only applies to a service that enables `with_openapi(...)`/`#[utoipa::path]` with a real
auth layer behind it; skip entirely for one that doesn't (this template's own
`services/example` has neither).

## Cross-references (do not duplicate)

- `21-security.md` — owns the general trust-model audit (forwarded headers, IP spoofing); this prompt applies it specifically to spec/enforcement reconciliation.
- `16-route-isolation.md` — owns internal-route exposure; an internal route appearing in the public spec is its finding, not this one's.
