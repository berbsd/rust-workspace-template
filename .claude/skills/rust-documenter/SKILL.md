---
name: rust-documenter
description: "Use when documenting Rust code — writing or improving doc comments, rustdoc, crate-level or module docs, or API documentation for Rust items — including OpenAPI documentation for Rust REST APIs (utoipa annotations, Scalar docs, ToSchema derives, path annotations on axum handlers). Triggers on \"document\", \"add docs\", \"rustdoc\", \"doc comments\", \"document code\", \"crate docs\", \"openapi\", \"utoipa\", \"scalar\", \"api docs\", \"ToSchema\", \"path annotation\", or documentation requests targeting Rust code."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash
---

# Rust Documenter

Document Rust code so the next developer understands intent, contracts, and non-obvious behavior. Idiomatic Rust is largely self-documenting through its type system; documentation provides the context that types and signatures cannot.

## If the project documents an OpenAPI surface

`utoipa` reads `///` doc comments to populate OpenAPI `summary`/`description` fields
automatically, so doc comments stay the single source of truth even when a handler also
carries `#[utoipa::path]`/`#[derive(ToSchema)]` annotations: write the doc comment here,
keep `# Errors` sections in agreement with the annotation's `responses(...)` status codes,
and use the Stripe-style imperative convention on handler summary lines (imperative verb +
article + lowercase noun, no trailing period — `Create a widget`, `List all orders`), not
the third-person form other items use.

For the full annotation reference — `#[utoipa::path]`/`ToSchema`/`IntoParams` mechanics,
wiring utoipa-scalar into a plain axum router (this template has no shared service
framework providing that), the Stripe-style summary convention in full, status-code and
pagination conventions, and the common utoipa pitfalls — read
`references/openapi-annotations.md` before adding or auditing OpenAPI annotations. Load it
on demand; it's not needed for plain rustdoc work.

## Companion Skill: `rust-quality`

