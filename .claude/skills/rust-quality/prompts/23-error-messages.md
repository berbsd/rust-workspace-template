# Error Message Consistency

Bring `#[error("...")]` strings and inline error constructions across `crates/` and `services/` into a single style. This is about *message text*, not enum shape, framework choice, or leakage — those belong to sibling prompts.

## Why

Most domain error variants flow to the wire as `ApiErrorBody.message`. The workspace has ~315 `#[error(...)]` strings; today they drift in casing (Title vs lowercase, ratio ~19/71), in tone ("Failed to" vs "Could not" vs "Unable to"), and in wording for identical semantics (`"User not found"` vs `"user not found"` vs `"unknown user"`). The result: a client doing fuzzy matching on `.message` sees different shapes from different services, and the SPA's error toast is inconsistent across the product. None of this is enforceable by clippy.

## Cross-references (do not duplicate)

- **Enum shape, variant order, `#[from]` conversions, `ApiErrorMapping` impl** → `13-error-pattern.md`. This prompt does not touch the enum's structure.
- **Garde validation framework, `validate(&())?`, `Validation(#[from] garde::Report)` variant** → `19-garde-validation.md`. This prompt accepts garde's *default* messages as-is; only flag where a custom override violates the style rules.
- **Response leakage, hidden DB errors, sensitive-info redaction** → `21-security.md`. Database-error variants must hide details (rule below); the *style* is owned here, the *requirement to hide* is owned there.
- **Swallowed errors / `.map_err` discarding source** → `17-swallowed-errors.md`. Out of scope here.

## Scope

Scan all `*.rs` files under `crates/` and `services/`. Skip `target/`, `tests/e2e/`, and `*.gen.rs`.

Sources to read:

1. `#[error("...")]` attributes on `thiserror::Error` enums (primary).
2. Inline error construction: `anyhow::anyhow!(...)`, `anyhow::bail!(...)`, `.context("...")`, `.with_context(|| "...")`.
3. `String::from("...")`, `format!("...")` strings passed into error variants.
4. Custom garde overrides: `#[garde(... message = "...")]`, `#[garde(custom = ...)]` where the closure builds a message.
5. Doc-comment examples that *demonstrate the framework* — particularly `crates/common-types/src/errors.rs` — must use the canonical style so readers don't learn the wrong convention.

## The style rules

### 1. Lowercase the sentence start; preserve proper nouns and acronyms

```rust
// Bad
#[error("Invalid configuration: {0}")]
#[error("Token validation failed: {0}")]
#[error("Unable to retrieve JWKS: {0}")]   // capital U at start is the smell
#[error("User not found: {0}")]

// Good
#[error("invalid configuration: {0}")]
#[error("token validation failed: {0}")]
#[error("unable to retrieve JWKS: {0}")]   // JWKS stays — it's an acronym
#[error("user not found: {0}")]
```

**Acronyms and proper nouns stay capitalized:** `HTTP`, `JWKS`, `JWT`, `JSON`, `SQL`, `URL`, `URI`, `API`, `OAuth`, `OIDC`, `OTP`, `MFA`, `CSRF`, `TOTP`, `S3`, `GCS`, `Stripe`, `Gemini`, `PostgreSQL`, `Redis`, `Twilio`, `Vertex AI`, service names from `crates/<name>` when referenced literally. When in doubt, look at how the rest of the codebase writes the same token.

The sentence-start lowercasing is the *only* casing change. Don't downcase acronyms mid-sentence.

### 2. No trailing period

```rust
// Bad
#[error("an account with this email already exists. Please sign in with your original provider.")]

// Good
#[error("an account with this email already exists — sign in with your original provider")]
```

Error messages are not sentences in the prose sense; they are labels. The trailing period reads as a hard stop and stutters when wrapped in client UI ("Failed: not found." vs "Failed: not found").

### 3. Colon-separated context, not em-dash and not parenthesized

```rust
// Bad
#[error("Provider error: {error} - {description}")]   // ASCII hyphen separator
#[error("Provider error: {error} – {description}")]   // en-dash
#[error("not found ({id})")]                          // parenthesized id

// Good
#[error("provider error: {error}: {description}")]
#[error("not found: {id}")]
```

