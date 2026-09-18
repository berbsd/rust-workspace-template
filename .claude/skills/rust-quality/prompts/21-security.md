# Security Audit

Find Rust-level security defects in `crates/` and `services/`: secret leakage, timing-attack-vulnerable comparisons, unsafe deserialization, response-body and error-message leakage, path traversal, open redirect, weak randomness, `unsafe` without justification, and similar threats. Input validation, authorization order, and SQL injection are covered by sibling prompts — cross-reference, don't duplicate.

## Why

The workspace lints catch panics, async-safety, and obvious correctness; the other rust-quality prompts catch shape and hygiene drift. Security defects are mostly *application-level*: the code compiles, runs, and passes tests, but leaks a token in a log line, compares an HMAC byte-by-byte, accepts an extra JSON field that overwrites a privileged column, or follows a user-supplied redirect to an attacker domain. None of those trigger clippy.

## Cross-references (do not duplicate)

- **Input validation, typed-ID nil rejection, garde coverage** → `19-garde-validation.md`.
- **SQL injection, transaction boundaries, query hygiene** → `sql-analyzer` skill.
- **Handler order (extract → validate → delegate), response shape, secrets in responses** → `34-handler-hygiene.md`. If this workspace has an authorization layer, its own `require_*`/proof-object audit belongs wherever that pattern is documented — not invented here.
- **Error-mapping, `ApiErrorMapping`, no-leak error envelopes** → `13-error-pattern.md`.
- **Swallowed errors that hide failed auth or audit-log writes** → `17-swallowed-errors.md`.
- **Internal vs. public routers, public exposure of `/internal/*`** → `16-route-isolation.md`.

If an issue belongs to one of those prompts, file it there. This prompt owns everything else.

## Scope

Scan all `*.rs` files in `crates/` and `services/`. Skip `target/`, `tests/e2e/` (covered by its own harness), and generated code (`*.gen.rs`, `OUT_DIR`).

## Patterns to flag

### 1. Secrets in logs, errors, or response bodies

```rust
// Bad — token in span field
tracing::info!(token = %jwt, "validating");

// Bad — secret in error message that flows to ApiErrorBody.message
return Err(ServiceError::External(format!("Stripe rejected key {api_key}")));

// Bad — entire request struct logged (may contain password, card_number, etc.)
tracing::debug!(?req, "received signup");
```

Flag any `tracing::*!` / `format!` / `.to_string()` / `Display`/`Debug` over a value whose name suggests a secret: `token`, `jwt`, `secret`, `password`, `api_key`, `client_secret`, `bearer`, `cookie`, `session`, `webhook_secret`, `private_key`, `card_number`, `cvv`. The fix is `#[tracing::instrument(skip_all, ...)]` plus selective `fields(%user_id)`, and `Display` impls that elide the inner value (`"<redacted>"`).

Authorization headers, `Set-Cookie`, raw request bodies, and full `HeaderMap` debug-printing are the most common offenders — flag any of those landing in a span field or error variant.

### 2. Non-constant-time secret comparison

```rust
// Bad — early-exit byte comparison leaks length and prefix
if provided_hmac == expected_hmac { ... }

// Bad — same with .eq() or string comparison on tokens
if provided_token.as_str() == stored_token.as_str() { ... }
```

Any equality check on HMACs, signatures, opaque tokens, webhook secrets, or cryptographic digests must use a constant-time comparator. Use `subtle::ConstantTimeEq` (`a.ct_eq(b).into()`), `ring::constant_time::verify_slices_are_equal`, or `hmac::Mac::verify_slice`. Flag any `==` / `PartialEq` / `.eq()` on bytes that came from authentication input.

User-id equality (`user_id == owner_id`) is fine — those aren't secrets. The rule applies to anything that an attacker could probe one byte at a time.

### 3. Path traversal on filename / path inputs

```rust
// Bad — user-supplied name composed into a filesystem path
let path = Path::new(&self.storage_root).join(&req.filename);
fs::write(path, &req.bytes).await?;
```

Flag any `Path::new(...).join(user_input)`, `format!("{}/{}", root, user_input)`, or `PathBuf::push(user_input)` where `user_input` traces back to a request body / query / path parameter. The minimal fix is a garde `pattern` rule on the input plus a post-canonicalization check (`canonicalize()?.starts_with(root)`), or — better — derive the filename server-side (`{uuid}.{ext}` where ext comes from a whitelist) and never trust the client-provided name.

GCS object names with `..` are similarly hazardous when the bucket layout is shared across tenants; flag client-controlled object paths.

### 4. Open redirect

```rust
// Bad — Location header taken from user input
Ok((StatusCode::FOUND, [(header::LOCATION, req.return_to.as_str())], ()))
```

Flag any `header::LOCATION` / `Redirect::to(...)` / `Redirect::temporary(...)` whose target traces to request input without being validated against an allowlist of internal paths or whitelisted hosts. The fix is either (a) reject any value containing `://` or starting with `//`, and only allow relative paths beginning with `/`, or (b) parse with `url::Url::parse` and check `host_str()` against an allowlist.

