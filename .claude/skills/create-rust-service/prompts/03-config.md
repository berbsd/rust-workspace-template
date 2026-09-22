# `config.rs`

A validated `Config` struct loaded from environment variables — replacing the
raw `std::env::var("DATABASE_URL")` / `std::env::var("PORT")` reads
`services/example` uses, with a typed, garde-validated home every future
setting gets added to instead of another ad hoc `std::env::var` call.

Uses `figment`'s `Env` provider (`Figment::new().merge(Env::raw())`) — reads
every environment variable as-is, no prefix, matching the exact
`DATABASE_URL`/`PORT` names this workspace's Dockerfile and docs already
expect. Add a nested struct with `#[serde(rename = "...")]` only once a
second config-owning concern (e.g. an outbound HTTP client) actually exists —
don't pre-build a prefix hierarchy for one flat field set.

## File

### `services/{{service}}/src/config.rs`

```rust
//! {{Service}} service configuration.

use figment::{Figment, providers::Env};
use garde::Validate;
use serde::Deserialize;

/// Default HTTP listener port when `PORT` is unset.
const fn default_port() -> u16 {
  8080
}

/// Root configuration for the standalone `{{service}}` binary.
///
/// Loaded from environment variables only — no config file, matching this
/// workspace's convention (see AGENTS.md's "Service anatomy"). Every field
/// carries a `/// Env: ` doc line naming its variable and default/required
/// status; add one for every field this service grows.
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct Config {
  /// Env: `DATABASE_URL` (required)
  #[garde(length(min = 1))]
  pub database_url: String,

  /// Env: `PORT` (default: 8080)
  #[serde(default = "default_port")]
  #[garde(skip)]
  pub port: u16,
}

/// Error loading or validating [`Config`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
  /// Environment variables didn't deserialize into [`Config`] — usually a
  /// missing required var or a value of the wrong type.
  #[error("failed to load config: {0}")]
  Load(#[from] Box<figment::Error>),
  /// [`Config`] deserialized but failed a `#[garde(...)]` rule.
  #[error("invalid config: {0}")]
  Invalid(#[from] garde::Report),
}

impl Config {
  /// Loads and validates [`Config`] from environment variables.
  ///
  /// # Errors
  /// Returns [`ConfigError::Load`] if a required env var is missing or
  /// malformed, or [`ConfigError::Invalid`] if a loaded value fails a
  /// `#[garde(...)]` rule.
  pub fn load() -> Result<Self, ConfigError> {
    let config: Self = Figment::new().merge(Env::raw()).extract().map_err(Box::new)?;
    config.validate()?;
    Ok(config)
  }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
  use super::*;

  #[test]
  fn rejects_empty_database_url() {
    // SAFETY (test-only): `figment::Jail` would be the isolated way to set
    // env vars for this test, but this workspace has no existing precedent
    // for it — validate the struct directly instead, which exercises the
    // same `#[garde(...)]` rule without touching process env at all.
    let config = Config { database_url: String::new(), port: default_port() };
    let report = config.validate().unwrap_err();
    assert!(report.iter().next().is_some());
  }
}
```

Add a field for every setting `SKILL.md` Step 1 identified beyond
`DATABASE_URL`/`PORT`, each with its own `/// Env: ` line and `#[garde(...)]`
rule — never a bare field with no validation and no doc comment.

## Placeholders used

- `{{service}}` — used only in prose (the doc comment), not in code.

## Verify

```bash
cargo check -p {{service}}
cargo clippy -p {{service}} --no-deps --all-targets --all-features
```

`Load`'s field is `Box<figment::Error>`, not a bare `figment::Error` —
`figment::Error` is over 200 bytes, and an unboxed variant that large
trips `clippy::result_large_err` (denied via `clippy::all`) on every
function returning `Result<_, ConfigError>`. Boxing it is why `load()`'s
body uses `.map_err(Box::new)?` instead of a bare `?` on the `extract()`
call — `#[from]` only generates a conversion from the field's exact type.

Still fails at this step (`feature` and `module` don't exist) — but the
failure should now be scoped to those two missing modules, not to
`config.rs` itself. If `cargo check` reports an error *inside* `config.rs`,
stop and fix it before continuing; don't let a broken step compound into the
next one.