Single colon between the kind and the detail; further colons between detail and sub-detail. Don't mix `-`, `–`, `—` as separators. Em-dash (`—`) is fine for an *aside in prose* (rule 2 example uses it), not as a label separator.

### 4. Use the canonical form for each error family

The codebase already has a dominant wording for each semantic family; deviating creates duplicates that are functionally identical but textually different. Match the majority.

| Semantic | Canonical | Note |
|----------|-----------|------|
| 404 not found | `"{resource} not found"` or `"not found: {0}"` | Singular resource word: `"user not found"`, `"widget not found"`, `"invite not found"`. Use `"not found: {0}"` only when the wrapped value identifies the resource generically. |
| 400 bad request | `"bad request: {detail}"` | The caller supplies the detail; this prompt only flags Title-case or duplicate-semantic prefixes. |
| 400 validation | `"validation failed: {0}"` | Always sourced from `Validation(#[from] garde::Report)`. Don't write a custom string. |
| 401 unauthenticated | `"unauthenticated: {detail}"` or `"unauthenticated"` | Distinct from 403. Reserve for missing/invalid credentials. |
| 401 token invalid | `"token validation failed: {0}"` or `"invalid token: {0}"` | Two variants are fine — `"invalid token"` for tampering/format errors, `"token validation failed"` for exp/aud/iss/nbf failures. |
| 403 forbidden | `"forbidden: {detail}"` | Not `"access denied"`, not `"permission denied"`. Use when the user is authenticated but lacks permission. |
| 403 not entitled | `"not entitled: {detail}"` | Domain-specific: plan-tier denial. Keep as separate canonical, don't merge with `forbidden`. |
| 409 conflict | `"conflict: {detail}"` or `"{resource} already {state}"` | Specific is fine when the state is canonical (`"already a member"`, `"already registered"`). Avoid `"X conflict"` (back-to-front). |
| 410 gone | `"gone: {detail}"` | Used for revoked/expired resources where the URI is no longer valid. |
| 429 rate limit | `"rate limit exceeded"` or `"rate limit exceeded: {detail}"` | Always lowercase. |
| 500 internal (hide DB) | `"an internal error occurred"` | Database/sqlx variants. **Never** include the source error in the message — that's owned by `21-security.md`. |
| 500 internal (with context) | `"internal error: {0}"` | Non-DB application errors where the context is safe to surface. |
| Configuration | `"invalid configuration: {0}"` | Startup-time, but still lowercase. |

When you find a variant that doesn't match its row in this table, the fix is to rename the string (not the enum variant — variant identifiers are owned by `13-error-pattern.md`).

### 5. Don't write the same error two ways in two files

Before adding a new variant, grep for the semantic across `crates/` and `services/`. If a synonym exists, match its wording exactly.

```rust
// Bad — two crates, same meaning, different strings
// crates/one-crate/src/error.rs
#[error("failed to build HTTP client: {0}")]
// crates/another-crate/src/error.rs
#[error("HTTP client build error: {0}")]   // synonym; should match

// Good — both files
#[error("failed to build HTTP client: {0}")]
```

The merger rule: prefer the form used by *more files*. Ties broken by the form closer to the table above (e.g., "failed to X: {0}" beats "X error: {0}" because the failure verb gives the reader a tense).

### 6. Interpolation placement is `prefix: {detail}`

```rust
// Bad
#[error("Widget {id} not found")]
#[error("not found Widget {id}")]

// Good
#[error("widget not found: {id}")]
```

The kind comes first as a stable prefix; the variable comes last as the detail. Clients can match on the prefix without parsing variable content.

### 7. Doc-comment examples follow the same rules

```rust
//! Bad — teaches Title-case to readers
//! ```
//! #[error("Database error: {0}")]
//! #[error("User not found")]
//! ```
```

Doc-comment examples in `crates/common-types/src/errors.rs` (the framework docs) and any service-level `//!` block must use the canonical style. Otherwise the next contributor learns the wrong convention from the documentation.

## Workflow

### 1. Inventory per crate