The `rust-quality` skill includes checks that complement documentation: Invariant Analysis (#5) documents critical assumptions, and Settings Documentation (#6) catalogs configurable settings. Run those checks after documenting to ensure invariants and settings are covered.

## Core Principles

1. **Comment the WHY, not the WHAT** — rationale, tradeoffs, constraints
2. **Document at the right layer** — crate → module → item → implementation
3. **Every function gets a doc comment** — `pub`, `pub(crate)`, *and* private (AGENTS.md "Rustdoc on every function"). Trivial accessors and documented trait-impl methods are the only exemption
4. **Use standard sections** — `# Examples`, `# Errors`, `# Panics`, `# Safety`
5. **Leverage the type system** — don't repeat what signatures already say
6. **Remove redundant comments** — a comment restating code is noise
7. **Stay grounded in the workspace** — only reference tools, libraries, and systems that actually exist in the project's dependency graph. Never invent or assume technologies the project does not use.

## The Surprise Test

Before adding an inline comment (`//`), ask: **would an experienced Rust developer on this team be surprised by this behavior?**

- **Yes → document it.** Non-obvious side effects, surprising performance, external system quirks, safety invariants, business logic thresholds.
- **No → skip it.** Idiomatic patterns, behavior clear from types/signatures, standard library usage, well-known crate conventions.

This test applies to *inline implementation comments only*. Every function — `pub`, `pub(crate)`, or private — always gets a `///` doc comment regardless.

## The Reintroduction Test

The Surprise Test decides whether to write a comment. This one decides whether a comment about
**the past** may stay.

> A comment describing a past state earns its place only if it stops a reader from
> *re-introducing* that state. If nothing can be re-introduced, git already has the history —
> delete it.

And when it does earn its place, **write it as a present-tense prohibition with its reason**,
not as a dated changelog. The reader needs the rule, not the timeline.

```rust
// Delete — changelog. The rule it trails is already stated above it; the
// date and the diff add nothing a reader can act on.
/// It carried `skip_serializing_if = "Option::is_none"` until 2026-08-10,
/// which contradicted this type's own documentation…

// Keep, but rewrite — this exists to stop someone "fixing" an apparent
// inconsistency. Present tense, no date:
/// **Not cached in-process.** Its only callers write whatever comes back
/// into a shared, push-invalidated cache; a value from a per-instance copy
/// would be persisted fleet-wide and outlive the event meant to clear it.
```

**Passes the test (keep, present tense):** a constraint that looks removable but isn't; a
dependency that must never be paired with another because of a known conflict between them;
an arrangement whose obvious simplification is a known bug.

**Fails the test (delete):** "was renamed from X"; "used to be two methods"; "this previously
returned 400"; "shipped N operations without the line until <date>"; any sentence whose only
content is that the code changed.

**Not history at all (keep as-is):**
- **Spec pointers** — `docs/specs/2026-07-16-milestones-design.md`. The date is a filename.
- **Dated measurements** — `measured on 2026-07-28 (1 vCPU): 121 ms download`. A benchmark
  without a date is *worse*; the date is how a reader judges staleness.
- **Fixture and example data** — `2026-08-03T07:00:00Z` in a doc example.
- **Regression-test rationale** — a test's `///` may say what defect it pins, but state the
  invariant, not the incident: "Every secured operation declares a 401" beats "the member
  handlers shipped 7 operations without one until <date>".

## Workflow

When asked to document a project, work through **all five layers** top-down. When asked to document a specific file or section, apply only the relevant layers.

### 1. Scan the project

Read the project structure before writing anything. Understand:
- Workspace layout (workspace members, crate dependency graph)
- `pub` surface area — what's exported, what's `pub(crate)`
- Error handling strategy (`thiserror` vs `anyhow` vs custom)
- Async runtime and patterns in use
- Feature flags and conditional compilation
- Existing doc comment style and conventions
- **Actual dependencies** — read `Cargo.toml` and `Cargo.lock` to know exactly which crates, tools, and external systems the project uses. Documentation must only reference technologies present in the workspace. Do not mention Kafka, Redis, RabbitMQ, DynamoDB, or any other tool/service unless it appears in the project's dependency graph or configuration.

### 2. Process crates in dependency order

Start from leaf crates (fewest dependencies) and work toward the root binary/app. Within each crate, process files in this order:

1. **`lib.rs` / `main.rs`** → Layer 1 (crate-level `//!` docs)
2. **Module files** → Layer 1 (module-level `//!` docs)
3. **Types: structs, enums, type aliases** → Layer 2 (public API)
4. **Traits and trait impls** → Layer 2 (public API)
5. **Functions, methods, associated functions** → Layer 2 (public API)
6. **Macros** → Layer 2 (public API)
7. **Implementation details** → Layer 3 (inline comments)
8. **Crate README** → Layer 4

### 3. Run Clippy and fix all documentation warnings

After writing documentation, run Clippy and **fix every warning** before considering the documentation complete. Do not leave warnings for the user to address.

```bash
cargo clippy --all-targets --all-features 2>&1
```

Common documentation warnings to fix:
- **Missing backticks** — Clippy warns when type names, function names, or code elements in doc comments are not wrapped in backticks. Fix: `` `MyType` `` not `MyType`.
- **Missing doc sections** — `missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc`. Fix: add the required `# Errors`, `# Panics`, or `# Safety` section.
- **Broken intra-doc links** — misspelled or unresolvable `[`TypeName`]` references. Fix: correct the link target.
- **Missing docs** — `missing_docs` on public items. Fix: add a `///` doc comment.

Run iteratively: fix warnings, re-run Clippy, repeat until clean.

### 4. Run doc build and doc tests

```bash
# Check doc comments compile and links resolve
cargo doc --no-deps --all-features 2>&1 | head -50

# Run doc tests
cargo test --doc
```

## Documentation Layers

### Layer 1: Crate & Module Headers

#### Crate-level docs (`//!` in `lib.rs` or `main.rs`)

Every crate gets `//!` docs at the top of `lib.rs` describing:
- What the crate provides (one sentence)
- Key types and their roles
- Feature flags (if any) with what each enables
- A usage example showing the primary integration pattern

```rust
//! HTTP client for the example service API.
//!
//! Provides [`ExampleClient`] for widget operations and
//! [`SignatureVerifier`] for validating incoming webhook payloads.
//!
//! # Features
//!
//! - `mock` — enables [`MockExampleClient`] for testing
//!
//! # Examples
//!
//! ```no_run
//! use example_client::ExampleClient;
//!
//! # async fn run() -> Result<(), example_client::Error> {
//! let client = ExampleClient::new("https://api.example.internal")?;
//! let widget = client.get_widget("wid_123").await?;
//! println!("Name: {}", widget.name);
//! # Ok(())
//! # }
//! ```
```

#### Module-level docs (`//!` at top of module file)

Modules with more than one or two items get `//!` docs describing:
- What the module provides (one sentence)
- How it relates to the rest of the crate

```rust
//! Request and response types for the example service API.
//!
//! All types implement [`serde::Serialize`] and [`serde::Deserialize`]
//! for JSON transport.
```

**Skip module docs on:** trivial modules that re-export a single type, `tests` modules, `prelude` modules where the crate docs cover it.

### Layer 2: Public API Doc Comments

**Every** `pub` item gets a `///` doc comment. This is not optional. The doc comment must include:

> **This workspace goes further than upstream Rust convention.** AGENTS.md requires a `///`
> summary on `pub`, `pub(crate)`, **and private** functions alike — a private helper is still
> read by the next maintainer, and a one-line summary is cheap. Treat an
> undocumented private fn as a defect, not a style preference. Trivial accessors and
> trait-impl methods the trait already documents are the only exemption.

#### Summary line (required for all `pub` items)

- First sentence: what the item *is* or *does*
- **Non-handler items:** Third-person singular present tense: "Returns", "Creates", "Parses" — ends with a period
- **HTTP handler functions:** a handler's summary line doubles as OpenAPI surface if the project documents one, not just rustdoc prose. Use the Stripe-style imperative convention: imperative verb + article + lowercase noun, no trailing period (`Create a widget`, `List all orders`).

#### Standard sections (include when applicable)

Add these sections in this order, only when they apply. See `references/rustdoc-sections.md` for detailed guidance.

| Section | When to include |
|---------|----------------|
| `# Examples` | All public items that can be demonstrated. Always include for public functions, methods, and types. |
| `# Errors` | Any function returning `Result`. List each error variant and when it occurs. |
| `# Panics` | Any function that can panic. Document all panic paths. |
| `# Safety` | All `unsafe fn`. List every invariant the caller must uphold. |

#### What to document per item type

**Structs:**
- What it represents in one sentence
- Fields with non-obvious meaning or constraints (use field-level `///`)
- Builder patterns if applicable
- `# Examples` showing construction and primary usage

**Enums:**
- What the enum represents
- Each variant gets `///` — what it means and when it occurs
- Especially important for error enums: document when each variant is returned

**Functions and methods:**
- What it does (summary line)
- Parameters with non-obvious constraints — mention them in prose, wrap names in backticks
- Return value semantics when not obvious from the type
- `# Examples` with a runnable doc test
- `# Errors` if returns `Result` — list each error variant
- `# Panics` if it can panic — list conditions

**Traits:**
- What capability or contract it represents
- When to implement it
- Document methods on the *trait declaration*, not on impls
- `# Examples` showing both implementation and usage

**Trait implementations:**
- Do NOT re-document methods that the trait already documents
- Add impl-specific docs only if behavior is notably different or has specific performance characteristics

**Type aliases:**
- What it represents and why the alias exists
- Link to the underlying type

**Constants and statics:**
- What the value represents
- Why this specific value (if not self-evident)

**Macros:**
- Show all supported invocation forms
- `# Examples` for each form

**Feature-gated items:**
- Use `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]` so docs.rs shows the required feature

#### What to skip

- `pub(crate)` and private **functions** are NOT skippable in this workspace — a one-line `///`
  summary is the floor, with `# Errors`/`# Panics`/`# Safety` where they apply. Keep them terse;
  a private helper rarely needs `# Examples`
- Private non-function items (fields on private structs, internal type aliases) — skip unless the
  meaning is non-obvious
- Trait impl methods where the trait's docs are sufficient
- Generated code (derive macro output, protobuf, etc.)
- Test modules and test functions
- Simple `From`/`Into` impls where the conversion is obvious from the types

### Layer 3: Complex Implementation Comments

Apply the **surprise test** to all implementation code. Add `//` inline comments only where behavior is genuinely non-obvious:

- **Business logic rationale** — why this threshold, formula, or sequence
- **Performance choices** — why this algorithm, data structure, or allocation strategy
- **External system quirks** — workarounds for API bugs, protocol oddities
- **Safety invariants** — preconditions, postconditions, invariants maintained
- **`// SAFETY:`** — mandatory for every `unsafe` block, explains why invariants hold
- **`// TODO(#issue):`** — tracked work with issue reference. Delete TODOs without issue refs.
- **`// HACK:`** — temporary workarounds with explanation
- **Async lifecycle** — detached tasks, non-obvious cancellation, cleanup ordering

See `references/async-unsafe-guide.md` for async and unsafe documentation patterns.

**Actively remove** comments that:
- Restate what the code does (`// increment counter`)
- Explain standard library or well-known crate usage
- Describe Rust language features
- Are commented-out code (delete it)
- Narrate a past change without preventing its return (see "The Reintroduction Test")
- Carry a `TODO(REF)` whose `REF` no longer resolves — verify the file or issue still exists;
  a dangling ref is worse than no ref, because it reads as tracked work that nobody tracks

### Layer 4: Crate READMEs

Each independently publishable or consumed crate gets a README. Structure:

```markdown
# {crate_name}

{One-sentence description.}

## Usage

Add to your `Cargo.toml`:

\`\`\`toml
[dependencies]
{crate_name} = "x.y.z"
\`\`\`

{Primary usage pattern with a concise code example.}

## Architecture

{Internal layers and responsibilities. Diagram if 3+ layers.}

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `mock` | Enables mock client for testing | No |

## Testing

\`\`\`sh
cargo test -p {crate_name}
\`\`\`
```

Keep READMEs under 120 lines. Optionally include the README as a doc test:

```rust
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
```

### Layer 5: Lint Configuration

Ensure the crate or workspace has documentation lints enabled. Recommend adding to `lib.rs` or workspace `Cargo.toml`:

**In `lib.rs`:**
```rust
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
```

**In workspace `Cargo.toml`:**
```toml
[workspace.lints.rust]
missing_docs = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"

[workspace.lints.clippy]
missing_safety_doc = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
```

## Doc Comment Formatting Rules

1. Use `///` for item docs. Use `//!` for crate/module inner docs. Never use `/** */`.
2. Summary line: third-person present tense ending with a period for non-handler items. **HTTP handlers use the Stripe-style imperative convention** (see above) — never the third-person form.
3. Separate summary from body with a blank `///` line.
4. Use **intra-doc links** for all cross-references: `[`Option`]`, `[`Vec::push`]`, `[`std::io::Error`]`.
5. Wrap code elements in backticks: `` `None` ``, `` `self` ``, `` `n` ``.
6. Use `#` to hide boilerplate in doc test examples (imports, main fn, error handling).
7. Use `?` instead of `unwrap()` in examples.
8. Keep doc comment lines under 100 characters.

## Examples

### Layer 1: Crate-level docs

```rust
//! Rate limiter using the token bucket algorithm.
//!
//! Provides [`RateLimiter`] for controlling request throughput and
//! [`RateLimitConfig`] for configuring bucket parameters per endpoint.
//!
//! # Examples
//!
//! ```no_run
//! use rate_limiter::{RateLimiter, RateLimitConfig};
//!
//! # async fn run() -> Result<(), rate_limiter::Error> {
//! let limiter = RateLimiter::new(RateLimitConfig::default());
//! limiter.acquire("api/users").await?;
//! # Ok(())
//! # }
//! ```
```

### Layer 2: Struct with methods

```rust
/// HTTP client for the example service API.
///
/// Manages connection pooling and automatic retry with exponential backoff.
/// All methods are safe to call from multiple tasks concurrently.
///
/// # Examples
///
/// ```no_run
/// # use example_client::ExampleClient;
/// # async fn run() -> Result<(), example_client::Error> {
/// let client = ExampleClient::new("https://api.example.internal")?;
/// let widget = client.get_widget("wid_123").await?;
/// # Ok(())
/// # }
/// ```
pub struct ExampleClient { /* ... */ }

impl ExampleClient {
    /// Creates a new client connected to the given `base_url`.
    ///
    /// The URL must include the scheme (`https://`). The client uses a
    /// shared connection pool with a 30-second idle timeout.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] if `base_url` cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use example_client::ExampleClient;
    /// let client = ExampleClient::new("https://api.example.internal")?;
    /// # Ok::<(), example_client::Error>(())
    /// ```
    pub fn new(base_url: &str) -> Result<Self, Error> { /* ... */ }

    /// Retrieves the widget with the given `id`.
    ///
    /// Returns `None` if the widget does not exist. Retries transient
    /// network errors up to 3 times with exponential backoff.
    ///
    /// # Errors
    ///
    /// - [`Error::Auth`] — API key is invalid or expired.
    /// - [`Error::Network`] — all retry attempts failed.
    /// - [`Error::Decode`] — response body is not valid JSON.
    pub async fn get_widget(&self, id: &str) -> Result<Option<Widget>, Error> { /* ... */ }
}
```

### Layer 2: Error enum

```rust
/// Errors returned by [`ExampleClient`] operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The base URL could not be parsed.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// The API key is invalid or has been revoked.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// A network request failed after exhausting retries.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The response body could not be deserialized.
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
}
```

### Layer 2: Trait

```rust
/// Persistent storage for rate limit state.
///
/// Implementations must be safe to use from multiple tasks concurrently.
/// State updates must be atomic — partial updates violate the rate limit
/// contract.
///
/// # Examples
///
/// ```
/// use rate_limiter::{Store, MemoryStore};
///
/// # async fn run() -> Result<(), rate_limiter::Error> {
/// let store = MemoryStore::new();
/// store.record_request("api/users").await?;
/// let count = store.request_count("api/users").await?;
/// assert_eq!(count, 1);
/// # Ok(())
/// # }
/// ```
pub trait Store: Send + Sync {
    /// Records a request against the given `key`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`] if the backend is unavailable.
    async fn record_request(&self, key: &str) -> Result<(), Error>;