Same rule applies to `state` / `next` query parameters in OAuth callbacks and to magic-link / unsubscribe URLs constructed from request input.

### 5. Weak / non-cryptographic randomness for tokens, IDs, or secrets

```rust
// Bad — fastrand / thread_rng for a session token
let token: String = (0..32).map(|_| fastrand::alphanumeric()).collect();

// Bad — UUIDv4 is fine for IDs; using it as a bearer token is not
let token = Uuid::new_v4().to_string();
```

Flag `fastrand`, `rand::thread_rng()`, or `Uuid::new_v4()` used for anything that grants access: session tokens, password-reset tokens, magic-link tokens, API keys, webhook signing keys, CSRF tokens. The fix is `rand::rngs::OsRng` (or `getrandom`) feeding a `[u8; 32]`, then base64url. Token length: 32 bytes (256-bit) minimum.

UUIDs *for IDs* are fine (`uuidv7()` is the workspace standard); the rule is about tokens.

### 6. JWT validation gaps

```rust
// Bad — signature verified, but no exp / aud / iss check
let claims = jsonwebtoken::decode::<Claims>(&token, &key, &Validation::default())?.claims;
```

For every JWT decode site outside the workspace's shared JWT-validation crate (if it has
one — this template's own `services/example` has no auth at all), flag:

- `Validation::default()` without `set_audience(...)` and `set_issuer(...)`.
- `validate_exp = false` or `validate_nbf = false`.
- `Algorithm::HS256` with a key shorter than 32 bytes.
- `Algorithm::None` anywhere — never.
- Decoding the token without verifying signature (`decode_header` is fine for kid lookup; `dangerous_insecure_decode` is not).

Outside `jwt-validator`, prefer not decoding at all — go through `ValidatedToken` / `AuthUser` extractors. Flag any service that re-implements JWT decoding inline.

### 7. Unrestricted deserialization

```rust
// Bad — unknown fields silently accepted, allowing mass-assignment via JSON
#[derive(Deserialize)]
struct UpdateWidget {
    name: Option<String>,
    is_admin: Option<bool>, // attacker adds this; old code didn't expect it
}
```

For request DTOs (anything that crosses an Axum extractor boundary as `Json<T>` or `Query<T>`), require `#[serde(deny_unknown_fields)]` on the struct definition. Flag DTOs that omit it. The risk is twofold: silent typos in the field name, and shipping a field rename without the client noticing.

For internal serde types (DB rows, event payloads inside the service), `deny_unknown_fields` is optional — skip the flag if the type isn't reachable from a handler input.

### 8. `unsafe` without justification

```rust
// Bad
unsafe { *ptr = value; }
```

Every `unsafe` block in `crates/` and `services/` must carry a `// SAFETY:` comment on the line above (or on the first inner line) explaining the invariant that makes it sound. Flag any `unsafe` block without one. The workspace policy is *unsafe-free by default*; if a new `unsafe` block appears, surface it for explicit user review.

FFI bindings inside `*-sys` crates are exempt.

### 9. Body / query / response size limits

Axum's default body limit is 2 MiB, which is fine for most JSON endpoints. Flag handlers that:

- Override the default via `DefaultBodyLimit::max(...)` to a value > 16 MiB without an upload-shaped reason.
- Read into `Bytes` / `String` from a stream without a length cap.
- Pass an unbounded `Vec<T>` from a request directly into a DB `IN (...)` or batch operation. Cap at `MAX_LIMIT` (50) or smaller for fan-out lists.
- Return list responses without applying the keyset-pagination cap (`MAX_LIMIT` = 50) — bypassing pagination is a DoS vector.

### 10. CORS / origin allowlist drift

```rust
// Bad
.allow_origin(Any)
.allow_credentials(true) // 4xx in browsers, but flags an attempt
```

Flag `tower_http::cors::Any` paired with `allow_credentials(true)`, or any `CorsLayer` that allows arbitrary origins on a production service. Compare each service's allowed origins against the canonical list (`PUBLIC_ALLOWED_ORIGINS` = `*.${parent_domain}` in dev *and* prod). Per-service deviations need a written reason in the handler or config doc comment.

### 11. Rate-limit-free authentication / mutation endpoints

This is a *report-only* category: clippy can't flag it. List handlers that should be rate-limited but currently aren't, judged by route:

- `POST /api/auth/v1/login` (and equivalents in any other service).
- `POST /api/auth/v1/password-reset/*`, `POST /api/auth/v1/magic-link/*`, `POST /api/auth/v1/otp/*`.
- Any `POST` that sends email or SMS.
- Any `POST` that creates billable resources (checkout sessions, widget creations, file uploads) when called by an unauthenticated principal.

The workspace doesn't ship a middleware for this yet; the report is the deliverable. Don't add `tower::limit::RateLimitLayer` without a design conversation.

### 12. Response leakage through error mapping

```rust
// Bad — DB error string ends up in ApiErrorBody.message → leaks schema / query
ServiceError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
```