```bash
grep -RnE '#\[error\("[^"]+"\)' --include='*.rs' crates/<name>
grep -RnE '#\[error\("[^"]+"\)' --include='*.rs' services/<name>
```

Capture each as `file:line | variant | "message"`. Don't fix yet — read everything first so you see duplicates before renaming.

### 2. Bucket findings

For each crate/service, group strings by:

- **Casing** — Title vs lowercase start.
- **Family** — not-found, bad-request, validation, forbidden, internal, config, token, conflict, rate-limit, other.
- **Duplicates** — strings with identical semantics but different wording (across crates, not just within).

### 3. Apply the canonical table

For each family, pick the canonical from the table in rule 4. Rename non-conforming strings to match. Preserve acronyms (rule 1).

### 4. Look outside `#[error(...)]`

Run the same audit against:

```bash
grep -RnE 'anyhow::(anyhow|bail)!\("[A-Z]' --include='*.rs' crates/ services/
grep -RnE '\.context\("[A-Z]' --include='*.rs' crates/ services/
grep -RnE '\.with_context\(\|\|\s*"[A-Z]' --include='*.rs' crates/ services/
```

Title-case starts in `anyhow!`/`bail!`/`context` are the next-tier offenders. Same rule applies — lowercase the sentence start, preserve acronyms.

### 5. Verify nothing matches on the literal string

Before committing, grep for any tests that pattern-match on the *exact* old message string:

```bash
grep -RnE 'assert.*"User not found"|assert.*"Invalid configuration"' --include='*.rs' tests/ services/*/tests/ crates/*/tests/
```

Update test assertions in the same commit. Clients that pattern-match on `.message` are out of scope here — the API contract is `error_code` (the JSON `error` field), not `message`. If a downstream client *does* match on message, that's a client bug to fix separately, but flag it.

## Fixing

1. **Lowercase rewrites** — pure mechanical text change. Low risk. Do these first.
2. **Acronym preservation** — when lowercasing, do a second pass to re-capitalize known acronyms inside the string. Use the acronym list in rule 1; extend with project-specific tokens as you find them.
3. **Punctuation cleanup** — remove trailing periods, swap em-dash separators to colons. Low risk.
4. **Family unification** — replace synonyms with the canonical form from the table. Medium risk if tests assert on text; check rule 5 grep before each rename.
5. **Doc-comment example sync** — update `//!` blocks that teach the framework to use the canonical style.

Never:

- Mass-find-replace across crates without reading the surrounding context. `"User not found"` in `crates/common-types/src/errors.rs` is a *doc example* and gets updated; `"User not found"` in `services/example/src/domain/error.rs` is a *real variant* that flows to the wire — both get the same fix, but only after reading.
- Rename the enum variant identifier itself (`UserNotFound`, `TokenValidationFailed`, etc.). Variant names are owned by `13-error-pattern.md` and changing them breaks callers.
- Translate / shorten / "improve" error messages while you're at it. The goal is consistency, not editorial rewriting.

## Verification

After fixes for a given crate/service:

```bash
cargo check -p <crate-name> --all-targets --all-features
cargo clippy -p <crate-name> --no-deps --all-targets --all-features
cargo test -p <crate-name>
```

For workspace-wide impact:

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
cargo test --workspace
```

Tests that assert on error message text will fail when the text changes — that's the point of running them. Update the assertions in the same commit; if a test asserts on the *old* text in a way that suggests the contract is the text itself, escalate to the user before changing.

## Report format

Group by family (rule 4 table). For each family, give:

- Canonical form selected.
- Files containing non-conforming strings, with `file:line` and the current text.
- Proposed new text per variant.

End with a per-crate matrix:

```
                | Title→lower | period | em-dash→colon | family unify | doc-example
common-types    |      3      |   0    |       0       |      0       |     5
example         |      3      |   0    |       0       |      0       |     0
<other crate>   |     ...     |  ...   |      ...      |     ...      |    ...
```

Followed by one short paragraph noting:
- Any *test* changes triggered by the string updates (rule 5 hits).
- Any string you proposed to canonicalize but where the existing form is *also* defensible (escalate — don't silently pick).
- Any newly discovered acronym/proper-noun that should be added to the rule 1 list.