    /// Returns the number of requests recorded for `key` in the
    /// current window.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`] if the backend is unavailable.
    async fn request_count(&self, key: &str) -> Result<u64, Error>;
}
```

### Layer 3: Inline comments

```rust
// Bad: restates code
// Create a new HashMap
let map = HashMap::new();

// Bad: explains standard library
// Spawn an async task
tokio::spawn(process(item));

// Good: business logic rationale
// 429 responses use the server's Retry-After header when present.
// The example API returns seconds (not a date), capped at 60s by contract.
let delay = response
    .headers()
    .get("retry-after")
    .and_then(|v| v.to_str().ok())
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(5);

// Good: SAFETY comment on unsafe block
// SAFETY: `ptr` was allocated by `Vec::into_raw_parts` with the same
// layout, and `len` and `cap` are the original values. The Vec is
// not used after this point.
let data = unsafe { Vec::from_raw_parts(ptr, len, cap) };
```

### Comments to remove

```rust
// Bad: type tells you this
/// The user's name.
pub name: String,

// Bad: explains Rust syntax
/// Implements Display for Error.
impl fmt::Display for Error { /* ... */ }

// Bad: default behavior
/// Creates a new default Config.
impl Default for Config { /* ... */ }

// Bad: restates the derive
/// Cloneable.
#[derive(Clone)]
pub struct Foo;

