---
name: create-rust-service
description: "Use when scaffolding a brand-new Rust service into this workspace's services/ directory, or adding a first/next feature slice to one. Generates a feature-first, hexagonal service — lib.rs, main.rs, config.rs, module.rs, and per-feature domain/port/service/adapter/error modules — consistent across every generated service. Triggers on \"create a service\", \"new service\", \"scaffold a service\", \"add a feature\", \"new feature slice\", or a request to add a resource/domain to services/."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Task
---

# Create Rust Service

Scaffold a new service — or a new feature slice inside an existing generated
service — using this workspace's feature-first, hexagonal shape. Each prompt
in `prompts/` is a focused generation step with a literal, parameterized Rust
template; run them in order and every generated service comes out looking
like every other one, the same way `rust-quality` audits every service
against the same rules.

## Two sanctioned shapes, not one

`services/example` stays the minimal, trivial reference it already is —
`main.rs` reading `DATABASE_URL`/`PORT` directly, `lib.rs` exposing a bare
`router(pool)`, one flat `feature/widget/{model,handler,repository,error}.rs`
slice. It demonstrates the wire-level conventions (handler → repository,
`ApiErrorMapping`, garde validation, keyset pagination) with the least
possible ceremony, and is deliberately **not** what this skill generates.

Every service this skill generates — and every feature slice added to one —
follows the richer shape below instead. A workspace can have both: read
`services/example` to learn the wire-level conventions; use this skill to
build an actual service. Do not backport this skill's shape onto
`services/example` itself unless a human asks for that explicitly — it was a
deliberate choice, not an oversight (see AGENTS.md's "Service anatomy"
section).

## Target shape

The shape below is the **default for a Postgres-backed feature with full
CRUD**. It is not universal — `prompts/00-architecture-decisions.md` decides,
per feature, whether persistence exists at all, which operations are real,
and whether caching is warranted, *before* any of prompts 1–11 run. Treat
every file below as conditional on that decision, not as a checklist every
feature must fill in regardless of whether it needs to.

```
services/<service>/
  Cargo.toml
  migrations/
    0001_<feature>.sql
  src/
    lib.rs                  # module tree, crate doc, router() composition
    main.rs                 # binary entrypoint: load config, module::init, serve
    config.rs                # Config struct (figment + garde), Config::load()
    module.rs                 # init(): Postgres pool + migrate + router, given a Config
    feature.rs                 # `pub(crate) mod <feature>;` per slice
    feature/
      <feature>.rs               # this feature's `router(pool)` — mirrors services/example
      <feature>/
        domain.rs                  # <Feature>Row — the persistence struct, no axum/sqlx wire types
        port.rs                    # <Feature>Repository trait (the storage port) — only if persisted
        service.rs                 # <Feature>Service: orchestration + domain rules
        error.rs                   # <Feature>Error (thiserror + ApiErrorMapping)
        adapter.rs                 # `pub(crate) mod http; pub(crate) mod postgres;`
        adapter/
          http.rs                    # <Feature>Response (never <Feature>Row) + handlers
          postgres.rs                 # Pg<Feature>Repository — port.rs impl — only if persisted
  tests/
    common.rs                 # shared #[sqlx::test] harness, mirrors services/example
    <feature>_it.rs            # end-to-end HTTP test against a real throwaway Postgres
```

Data flow, same as the pending `docs/specs/2026-09-18-hexagonal-feature-architecture-design.md`
this shape is drawn from: `adapter/http.rs` extracts + validates (garde) →
calls `<Feature>Service` with plain values (no axum types) →
`<Feature>Service` applies domain rules → calls `&dyn <Feature>Repository`
(the port, when one exists) → `adapter/postgres.rs` runs the query. Errors
are `<Feature>Error` the whole way; `adapter/http.rs` is the only place that
turns one into an HTTP response, via `impl_api_error_response!`.

`domain.rs`'s `<Feature>Row` and `adapter/http.rs`'s `<Feature>Response` are
two distinct types, always — `<Feature>Row` is never `#[derive(Serialize)]`,
never returned from a handler, never crosses into `adapter/http.rs` except
as the input to an explicit `From<<Feature>Row> for <Feature>Response`. See
"Core Principles" below.

No inbound port trait — there is exactly one driving adapter (HTTP) per
feature, and a trait with one caller and one implementor is ceremony without
benefit. Add one only if a second driving adapter (a CLI, a subscriber)
genuinely appears for that feature.

## Available Prompts

