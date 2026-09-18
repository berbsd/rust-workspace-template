# {{PROJECT_NAME}} — Claude Code Guide

Rust 2024 microservices workspace. This file is the top-level contract for
working in this repo — read it before making changes, and keep it accurate as the shape
of the workspace evolves.

## Architectural north star: consistency across services

**The defining requirement of this workspace is that every service looks and behaves the
same.** Same feature-first layout, same handler contract, same error envelope, same
naming, same route tiers, same config and telemetry shape. A developer who knows one
service should be able to read any other without relearning anything. When you add or
change code, the question is not only "is this correct?" but "does this match how every
other service does it?" Divergence is a defect even when it compiles.

`services/example` is the living exemplar: one feature slice (`feature/widget`), the
plain `adapter`/`domain`/`feature`/`port` shape. Follow it when scaffolding a new
service; don't invent a second shape.

## Specs & plans

Specs go in `docs/specs/` as `YYYY-MM-DD-<topic>-design.md`, plans in `docs/plans/` as
`YYYY-MM-DD-<topic>-plan.md`.

## Working Rules

These apply to every task, not just coding tasks.

- **Propose before doing.** Present options with recommendations and tradeoffs first. Wait for the user to pick a direction before editing code. The only exception is a trivial, unambiguous fix the user explicitly asked for.
- **Fix + risk.** When addressing an issue, state the proposed fix *and* its risk (what could break, what assumptions it makes, what it doesn't cover) before applying it.
- **Reuse before adding.** Before writing new code, search for existing patterns, helpers, or types that already solve the problem. Prefer extending or reusing them. Call out duplication you'd be introducing and ask whether to extract instead.
- **Smallest viable change.** Prefer the minimal diff that solves the problem. Touch only what the task requires — don't refactor, rename, reformat, or "improve" adjacent code unasked. Don't add abstraction (traits, generics, config knobs, indirection layers) until there are concrete call sites that need it; solve the case in front of you, not a hypothetical future one. Every line added is a line someone later has to read and maintain.
- **No unrequested functionality.** Deliver exactly what was asked. Anything beyond the request — an extra endpoint, field, config option, helper, or "obvious next step" — needs an explicit OK before it's written. Bloat enters the codebase as helpfulness.
- **Match the platform.** New code conforms to the established pattern for its area. If you must deviate, say so and why. Don't invent a second way to do something the workspace already does one way.
- **No code smells.** Don't ship dead code, commented-out blocks, TODOs without issue refs, magic numbers, copy-pasted blocks, deeply nested branches, swallowed errors, or `#[allow(...)]` without justification. If you notice one nearby, flag it — don't silently inherit it.
- **Single-purpose, small functions.** Each function does one thing; aim under ~50 lines of body. Orchestrators (handlers, top-level service methods, `main`) compose named steps, they don't implement them. **Never silence size/complexity lints** (`too_many_lines`, `cognitive_complexity`) — they're signal.
- **Pure where practical.** Keep domain logic (mapping, validation, computation) in pure functions that take inputs and return outputs; push IO and mutation to the edges (handlers, repositories, clients). Prefer returning new values over mutating arguments. This functional-core / imperative-shell split keeps logic unit-testable without mocks — it does not mean avoiding IO, only isolating it.
- **Rustdoc on every function.** `pub`, `pub(crate)`, *and* private — `///` summary minimum; add `# Errors`/`# Panics`/`# Safety` where they apply. Trivial accessors and documented trait-impl methods are the only exemption.
- **Sibling files over `mod.rs`.** Splitting a module keeps the parent as `foo.rs` beside a new `foo/` directory — never `foo/mod.rs`.
- **Every test runs, every time. `#[ignore]` is banned.** A test that does not run is not a test, it is a comment that costs CI time to skip. There is no "requires a database" exemption — `#[sqlx::test]` provisions a throwaway database per test and `just db-ensure` starts one, so a DB-backed test runs like any other. The only acceptable skip is a doctest that genuinely cannot execute, and even then prefer `no_run`, which still type-checks.
- **Every defect gets a regression test.** When a defect is found, the fix lands with a test that fails on the old behavior and passes on the new.
- **Tests validate intended behavior, not the implementation.** Derive a test's expected values from the contract (spec, requirement, API doc), never by running the code and pasting its output into the assertion.
- **Quality gate after changes.** After non-trivial Rust changes, run the `rust-quality` skill (or the relevant subset). Never silence a lint without explicit user permission.

## Layout

- `services/<name>/` — a service: `main.rs` (binary entrypoint), `lib.rs` (exposes a `router(pool)` function), `feature/<name>/` (one vertical slice per resource: `model.rs`, `handler.rs`, `repository.rs`, `error.rs`), `migrations/`, `tests/`. Auto-discovered via `services/*`. `services/example` is the living exemplar — copy its shape and swap `widget` for your first real feature.
- `hosts/<name>/` — binaries that compose multiple services' `router(pool)` functions into one deployable process (`.nest("/prefix", service::router(pool))` per member), purely for saving always-warm compute cost. Every service also stands alone (`cargo run -p <service>`) — a host is never required. `example-host` mounts `services/example` to demonstrate the pattern; see its `main.rs` doc comment for what mounting a second service takes.
- `crates/<name>/` — workspace-wide libraries. Today just `common-types` (the shared API error envelope and keyset pagination helper) — there is no shared service framework in this template; each service depends on `axum`/`sqlx`/`tokio` directly rather than through a bootstrap layer. Add a crate here when logic is genuinely shared across 2+ services, not preemptively.
- `bin/` — project scripts (on PATH via direnv): `bootstrap` installs the dev toolchain, `deploy`/`deploy_all` are placeholders for your own infrastructure.
- **Adding a service**: scaffold `services/<name>/` following `services/example`'s shape, add its path to root `[workspace.dependencies]`.
- `jobs/<name>/` and `workers/<name>/` — neither directory exists yet, and the root `Cargo.toml` has no glob for either (Cargo fails on a glob whose directory is missing), so adding the first one means adding its glob too. `jobs/` is for a **one-off** run — an import, a backfill, a migration: finite input, run and done. `workers/` is for **ongoing** work — queue drains, sweeps, generation: triggered on demand and/or on a schedule, running forever in principle.
- Root `Cargo.toml` — workspace deps + lints; `justfile` — task automation (`just check`, `just test`, `just fmt`).

## Service anatomy

Every service follows the same shape (see `services/example`):

- **`main.rs`** — reads config from env vars directly (`DATABASE_URL`, `PORT`), connects a `sqlx::PgPool`, runs `sqlx::migrate!`, calls the crate's `router(pool)`, and serves it with `axum::serve`. No shared bootstrap crate — this is the whole of what a binary does.
- **`lib.rs`** — exposes `pub fn router(pool: PgPool) -> Router`, the one function both the standalone binary and a composing host call.
- **`feature/<name>/`** — one vertical slice per resource: `model.rs` (wire + domain types), `handler.rs` (extract → validate → delegate to the repository → respond), `repository.rs` (a trait plus a Postgres-backed impl — the seam for testing, even though this template's own tests prefer a real throwaway database over mocking it), `error.rs` (the feature's error enum, mapped onto `common_types::ApiErrorBody` via `impl_api_error_response!`).
- Keep this shape when adding a feature. Reach for a shared crate under `crates/` only once two services genuinely need the same code — don't build one in advance of a second consumer.

## Workspace Rules

- **Names**: kebab-case dirs/packages. Service crates are **singular** (`cargo run -p example`). A DB **schema** is the singular service name — no exceptions. **Tables** are plural and carry no entity prefix. REST resources are **plural** (`/widgets/{id}`). Rust types: `{Service}Error`, `{Service}Metrics`, `{Service}Repository`, `{Domain}Service`, `Config`/`{Domain}Config`. Metric names: `{{METRIC_NAMESPACE}}.<service>.<subsystem>.operations_total` / `_duration_seconds` (verb in the `operation` label, never baked into the metric name).
- **Deps**: declared in root `[workspace.dependencies]` — external *and* internal crates — used as `{ workspace = true, features = [...] }`. The root manifest is the single record of where an internal crate lives; moving one edits its path line only. **Lints**: `[lints] workspace = true` in every crate.
- **Commits**: conventional commits — `feat|fix|refactor|docs|chore|test|perf|ci|build|style|revert(scope): description`.
- **Before commit**: `just check` must pass. Workspace lints (zero-panic, async safety, unused `Result`) are denied — warnings on new code are blockers.

## Migrations Are Immutable

**Never edit a migration file that has been applied anywhere — not the SQL, not a
comment, not whitespace.** `sqlx` stores a checksum of each file's *entire contents* in
`_sqlx_migrations` and compares it at startup. A one-word comment change produces
`VersionMismatch(<version>)`, and the binary exits 1 on the next deploy that already ran
it.

Changing what a migration did means **adding a new migration** that alters the result.
Renaming a thing in prose means editing the Rust doc comment that describes it, never the
`.sql` that created it.

## Zero-Panic Policy

`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` are **denied** in all crates by
`[workspace.lints]` in the root `Cargo.toml` — not a guideline, a build failure. Use `?`,
pattern matching, or proper error propagation. Holding `Mutex`/`RefCell` across `.await`
is denied too.

## Skills

Project skills live in `.claude/skills/`. Reach for `rust-documenter` when writing docs,
the `rust-quality` prompts as the per-area contract (including handler hygiene, check
#34), and `sql-analyzer` when reviewing queries/migrations. **Run
`validate-implementation` before declaring work done.**

Some `rust-quality` checks describe patterns this template doesn't have yet (a
transactional outbox, a JWT revocation cache, service-to-service typed clients) — those
are conditional, skip them if the workspace has grown none of that.

| Area | Source of truth |
|---|---|
| Service anatomy, file placement, the `main`/`lib`/`feature` shape | This file's "Service anatomy" section + `services/example` |
| Domain errors & API envelope, garde validation, error-message wording | `rust-quality` |
| Rustdoc content — `///`/`//!` on every item, `# Errors`/`# Panics`/`# Safety` | `rust-documenter` skill |
| OpenAPI / utoipa annotations, Scalar docs — optional in this template, `common-types` supports it behind the `openapi` feature but `services/example` doesn't enable it | `rust-documenter` skill's `references/openapi-annotations.md` |
| Security (secret logging, timing-safe compare, leakage, path traversal, CORS, crypto) | `rust-quality` skill |