// Good: no comment needed for any of these — self-documenting
```

## Checklist

Before completing documentation, verify:

- [ ] Every `pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`, `pub const`, `pub static`, and `pub macro` has a `///` doc comment
- [ ] Every `pub(crate)` and **private** `fn` has at least a `///` summary line (AGENTS.md rule)
- [ ] Every function returning `Result` has `# Errors` listing each variant
- [ ] Every function that can panic has `# Panics`
- [ ] Every `unsafe fn` has `# Safety`
- [ ] Every `unsafe` block has a `// SAFETY:` comment
- [ ] `lib.rs` has `//!` crate-level docs
- [ ] Non-trivial modules have `//!` module docs
- [ ] Error enum variants each have `///` explaining when they occur
- [ ] Intra-doc links are used (not raw URLs or plain text references)
- [ ] Doc tests compile (`cargo test --doc`)
- [ ] No broken links (`cargo doc --no-deps 2>&1 | grep warning`)
- [ ] Clippy passes with zero documentation warnings (`cargo clippy --all-targets --all-features`)
- [ ] All code elements in doc comments are wrapped in backticks
- [ ] Documentation only references crates, tools, and systems present in the workspace
- [ ] Redundant comments removed
- [ ] Every comment about a past state passes the Reintroduction Test, and is written as a
      present-tense prohibition rather than a dated changelog
- [ ] No `TODO(REF)` whose `REF` does not resolve to an existing file or issue
- [ ] If this service documents an OpenAPI surface, also run the checklist in
      `references/openapi-annotations.md`
