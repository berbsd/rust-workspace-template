# Secret Type Wrapping

Find every secret-bearing value in `crates/` and `services/` whose Rust type is plain `String` or `Vec<u8>` and lift it into `secrecy::SecretString` (or a `secrecy::SecretBox<...>` for non-string bytes). The goal is *type-level* protection: a future contributor's `tracing::debug!(?config)` cannot leak a secret because the wrapper's `Debug` impl already redacts.

This is the input/transport-boundary discipline. The output-boundary discipline (don't log tokens, don't put them in error messages, don't return them in response bodies) is owned by `21-security.md` — both apply, they are complementary.

## Why

Most deployments encrypt secrets at rest (a secrets manager, sealed env vars, a vault) and treat env-injected config as sensitive by the time it reaches the process. The infra side classifies what's a secret; the Rust side must match that classification in the type system, or the classification is theater. A secret-bearing field left as plain `String` — an API key, a signing key, an OIDC `client_secret`, an HMAC secret — makes every `Config` that derives `Debug` (and most do, for ergonomic error messages) one accidental `tracing::debug!(?config)` away from a full-credential leak in whatever log sink is downstream, often one that indexes the entry permanently even after rotation.

`SecretString` makes the type the contract:

- `?config` renders the field as `Secret([REDACTED ...])`.
- The value cannot be reached without an explicit `.expose_secret()` call — grep-able, reviewable, intentional.
- The buffer is zeroized on drop, narrowing the post-free memory disclosure surface.

clippy can't see the difference between `pub api_key: String` and `pub api_key: SecretString` — this prompt is the discipline.

## Cross-references (do not duplicate)

- **Don't log tokens, don't put them in error strings, don't return them in response bodies** → `21-security.md`. The `SecretString` wrapper is a type-level *help*, but if a consumer calls `.expose_secret()` and then logs the result, that's an output-boundary violation owned by `21-security.md`.
- **Garde validation patterns for `Option<SecretString>` / non-empty checks via factory closure** → `19-garde-validation.md`. This prompt establishes the wrapping; the validation expression form is shared.
- **Path/import conventions for `secrecy::ExposeSecret`** → `22-path-conventions.md`. Bring `ExposeSecret` into scope with `use`; never UFCS-qualify it inline.
- **Workspace `secrecy` dep** lives in root `Cargo.toml`:
  `secrecy = { version = "0.10.3", features = ["serde"] }`. Per-crate manifests opt in with `secrecy = { workspace = true }`.

## Scope

Scan all `*.rs` files under `crates/` and `services/`. Specifically:

1. Every `struct *Config` field whose name matches the secret patterns below.
2. Every constructor/builder argument flowing from those fields into adapters (`HmacSigner::new(access_id, secret)`, `ResendEmailProvider::new(..., api_key, ...)`, etc.).
3. Every struct field that *stores* a secret in an adapter (e.g. `pub struct ResendEmailProvider { api_key: String }` after the value left the Config).

If this workspace manages secrets via infra-as-code (a Terraform/Pulumi module declaring secret-manager resources, a sealed-secrets manifest, etc.), cross-reference against it — anything declared there as a secret resource is, by definition, a secret. If its Rust mirror is `String`, it's a candidate. Skip this cross-reference if no such infra definition exists in the workspace.

Skip `target/`, `tests/e2e/`, and `*.gen.rs`.

## What is a secret

A value is a secret if **any** of these is true:

1. Infra-as-code (if present) wraps it as a secret-manager resource.
2. Disclosure enables impersonation (tokens, signing keys, webhook secrets, HMAC secrets).
3. Disclosure enables billing/quota exhaustion against the workspace's account with a third-party provider (payment processors, LLM/API providers, messaging providers, map/geocoding providers, etc.).
4. Disclosure compromises a cryptographic invariant (RSA private keys, JWT signing PEMs, session cookie signing keys, HMAC keys).

A value is **not** a secret if:

- It's a *key identifier* (e.g. an object-storage client's `hmac_access_id`, OIDC `client_id`). Identifiers pair with secrets but aren't independently sensitive.
- It's a configuration URL (e.g. a webhook base URL, an upstream service's base URL). URLs are public knowledge by deployment.
- It's a public allowlist (e.g. `allowed_origins`, `accepted_audiences`).
- It's an HTTP audience claim or issuer URL (`internal_auth.audience`, `auth.issuer`).
- It's a flag/boolean derived from a sensitive setting (`billing_enabled`, `allow_dev_tokens`) — the *value* is policy, not a credential.