For every `impl ApiErrorMapping for <Service>Error`, walk each variant and confirm the *message* string does not embed the source error's `Display`. The source should go to logs (`tracing::error!(error = ?e, ...)`); the response message should be a fixed, user-facing string. Same rule for `details: Option<Value>` — never serialize the raw DB error.

Cross-link: variant ordering and `#[from] garde::Report` are in `13-error-pattern.md`; this prompt owns the leakage angle.

### 13. Webhook signature verification

For every endpoint receiving a webhook (Stripe, Pub/Sub push, GitHub, Twilio, etc.):

- Signature must be verified before any business logic runs. Flag handlers that read the body, do work, and verify later.
- Timestamp check (replay window, usually ≤ 5 min) must be present.
- Secret comes from config (env var), not a literal.
- Comparison uses constant-time (rule #2).
- On failure, return a fixed error string — never echo the supplied signature or computed expected value.

### 14. `Debug` impls on secret-bearing structs

```rust
// Bad — `#[derive(Debug)]` on a struct holding raw tokens
#[derive(Debug)]
pub struct Session { user_id: UserId, refresh_token: String }
```

For any struct field whose name matches the secret patterns from rule #1, the containing struct must either (a) not derive `Debug`, (b) derive it via `derive_more::Debug` with `#[debug(skip)]` on the secret field, or (c) implement `Debug` manually to redact. The risk: someone logs the struct via `?` formatting and the secret ships to Stackdriver.

### 15. JWT validator with no revocation-cache subscription

```rust
// Bad — validates platform JWTs but never hears about a revocation
let validator = JwtValidator::from_auth_service(&config, http).await?;
```

the auth service publishes a token-revocation event (`jti`, `exp`) whenever `POST /revoke` or
`POST /logout` invalidates an access token — but the JWT itself is stateless, so a service that
verifies it via `jwt_validator::JwtValidator`/`PreparedJwks` and never attaches a
`RevocationCache` keeps accepting that exact token in *its own process* until the token's natural
`exp`, no matter what the auth service did. This is not hypothetical: a `JwtValidator` without a
revocation subscriber silently continues honoring a revoked token until it expires on its own,
and this applies independently to every service, not just auth (the auth service wires its own
`RevocationCache` into its token-minting path; a downstream service needs its own instance wired
into its `JwtValidator`).

Flag any service whose `main.rs`/`module.rs` constructs a `JwtValidator` (i.e. actually verifies
platform-user JWTs, not a mock/an OIDC-only validator) without:

1. `Arc::new(jwt_validator::RevocationCache::new())`, attached via `.with_revocation_cache(...)`.
2. A subscriber consuming the auth service's token-revocation event that calls
   `cache.revoke(event.jti, event.exp)` on delivery.

The fix is structural, not a one-line patch — the revocation cache and its subscriber need to be
wired together in the service's own startup code, not hand-rolled inline in the finding.

This check only applies to a workspace that actually has a `jwt-validator`-style crate and an
auth service publishing revocation events; skip it entirely if neither exists.

## Fixing

Pick the narrowest fix that resolves the threat:

1. **Redact** — `#[tracing::instrument(skip_all, fields(%user_id))]`, custom `Display`/`Debug` impls, error messages that don't embed sources.
2. **Replace the primitive** — `subtle::ConstantTimeEq` for secrets, `OsRng` for tokens, `Url::parse` + allowlist for redirects, `deny_unknown_fields` for DTOs.
3. **Server-derive** — never accept a value from the client that you can compute yourself (filenames, ID-of-current-user, timestamps).
4. **Cap or reject** — body size limits, pagination cap enforcement, signature timestamp window.
5. **Escalate** — anything new under `unsafe`, anything that disables a layer of the JWT validator, anything that adds `Any` to CORS — file the finding but don't fix without explicit user approval.

## Verification

After fixes:

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
cargo test --workspace
```

For changes touching crypto or token generation, run `cargo audit` (or `cargo deny check advisories`) and report transitive findings.

For changes touching webhook handlers, run the matching k6 scenario under `tests/e2e/` (mints an auth primitive, then exercises the endpoint with a known-good and a known-bad signature).

## Report format

Group by rule (1–15), then by file. For each finding give file:line, the offending snippet (one line), the rule violated, and the chosen fix. End with a service × rule matrix so the user sees where threats cluster:

```
              | secrets | timing | path-trav | redirect | rng | jwt | serde | unsafe | size | cors | webhook | dbg | revoc
auth          |    1    |   0    |     0     |    1     |  0  |  0  |   0   |   0    |  0   |  0   |    0    |  0  |   0
widget        |    0    |   0    |     2     |    0     |  0  |  0  |   1   |   0    |  1   |  0   |    0    |  0  |   1
orders        |    0    |   1    |     0     |    0     |  0  |  0  |   0   |   0    |  0   |  0   |    1    |  0  |   1
```

For rule #11 (rate-limiting) the cell is the *list of unprotected endpoints*, not a count — the user needs the route names to plan the follow-up.