| # | Prompt | Produces |
|---|--------|----------|
| 0 | `prompts/00-architecture-decisions.md` | Nothing generated — decides persistence, real operation set, and caching before any file is written |
| 1 | `prompts/01-crate-scaffold.md` | `Cargo.toml`, directory tree, root workspace registration |
| 2 | `prompts/02-lib-and-main.md` | `src/lib.rs`, `src/main.rs` |
| 3 | `prompts/03-config.md` | `src/config.rs` |
| 4 | `prompts/04-module.md` | `src/module.rs` |
| 5 | `prompts/05-feature-domain-port.md` | `feature/<f>/domain.rs`, `feature/<f>/port.rs` |
| 6 | `prompts/06-feature-service.md` | `feature/<f>/service.rs` |
| 7 | `prompts/07-feature-adapter-postgres.md` | `feature/<f>/adapter/postgres.rs`, `migrations/0001_<f>.sql` |
| 8 | `prompts/08-feature-adapter-http.md` | `feature/<f>/adapter/http.rs` |
| 9 | `prompts/09-feature-error.md` | `feature/<f>/error.rs` |
| 10 | `prompts/10-feature-wiring.md` | `feature/<f>/adapter.rs`, `feature/<f>.rs`, `feature.rs`, `lib.rs` router wiring |
| 11 | `prompts/11-tests.md` | `tests/common.rs`, `tests/<f>_it.rs`, `service.rs` unit tests |
| 12 | `prompts/12-verification.md` | Nothing new — proves the previous 11 steps actually work |
| — | `prompts/13-add-feature.md` | Appendix: re-runs steps 5–11 to add feature N+1 to an existing generated service |

Prompts 5–9 repeat once per feature slice — a new service's first feature and
every feature added afterward via `prompts/13-add-feature.md`.

## Placeholders

Every prompt uses the same four placeholders; resolve them once at the start
and hold them for every step:

- `{{service}}` — kebab-case crate/directory name (`note`, `time-entry`). Also
  the binary name and the workspace-dependency key.
- `{{service_snake}}` — `{{service}}` with `-` → `_` (`time_entry`); only
  differs from `{{service}}` for a multi-word name. Used wherever a Rust
  identifier is needed (the crate path in `use`, the workspace-dep key stays
  kebab-case since Cargo allows hyphens there).
- `{{feature}}` — snake_case feature module name, singular (`note`,
  `time_entry`). The first feature is usually the service's namesake; ask if
  it should be named differently.
- `{{Feature}}` — PascalCase of `{{feature}}` (`Note`, `TimeEntry`) — the
  repository trait prefix, service struct prefix, and error enum prefix.
  The persistence struct itself is `{{Feature}}Row` (`NoteRow`), never bare
  `{{Feature}}` — see `05-feature-domain-port.md` and "Core Principles".
- `{{feature_plural}}` — `{{feature}}` pluralized (`notes`, `time_entries`) —
  table name and route path segment. Flag an irregular plural for the user
  instead of guessing.

## Execution

### Step 1: Architecture decisions, then inputs

**Run `prompts/00-architecture-decisions.md` first — every time, before
asking anything else.** It is not optional and not part of the menu, the
same way `rust-quality`'s check #33 always runs first: every later prompt's
shape depends on its answers (does this feature persist to Postgres at all?
which operations are real? does it need caching?), and generating the full
CRUD-plus-Postgres shape for a feature that doesn't need it is exactly the
"no unrequested functionality" violation AGENTS.md bans everywhere else in
this workspace.

Then gather (or infer from the user's request) before writing anything:

1. `{{service}}` and a one-line Cargo.toml `description`.
2. `{{feature}}` / `{{Feature}}` for the first feature slice.
3. The entity's fields beyond `id`/`created_at` (name, types, which are
   nullable) — every prompt below assumes at least one more field exists;
   a zero-extra-field entity is `services/example`'s `widget` and doesn't
   need this skill.
4. Any config beyond `DATABASE_URL`/`PORT` the service already knows it
   needs (an external API key, a feature flag). If none, `prompts/03-config.md`
   still produces a `Config` with just those two — every future field gets a
   home without restructuring.