When in doubt, ask: "If this value is logged once, do we rotate?" If yes, it's a secret.

## Patterns to flag

### 1. Secret-bearing field as plain `String`

```rust
// Bad
pub struct StripeConfig {
  #[garde(length(min = 1))]
  pub secret_key: String,
}

// Good
use secrecy::{ExposeSecret, SecretString};

pub struct StripeConfig {
  #[garde(custom(validate_non_empty_secret()))]
  pub secret_key: SecretString,
}
```

Name-shape heuristics: `secret_key`, `*_token`, `api_key`, `*_secret`, `client_secret`, `webhook_secret`, `signing_key`, `private_key`, `hmac_secret`, `password`, `auth_token`, `as_token`, `hs_token`. Most are obvious; the false-positive trap is `*_id` and `*_audience` (not secrets, see above).

### 2. Adapter struct holding the unwrapped value past the Config boundary

```rust
// Bad — the value left Config, but it's now a plain field again
pub struct ResendEmailProvider {
  api_key: String,
  // ...
}

// Good — secret-ness travels with the value
pub struct ResendEmailProvider {
  api_key: SecretString,
  // ...
}
```

If a `*Config` field has been wrapped but the adapter that consumes it stores `String`, the wrap is meaningless — the leakage path moves from `?config` to `?provider` and nothing else changes. Wrap both.

### 3. `.expose_secret()` called eagerly at construction

```rust
// Bad — secret leaves the wrapper at startup, sits as String for the
// lifetime of the adapter
pub fn new(api_key: SecretString) -> Self {
  Self { api_key: api_key.expose_secret().to_owned() }
}

// Good — wrapper held; expose() called exactly at the protocol boundary
pub async fn send(&self, body: ...) -> Result<...> {
  self.http
    .post(&url)
    .bearer_auth(self.api_key.expose_secret()) // <-- only here
    .body(body)
    .send()
    .await
}
```

Each `.expose_secret()` call site is a deliberate cleartext window. Push it as close to the wire as possible — the HTTP header builder, the HMAC `update()` call, the `EncodingKey::from_rsa_pem` call. Never copy the inner value into a long-lived `String`.

### 4. `Serialize` on a struct containing `SecretString`

```rust
// Bad
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct MinterConfig {
  pub signing_key_pem: SecretString,  // Serialize would emit cleartext
}
```

`secrecy = "0.10"`'s `serde` feature provides `Serialize for SecretBox<T>` where `T: Serialize` — and it serializes the inner buffer, defeating the redaction. Drop the `Serialize` derive unless you have a concrete reason to serialize the struct *and* a plan to skip-serialize the secret field. The default position: secret-bearing configs are `Deserialize`-only.

If Serialize is genuinely needed for a non-secret part of the struct, either split the struct (extract the secret-bearing nested type) or annotate the secret field with `#[serde(skip_serializing)]`.

### 5. `Clone` on a struct containing `SecretString`

