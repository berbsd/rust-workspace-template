# Path Conventions

Drive consistency with the 8 path-style rules in AGENTS.md → **Path Conventions** across `crates/` and `services/`. Most rules are *not* lint-enforceable today — this prompt is the human/LLM substitute for clippy.

## Why

clippy enforces wildcard imports (`wildcard_imports`, `enum_glob_use`) and a few hygiene checks, but cannot tell you when a path is over-qualified at the call site, when an enum variant was imported by full path instead of via its enum, when a non-canonical re-export was used, or when `super::super::` chains have crept into crate-local paths. These are the readability defects that make a stranger walking into a service file think "where did *that* come from?" — and they accumulate silently.

## Cross-references (do not duplicate)

- **`use foo::*;` and `use Enum::*;` floor** — `clippy::wildcard_imports` and `clippy::enum_glob_use` are at `warn` in workspace lints. If clippy is clean, rule 7 is satisfied. This prompt owns the cases clippy *can't* see.
- **`mod.rs` vs sibling-file layout** — owned by AGENTS.md → Working Rules (and the parent skill's `15-skeleton-consistency.md`). Don't flag here.
- **Router composition / `crate::router::*` shape** — owned by `16-route-isolation.md`. Don't flag here.

## Scope

Scan all `*.rs` files in `crates/` and `services/`. Skip `target/`, `tests/e2e/`, and generated files (`*.gen.rs`, `OUT_DIR`).

Run **one service or crate at a time**. Producing a workspace-wide finding list at once produces a diff too large to review safely; the convention is forward-looking and the sweep should follow the order from AGENTS.md → Skills (shared crates first, then the reference service, then opportunistically as feature work touches each remaining service).

## Workflow

### 1. Establish the floor with clippy

```bash
cargo clippy -p <crate-name> --no-deps --all-targets --all-features
```

If `wildcard_imports`, `enum_glob_use`, `single_component_path_imports`, or `unnecessary_self_imports` warn, fix those *first*. They're mechanical, and the rest of the sweep is easier once the wildcards are gone (no more guessing where `Foo` came from).

### 2. Grep-driven mechanical findings

Run each command from the crate root. Group results by file. Each line is a candidate — confirm against the rule before fixing (some are false positives in macros / test-only code).

### 3. Walk the file visually for judgment-call findings

For each `*.rs` file with grep hits, open the file and look at the `use` block plus the first few call sites. The judgment-call rules (one-level qualification, canonical re-exports) are not greppable with confidence — they need a reader.

### 4. Apply fixes in a single bounded PR per crate/service

Don't open dedicated path-cleanup PRs for cold services. For active services, fold the cleanup into the next feature PR. The convention is forward-looking; existing inconsistency converges as files are touched.

## Patterns to flag

### 1. `use Enum::VARIANT;` — enum variant imported by full path

Rule 3 says: `use` the enum, then `Enum::Variant` at the call site. The anti-pattern is importing the variant directly, which strips the type context.

```rust
// Bad — what enum is NOT_FOUND from? StatusCode? a service enum?
use axum::http::StatusCode::NOT_FOUND;

// Good
use axum::http::StatusCode;
// ... StatusCode::NOT_FOUND
```

```bash
# UPPER_SNAKE constants in import paths — the SCREAMING_CASE leaf is the tell
grep -RnE '\buse\s+([a-z_][a-z0-9_:]*::)+[A-Z][A-Za-z0-9_]+::[A-Z_][A-Z0-9_]+\s*;' --include='*.rs'

# CamelCase variant in import paths — second tier (false-positive-prone since
# CamelCase leaf can be a re-exported type; eyeball each hit)
grep -RnE '\buse\s+([a-z_][a-z0-9_:]*::)+[A-Z][A-Za-z0-9_]+::[A-Z][A-Za-z0-9_]+\s*;' --include='*.rs'
```

The first grep is high-signal (constants and SCREAMING_CASE enum variants). The second needs visual review — `use module::TraitName::AssocConst` looks identical to `use module::EnumName::Variant`.

### 2. Bare function calls that should be one-level qualified

Rule 2: prefer `fs::read_to_string(...)` over both `read_to_string(...)` (no prefix) and `std::fs::read_to_string(...)` (full path). The single-segment prefix names the origin without bloating the call site.

This rule is **not greppable** with confidence — you need to know which symbols are functions vs types vs macros. Walk each `use` block visually and ask:

- Does the file `use std::fs::read_to_string;` then call `read_to_string(path)`? → flag. Change to `use std::fs;` then `fs::read_to_string(path)`.
- Does the file `use tokio::time::sleep;` then call `sleep(d).await`? → flag. Change to `use tokio::time;` then `time::sleep(d).await`.
- Does the file write `std::fs::read_to_string(path)` inline? → flag. Add `use std::fs;` and shorten.

**Exception:** functions whose name already carries context (`Vec::new`, `String::from`, `HashMap::new` — these are `Type::method` constructors, *not* free functions; leave them). Trait methods called via UFCS (`<T as Trait>::method`) stay as-is when needed to disambiguate.

### 3. Fully-qualified types in signatures, fields, bounds

Rule 1: `use` the type and refer by short name. Full paths only to disambiguate (e.g. `std::result::Result` inside a module that aliases `Result`).

```bash
# Common offenders — std types fully qualified
grep -RnE '\b(std|core|alloc)::(sync|collections|time|path|net|fs|io|cell)::[A-Z][A-Za-z0-9_]+' --include='*.rs' | \
  grep -vE '^\s*(use |//|/\*|\*)'

# tokio types fully qualified
grep -RnE '\btokio::(sync|task|time|net|fs|io)::[A-Z][A-Za-z0-9_]+' --include='*.rs' | \
  grep -vE '^\s*(use |//|/\*|\*)'

# axum / serde / chrono types fully qualified
grep -RnE '\b(axum|serde|chrono|uuid)::[a-z_][a-z0-9_]+::[A-Z][A-Za-z0-9_]+' --include='*.rs' | \
  grep -vE '^\s*(use |//|/\*|\*)'
```

The `grep -v` strips out the `use` lines themselves (where full paths are correct) and comments. Hits in code are candidates for shortening: add a `use`, drop the prefix.

**Exception:** disambiguation. `std::result::Result<T, ServiceError>` inside a module that defines `type Result<T> = std::result::Result<T, ServiceError>;` is intentional and stays.

### 4. Non-canonical re-export paths

Rule 6: pull from the canonical re-export, not the inner module. Each major dependency in this workspace has a "public" surface and an "internal" surface; using the internal one couples the call site to internal layout.

```bash
# Known non-canonical paths in this workspace's dependency set
grep -RnE '\btokio::sync::(mutex|rwlock|mpsc|oneshot|broadcast|notify|semaphore|watch)::[A-Z]' --include='*.rs' | \
  grep -vE '^\s*//'
grep -RnE '\baxum::extract::(path|query|json|state|connect_info|matched_path)::[A-Z]' --include='*.rs' | \
  grep -vE '^\s*//'
grep -RnE '\bserde::(ser|de|de_::|ser_::)::[A-Z]' --include='*.rs' | grep -vE '^\s*//'
```

Canonical forms:

| Bad | Good |
|-----|------|
| `tokio::sync::mutex::Mutex` | `tokio::sync::Mutex` |
| `tokio::sync::mpsc::Sender` | `tokio::sync::mpsc::Sender` (correct — `mpsc` is canonical) |
| `axum::extract::path::Path` | `axum::extract::Path` |
| `axum::extract::query::Query` | `axum::extract::Query` |
| `serde::ser::Serialize` | `serde::Serialize` |
| `serde::de::Deserialize` | `serde::Deserialize` |
| `chrono::naive::date::NaiveDate` | `chrono::NaiveDate` |

When in doubt, check the dependency's `lib.rs` re-exports — that's the canonical surface.

### 5. `super::super::` (and deeper) crate-local paths

Rule 8: cap relative `super::` at one level. Once it would chain, reach for `crate::`.

```bash
grep -RnE '\b(super::){2,}' --include='*.rs'
```

Every hit is a candidate. The fix is `crate::feature::<name>::<thing>` (or wherever the symbol actually lives — let the LSP help). Single `super::` is fine and stays.

### 6. `clippy::single_component_path_imports` survivors

Rule covered by the lint, but if anyone has `#[allow]`-suppressed it locally, this catches the leftover:

```bash
grep -RnE '^\s*use\s+[a-z_][a-z0-9_]*\s*;' --include='*.rs'
```

Hits like `use tracing;` are noise — `tracing::info!(...)` works at the call site without the import. Delete the `use`.

### 7. `use Trait;` missing where trait methods are called via UFCS

Rule 4: prefer `use Trait;` then `value.method()` over `<T as Trait>::method(value)`. UFCS is reserved for genuine disambiguation between two traits with the same method name.

```bash
# Heuristic — UFCS call sites. Many will be legitimate disambiguation; verify each.
grep -RnE '<[A-Za-z_][A-Za-z0-9_:<>]+\s+as\s+[A-Za-z_]' --include='*.rs'
```

For each hit, check: does this file `use` the trait? If yes, can the call site become `value.method()`? If no, is there an actual name collision that forces UFCS? Document the collision with a one-line comment, or shorten.

### 8. `use foo::{self};` and self-import noise

Rule covered by `clippy::unnecessary_self_imports`, but worth a manual check for variants the lint may miss in odd-shaped `use` trees:

```bash
grep -RnE '\buse\s+[a-z_:]+::\{[^}]*\bself\b[^}]*\}' --include='*.rs'
```

`use std::io::{self, Read};` is legitimate — `self` is bringing in the `io` module alongside the `Read` trait. Flag only the *single-element* `{self}` forms.

## Fixing

Pick the narrowest fix per finding:

1. **Add a `use` and shorten the call site** — most common path. `use tokio::time;` then `time::sleep(d)`.
2. **Replace a full path with the canonical re-export** — `use axum::extract::Path;` instead of `axum::extract::path::Path`.
3. **Switch from `Enum::Variant` import to `Enum` import** — `use axum::http::StatusCode;` then `StatusCode::NOT_FOUND` at the use-site.
4. **Replace `super::super::*` with `crate::*`** — let the LSP find the destination.
5. **Delete a no-op import** — single-component imports of items already at root scope.
6. **Document the exception** — when a full path is intentional (disambiguation, macro hygiene), leave a one-line comment so the next sweep skips it.

Never:
- Mass-rewrite imports across multiple crates in one PR. Stay within a single crate or service.
- Add `#[allow(clippy::wildcard_imports)]` to suppress a finding. The lint is load-bearing; if the wildcard is *truly* required (prelude module designed for it), the silencing comment explains why and references the prelude.
- Introduce a new `prelude` module to make wildcards palatable. The convention is "use what you import"; new preludes are a separate design conversation.

## Verification

After fixes for a given crate/service:

```bash
cargo +nightly fmt -p <crate-name>
cargo check -p <crate-name> --all-targets --all-features
cargo clippy -p <crate-name> --no-deps --all-targets --all-features
cargo nextest run -p <crate-name> --all-features
```

For workspace-wide impact (rare — only when touching shared crates):

```bash
cargo +nightly fmt
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
cargo nextest run --workspace --all-features
```

If `cargo +nightly fmt` reorders imports in ways that conflict with the cleanup (it shouldn't, given the workspace's `group_imports = "StdExternalCrate"`), run fmt *first*, then re-grep — many findings will disappear.

## Report format

Group by rule (1–8), then by file. For each finding give file:line, the offending one-liner, the rule violated, and the chosen fix.

End with a per-rule count for the crate/service:

```
rule | count | examples
1    |   3   | services/example/src/feature/widget/handler.rs:42
2    |  12   | crates/common-types/src/pagination.rs:55,88,101 ...
3    |   1   | services/example/src/feature/widget/handler.rs:18
4    |   2   | services/example/src/feature/widget/repository.rs:70
5    |   0   |
6    |   0   |
7    |   1   | crates/common-types/src/errors.rs:73 (intentional UFCS — documented)
8    |   0   |
```

Followed by a one-paragraph summary of which fixes were applied, which were left as documented exceptions, and any patterns you saw that suggest the *convention itself* needs amendment (escalate those — don't silently update AGENTS.md from inside the sweep).
