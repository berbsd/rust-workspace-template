# Appendix: Adding a Feature to an Existing Generated Service

Re-runs prompts 5–11 for a new `{{feature}}`/`{{Feature}}` against a service
this skill already scaffolded — `01-crate-scaffold.md` through
`04-module.md` do not re-run. A service's crate manifest, `lib.rs` module
tree (beyond the one new line below), `config.rs`, and `module.rs` are
established once, at service creation.

## What's different from a first feature

### 1. `services/{{service}}/src/feature.rs` — append, don't replace

```rust
pub(crate) mod {{feature}};
```

Add this line alongside the existing `pub(crate) mod <other_feature>;` — the
file's doc comment and every other feature's line stay untouched.

### 2. `services/{{service}}/src/lib.rs` — one more `.merge(...)`

`router(pool: PgPool) -> Router` gets a second (or third, …) merge call:

```rust
pub fn router(pool: PgPool) -> Router {
  Router::new()
    .route("/health", get(health))
    .merge(feature::widget::router(pool.clone()))       // existing feature(s)
    .merge(feature::{{feature}}::router(pool))            // new
}
```

Every merge call after the first needs `pool.clone()` — only the last one
can consume `pool` by value. Order doesn't matter functionally, but keep it
alphabetical by feature name so the list stays scannable as it grows.

### 3. A new migration file, numbered after the last one

`services/{{service}}/migrations/000N_{{feature}}.sql` — `N` is one past
the highest existing migration number. Follow `07-feature-adapter-postgres.md`'s
migration template exactly; the immutability rule (never edit a migration
once applied) already applies to every earlier migration in this directory,
which is exactly why this one is new rather than an edit to
`0001_<existing>.sql`.

### 4. `Cargo.toml` — usually unchanged

A new feature only needs a new dependency if it does something the first
feature didn't (e.g. calls an external HTTP API). If so, add it to
`[workspace.dependencies]` first (root `Cargo.toml`), then reference it here
with `{ workspace = true }` — never a service-local version pin (see
AGENTS.md's "Workspace Rules").

## Steps to run, in order

0. `00-architecture-decisions.md` — every new feature gets its own answers;
   "the existing features are all Postgres CRUD" is not a reason to skip
   asking whether *this* one needs persistence, the full operation set, or
   caching.
1. `05-feature-domain-port.md` → new `feature/{{feature}}/domain.rs` + `port.rs`
2. `06-feature-service.md` → new `feature/{{feature}}/service.rs`
3. `07-feature-adapter-postgres.md` → new `feature/{{feature}}/adapter/postgres.rs`
   + the next-numbered migration
4. `08-feature-adapter-http.md` → new `feature/{{feature}}/adapter/http.rs`
5. `09-feature-error.md` → new `feature/{{feature}}/error.rs`
6. `10-feature-wiring.md`'s files 1–2 only (`adapter.rs`, `feature/{{feature}}.rs`)
   — its file 3 (`feature.rs`) and file 4 (`lib.rs`) are the two edits shown
   above instead of the fresh-file versions those sections describe
7. `11-tests.md` → new `tests/{{feature}}_it.rs` (`tests/common.rs` is
   already there — do not overwrite it)
8. `12-verification.md` — same as any other change; run it in full,
   including `just check`, which now also re-runs every earlier feature's
   tests to confirm nothing regressed

## Verify

```bash
grep -c "pub(crate) mod" services/{{service}}/src/feature.rs
```

Should equal the number of features this service now has — a missing line
here is a feature that compiles (its own module tree is self-contained) but
is unreachable from any router, which `cargo check` will not catch on its
own (an unused-but-declared `mod` only warns if nothing inside it is ever
referenced, and `router()`'s own internals do reference it — the risk is
the opposite: forgetting the `mod` line entirely, which is a compile error
the moment `feature::{{feature}}` is referenced anywhere, including in this
same prompt's `lib.rs` edit). Confirm by running `12-verification.md` in
full rather than trusting the grep alone.