`SecretString = SecretBox<str>` is **not** `Clone` in `secrecy = "0.10"` (the trait is `CloneableSecret`, only implemented for integer primitives in the crate; `str` is `?Sized` so an opt-in wouldn't apply, and the orphan rule blocks downstream impls for `String`).

```rust
// Bad — compiles via Arc<SecretString>, but the indirection is the worst
// of both worlds: extra heap pointer + nothing prevents callers from
// caching the cloned Arc forever
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct StripeConfig {
  pub secret_key: Arc<SecretString>,
}

// Good — drop the Clone derive; nothing legitimately clones a Config
#[derive(Debug, Deserialize, Validate)]
pub struct StripeConfig {
  pub secret_key: SecretString,
}
```

Before dropping `Clone`, grep for `.clone()` on the struct and any ancestor that embeds it:

```bash
rg -n '<TYPE>::clone|let .* = .*config\.clone\(\)' --type rust
```

The vast majority of Config `Clone` derives in this workspace are vestigial — added by reflex from sibling modules. Dropping them is a no-op. Add the standardized doc comment in place of the derive:

```rust
/// `Clone` is intentionally not derived — `<field>` is a non-`Clone`
/// `SecretString`, and nothing in this service clones the root config.
```

If a real caller depends on `Clone`, the fix is to restructure ownership (move the field, wrap the entire Config in `Arc`, or split the secret-bearing piece into a separate non-Clone substruct) — *not* to wrap the secret in `Arc<SecretString>`.

### 6. `garde` validators that try to read the inner value

`#[garde(length(min = 1))]` no longer applies — `SecretString` doesn't expose a `.len()` to garde's derive. Replace with a factory-returning-closure validator that exposes once:

```rust
fn validate_non_empty_secret() -> impl FnOnce(&SecretString, &()) -> garde::Result {
  |value, _ctx| {
    if value.expose_secret().is_empty() {
      return Err(garde::Error::new("must not be empty"));
    }
    Ok(())
  }
}

#[garde(custom(validate_non_empty_secret()))]
pub api_key: SecretString,
```

For `Option<SecretString>`, follow the same shape but iterate the `Option` inside the closure (see whichever crate in this workspace already wraps an optional secret for the canonical form — e.g. an object-storage client's HMAC credential). Do **not** use `#[garde(inner(...))]` — the `inner` combinator dives into the `Option` and then hits the same `length` problem on the inner `SecretString`.

The factory shape is preferred over a free function with `&SecretString, &()` parameters because the latter trips `clippy::trivially_copy_pass_by_ref` on the `&()` ctx arg. The factory hides the closure parameters from that lint.

### 7. Test fixtures hard-coding the wrapped form

Tests constructing a Config or adapter with a literal secret must wrap explicitly:

```rust
// Bad
ResendEmailProvider::new(client, base_url, "re_test_key".into(), "from@x".into())

// Good
ResendEmailProvider::new(
  client,
  base_url,
  SecretString::from("re_test_key".to_owned()),
  "from@x".into(),
)
```

`"literal".into()` works when the target type is `String`. When the target is `SecretString`, `From<&str>` is *not* provided; use `SecretString::from("literal".to_owned())` or `String::from("literal").into()`. The compiler error is clear; just be consistent.

### 8. Cross-boundary leak: builder `from_config(&Config)`

If the source Config holds `SecretString` and is not `Clone`, a `from_config(&Config)` that needs to *take ownership* of the secret to construct an adapter is forced to clone — which won't compile. Change the signature to take by value:

```rust
// Bad in a Clone-dropped world — won't compile
pub fn from_config(config: &GcsConfig) -> Self {
  builder.with_hmac(config.hmac_access_id.clone(), config.hmac_secret.clone())
}

// Good — moves the secret in, no clone needed
pub fn from_config(config: GcsConfig) -> Self {
  match (config.hmac_access_id, config.hmac_secret) { ... }
}
```

Update all call sites (`from_config(&config.gcs)` → `from_config(config.gcs)`); partial moves out of the parent `Config` are legitimate Rust and the rest of the field remains usable.

### 9. Logging or `Display` formatting after `.expose_secret()`

```rust
// Bad — the secret is now in a String that gets logged
let revealed = self.api_key.expose_secret().to_owned();
tracing::info!(key = %revealed, "using key");

// Bad — Display of an exposed value
tracing::info!(key = %self.api_key.expose_secret(), "calling Resend");
```

The `expose_secret()` return value (`&str`) must flow only into a wire-format builder (`bearer_auth`, `header`, HMAC `update`, JWT encoder). It must not flow into a `tracing` field, `format!`, error message, or response body. The `21-security.md` patterns apply post-exposure.

### 10. Doc-example drift

```rust
// Bad — teaches a pattern the codebase has moved away from
/// ```toml
/// api_key = "re_xxx"
/// ```
pub api_key: String,
```

Doc examples in module-level `//!` blocks and in struct-level `///` blocks must show the wrapped type or a redacted placeholder. If the example uses `secrecy::SecretString::from("...")` literally, that's fine. If it shows the field as plain `String`, fix the example when fixing the type.

## Workflow

### 1. Inventory

```bash
# Plain-String fields whose name suggests a secret
rg -nE 'pub\s+(secret_key|webhook_secret|signing_key|signing_key_pem|hmac_secret|api_key|client_secret|password|as_token|hs_token|auth_token|private_key):\s*String' --type rust crates/ services/

# Cross-reference with infra-as-code, if this workspace has any
rg -lnE 'secret' infra/ 2>/dev/null
```

For each hit, record: file, line, field name, owning struct, and whether the struct currently derives `Clone` / `Serialize`.

### 2. Choose phase

The cost of wrapping is roughly:

- **Shared crate (high leverage):** a secret field on a widely-depended-on client or token-issuing crate (e.g. an object-storage client's HMAC secret, an auth crate's signing key). One change cascades to every consumer. Do these first.
- **High-blast-radius service-local secrets:** payment-processor keys, webhook secrets, OIDC `client_secret`. Each is one service plus its adapter.
- **API keys (lower blast radius):** the rest of the third-party API keys (LLM providers, email/SMS providers, maps/geocoding, captcha, etc.). Mechanical per-service work after the pattern is established.

### 3. Wrap one struct end-to-end

Per struct:

1. Add `secrecy = { workspace = true }` to the crate's `Cargo.toml`.
2. Change the field type to `SecretString` (or `Option<SecretString>`).
3. Replace `#[garde(length(...))]` with `#[garde(custom(validate_non_empty_secret()))]` (or the `Option`-aware variant).
4. Drop `Clone` from the struct's derive — add the standardized doc comment.
5. Drop `Serialize` from the struct's derive unless you've audited every use-site.
6. Propagate the type change into adapter struct fields and constructor signatures.
7. Push `.expose_secret()` to the wire-format builder (header, HMAC, encoder).
8. Update test fixtures to wrap literals in `SecretString::from(...)`.
9. Run the crate's tests; ensure the wire contract is unchanged.

If the consumer needs to take the value by value and the source struct is no longer `Clone`, change the builder/consumer signature from `&Config` to `Config` (move) and update call sites.

### 4. Cascade `Clone` drops

When a shared-crate type drops `Clone`, every consumer Config that embeds it loses `Clone` too. The compile error names every embed site. Fix them:

- Drop `Clone` from the consumer struct's derive.
- Add the standardized doc comment.
- Confirm no production caller clones it. If one does, **do not** restore `Clone` via `Arc<SecretString>` — restructure ownership instead.

### 5. Verify

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
cargo nextest run -p <crate-under-test> --all-features
```

Per-crate tests should pin the wire contract (a test asserting the exact `Authorization` header value, or the exact HMAC signature bytes, is the model — it should pass after the wrap unchanged, because the signing primitive sees the same bytes either way).

After verification, grep for any leftover plain-String secret in the same crate:

```bash
rg -nE 'pub\s+\w*(secret|token|key|password)\w*:\s*String' --type rust crates/<name> services/<name>
```

Visually triage. Identifiers (`access_id`, `client_id`, `*_id`) stay as `String`.

## Fixing

1. **One-secret-per-commit when possible.** Each commit changes one `Config` field, its consumers, and its tests. Easier to review, easier to revert.
2. **Shared-crate changes commit before consumers.** The cascade is mechanical and the consumer-commit message reads cleaner against a clean baseline.
3. **Never** `Arc<SecretString>` as a workaround. If you reach for that, the design is wrong — either move the value or split the struct.
4. **Never** `#[serde(skip)]` on a `SecretString` field unless the runtime *intentionally* needs to ignore env input. If the secret is env-loaded, `Deserialize` must reach it.
5. **Never** call `.expose_secret()` outside `crates/{name}/src/adapter/` or the equivalent wire-format module. The `infra-classified secret` should be cleartext for one statement only.
6. **Pre-existing `Clone` callers** must be removed or the value moved. Do not introduce `secrecy::CloneableSecret` shims; the orphan rule blocks them for `String` anyway.

## Verification

After a Phase / per-struct change, the diff should show:

- Field types changed to `SecretString` (or `Option<SecretString>`).
- One `.expose_secret()` per cleartext window, located at the wire-format builder.
- No new `?config` or `tracing::*!(config = ?...)` call sites.
- `Clone` and `Serialize` derives dropped where the contained type doesn't support them; standardized doc comment added.
- `garde` annotations switched to `custom(...)` factory form.
- Test fixtures construct via `SecretString::from(...)`.
- `just check` passes — wire contracts unchanged.

## Report format

Group by service / crate. For each:

- **Secret inventory** — list every field that is now `SecretString` and every field still as `String` (with a one-line note explaining why the latter is *not* a secret, or marking it as a candidate for the next phase).
- **`.expose_secret()` call-site map** — each entry as `file:line | reason`. Reviewers can audit cleartext windows at a glance.
- **Clone / Serialize derive deltas** — list every Config struct that lost a derive, with the standardized doc comment present.
- **Cross-reference with infra** — if this workspace has infra-as-code declaring secrets, for each service touched, list the entries that map to wrapped fields. Anything declared there that *doesn't* map to a `SecretString` in this commit is a candidate for the next phase.

End with a one-paragraph note flagging:

- Any field where wrapping is *not* applied because rotation policy or operational context makes it acceptable (escalate before deciding; the default is to wrap).
- Any consumer that legitimately needs `Clone` and forced a structural change (move into Arc<Config>, split-struct, etc.).
- Any test that asserts on a debug-printed Config — those break after the wrap, and the fix is to either remove the assertion or assert on the redacted form.