5. The three answers from `00-architecture-decisions.md`: persistence
   (Postgres / none), the real operation set (not default-CRUD unless CRUD
   is actually what's needed), and caching (none, by default).

### Step 2: Run the prompts in order

For prompts 1–11: read the prompt file, substitute the placeholders, write
the file(s) it produces. Each prompt states its own file-by-file content —
follow it precisely rather than improvising a different shape.

### Step 3: Verification (always runs)

**`prompts/12-verification.md` is not optional**, the same way
`rust-quality`'s check #33 always runs regardless of what else was picked.
Run it after every prompt 1–11 pass, whether this is a new service or a new
feature on an existing one. A service that doesn't compile or doesn't answer
its own smoke-test request isn't scaffolded, it's a pile of files that looks
like Rust.

### Step 4: Report

- Files created, by prompt number.
- The exact `just run {{service}}` / `curl` smoke test that was run and its
  output.
- Anything the user still needs to decide (auth layer, additional config,
  a second feature) — this skill does not invent scope beyond what Step 1
  gathered.

## Adding a feature to an existing generated service

Use `prompts/13-add-feature.md` instead of starting over: it re-runs prompts
5–11 for the new `{{feature}}`/`{{Feature}}` against the existing crate,
touching only `feature.rs` and `lib.rs`'s router composition outside the new
feature's own directory. `prompts/01-crate-scaffold.md` through
`prompts/04-module.md` do not re-run — a service's crate scaffold, `lib.rs`
module tree (beyond the one new `mod` line), `config.rs`, and `module.rs` are
established once.

## Out of scope by default

These mirror real pieces of a mature service (see `stillgood/platform`'s
`profile` service) but need infrastructure this workspace doesn't have yet.
Do not add them speculatively — add the prerequisite first, as its own
reviewed change, then extend this skill:

- **`metrics.rs`** — needs a metrics crate (`metrics`, `prometheus`, …) as a
  workspace dependency; none exists today (`rust-quality` check #18 tracks
  this gap).
- **`access.rs`** (auth guards) / an auth extractor on handlers — needs an
  auth layer; every service in this workspace is public today
  (`rust-quality` checks #16, #21, #26, #29 all note the same gap).
- **`event.rs`** (structured event-name constants), an outbox, or a
  publisher — needs a pub/sub client; this workspace has none.

If the workspace grows one of these, add a numbered prompt for it here rather
than improvising it inline the first time a generated service needs it.

## Core Principles

1. **Don't over-engineer.** Generate only what `00-architecture-decisions.md`'s
   answers call for — a feature with no persistence gets no `port.rs`/
   `adapter/postgres.rs`/migration; a read-only feature gets no `create`/
   `delete`; a feature with no caching need gets no cache layer. More
   generated code than the feature actually requires is the same
   "unrequested functionality" AGENTS.md bans for hand-written code — this
   skill's templates are a ceiling to trim from, not a floor to always fill.
2. **Never expose the persistence row directly.** `domain.rs`'s
   `{{Feature}}Row` (the `sqlx::FromRow` struct) never derives `Serialize`,
   never appears as a handler's return type, and never crosses into
   `adapter/http.rs` except as the input to an explicit
   `From<{{Feature}}Row> for {{Feature}}Response` — even on the day every
   field happens to match. A column added to the table later must not
   silently become an API field because nothing marked the boundary.
3. **Every generated file compiles.** Run `cargo check -p {{service}}` after
   each prompt, not just at the end — a broken step 6 should never be
   discovered at step 11.
4. **No placeholder leaks.** Grep the finished service for `{{` before
   calling it done; an unresolved placeholder is a broken generation, not a
   TODO.
5. **Never invent a second shape.** If a generated service's requirements
   don't fit a template file (e.g. the feature has two storage backends),
   stop and ask rather than silently deviating — a one-off shape defeats the
   entire point of this skill existing.
6. **Reuse `common-types`.** Pagination (`PaginatedResponse`, `KeysetCursor`),
   the error envelope (`ApiErrorBody`, `ApiErrorMapping`,
   `impl_api_error_response!`) come from `common-types`, never
   reimplemented per service.
7. **`just check` is the final gate**, same as any other change in this
   workspace — `prompts/12-verification.md` runs it.

## Integration with Other Skills

### rust-quality
Run the relevant subset (at minimum #13 error pattern, #19 garde validation,
#33 no skipped tests, #34 handler hygiene) against a freshly generated
service before considering it done — this skill generates the shape those
checks expect, but generation is not a substitute for the audit.

### rust-documenter
Every generated `pub`/`pub(crate)`/private function needs a `///` summary
per AGENTS.md's rule; the templates below include one for every item, but
if a prompt is adapted beyond its template, re-run `rust-documenter` over
the diff.
