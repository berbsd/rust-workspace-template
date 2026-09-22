# Crate Scaffold

Creates the new service's crate: `Cargo.toml`, the directory tree, and its
root-workspace registration. Nothing here is feature-specific — this step
runs exactly once per service, never again when a feature is added later.

## Files

### 1. `services/{{service}}/Cargo.toml`

```toml
[package]
authors.workspace = true
description       = "{{description}}"
edition.workspace = true
license.workspace = true
name              = "{{service}}"
publish.workspace = true
version.workspace = true

[[bin]]
# Must equal the package name: the generic Docker image builds `--bin
# ${SERVICE}` and copies `target/release/${SERVICE}`, so a binary named
# anything else cannot be built into an image at all.
name = "{{service}}"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
common-types = { workspace = true, features = ["garde"] }

async-trait        = { workspace = true }
axum               = { workspace = true }
chrono              = { workspace = true, features = ["serde"] }
figment              = { workspace = true }
garde                 = { workspace = true, features = ["derive"] }
serde                  = { workspace = true, features = ["derive"] }
serde_json               = { workspace = true }
sqlx                       = { workspace = true, features = ["chrono", "macros", "migrate", "postgres", "runtime-tokio", "uuid"] }
thiserror                    = { workspace = true }
tokio                          = { workspace = true, features = ["macros", "rt-multi-thread", "signal"] }
tracing                          = { workspace = true }
tracing-subscriber                 = { workspace = true, features = ["env-filter"] }
uuid                                  = { workspace = true, features = ["serde", "v7"] }

[dev-dependencies]
serde_json = { workspace = true }
tokio      = { workspace = true, features = ["full", "test-util"] }
tower      = { workspace = true }
```

Run `cargo +nightly fmt -p {{service}}` after writing this (and every other)
file — the column alignment above is illustrative, not hand-matched.

### 2. Root `Cargo.toml` registration

Add one line to `[workspace.dependencies]`'s internal-crates block, keeping
it alphabetically sorted alongside `common-types` and `example`:

```toml
{{service}} = { path = "services/{{service}}" }
```

No `members` glob edit needed — `services/*` already covers it.

### 3. Directory tree

Git doesn't track empty directories, so nothing to create yet beyond the two
files above; the remaining prompts create `src/`, `migrations/`, and `tests/`
as they populate them. For reference, the finished tree:

```
services/{{service}}/
  Cargo.toml
  migrations/
    0001_{{feature}}.sql
  src/
    lib.rs
    main.rs
    config.rs
    module.rs
    feature.rs
    feature/
      {{feature}}.rs
      {{feature}}/
        domain.rs
        port.rs
        service.rs
        error.rs
        adapter.rs
        adapter/
          http.rs
          postgres.rs
  tests/
    common.rs
    {{feature}}_it.rs
```

## Verify

`cargo metadata` resolves targets from `[[bin]] path`, so it will fail with
"can't find `{{service}}` bin" until `src/main.rs` exists — that's expected
at this step. Confirm only that the TOML parses and the new dependency key
is well-formed:

```bash
taplo check services/{{service}}/Cargo.toml
```

Full resolution (`cargo metadata --no-deps`, `cargo check -p {{service}}`)
becomes meaningful once `02-lib-and-main.md` creates `src/main.rs`.
