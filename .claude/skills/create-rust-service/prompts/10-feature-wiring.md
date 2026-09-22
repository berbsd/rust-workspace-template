# Feature Wiring

Ties the five files from `05`–`09` together into one `router(pool)`
function, and registers that function with the rest of the crate. This is
the prompt where `cargo check -p {{service}}` should finally go from
"unresolved module" errors to either a clean pass or real, fixable type
errors inside the feature itself.

## Files

### 1. `services/{{service}}/src/feature/{{feature}}/adapter.rs`

```rust
//! Adapters for the {{feature}} feature: an HTTP adapter (driving) and a
//! Postgres adapter (driven).

pub(crate) mod http;
pub(crate) mod postgres;
```

### 2. `services/{{service}}/src/feature/{{feature}}.rs`

```rust
//! The {{feature}} feature — {{one-line description}}.

mod adapter;
mod domain;
mod error;
mod port;
mod service;

use std::sync::Arc;

use axum::{
  Router,
  routing::{get, post},
};
use sqlx::PgPool;

use self::{
  adapter::{http, postgres::Pg{{Feature}}Repository},
  port::{{Feature}}Repository,
  service::{{Feature}}Service,
};

/// Builds this feature's routes over a fresh Postgres-backed service.
pub(crate) fn router(pool: PgPool) -> Router {
  let repo: Arc<dyn {{Feature}}Repository> = Arc::new(Pg{{Feature}}Repository::new(pool));
  let service = Arc::new({{Feature}}Service::new(repo));
  Router::new()
    .route(
      "/{{feature_plural}}",
      post(http::create_{{feature}}).get(http::list_{{feature_plural}}),
    )
    .route(
      "/{{feature_plural}}/{id}",
      get(http::get_{{feature}}).delete(http::delete_{{feature}}),
    )
    .with_state(service)
}
```

### 3. `services/{{service}}/src/feature.rs`

```rust
//! Feature-first modules. Each `feature/<name>/` is a vertical slice: its
//! own domain entity, storage port, application service, and adapters
//! (HTTP inbound, Postgres outbound), composed by that feature's own
//! `router()` function.

pub(crate) mod {{feature}};
```

If this is the **second or later** feature in the service, add this line
next to the existing `pub(crate) mod <other feature>;` — don't replace it.

### 4. `src/lib.rs`'s router — confirm, don't rewrite

`02-lib-and-main.md` already wrote `lib.rs`'s `router(pool)` with
`.merge(feature::{{feature}}::router(pool))` for this, the service's first
feature. Nothing to change here for feature 1. Adding feature 2+ to an
already-generated service is `13-add-feature.md`'s job, not this one — it
adds a second `.merge(...)` line to this same function.

## Placeholders used

- `{{feature}}` / `{{Feature}}` / `{{feature_plural}}`
- `{{one-line description}}` — from `SKILL.md` Step 1

## Verify

```bash
cargo check -p {{service}} --all-targets --all-features
cargo clippy -p {{service}} --no-deps --all-targets --all-features
```

Both must be clean now — every module the crate declares exists, every type
referenced across `domain.rs`/`port.rs`/`service.rs`/`error.rs`/`adapter/`
resolves. Grep for leftover placeholders before moving on:

```bash
grep -rn '{{' services/{{service}}/src/ && echo "UNRESOLVED PLACEHOLDER — fix before continuing" || echo "clean"
```
