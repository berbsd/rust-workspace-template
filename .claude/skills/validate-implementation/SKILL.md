---
name: validate-implementation
description: "Use before declaring a Rust feature done in this workspace, before opening a PR, or when asked to validate, verify, or check whether an implementation is ready to ship. Triggers on \"validate implementation\", \"pre-commit check\", \"ready to commit\", \"final validation\", \"is this done\", \"ship check\", \"is this ready to ship\", \"check if done\", \"final review\"."
allowed-tools: Read, Glob, Grep, Bash, Skill
---

# Validate Implementation — Pre-Completion Gate

Run a structured validation pass over new code in this workspace before declaring work done. This skill is a thin orchestrator: it inspects the diff and delegates every actual audit to existing skills (`rust-quality`'s garde-validation and handler-hygiene checks, `sql-analyzer`, `rust-documenter`) rather than reimplementing them.

Use this skill when the user is about to commit, open a PR, or ask "is this done?". The goal is to catch issues *before* the user discovers them in review or production.

## Codebase Conventions

This skill targets this workspace, which uses:

- **Rust 2024 edition** with **Axum** handlers and **SQLx** queries
- **Input validation**: `garde` crate with `#[derive(garde::Validate)]` and `req.validate(&())?` at the top of every handler
- **Strongly-typed IDs**, if this workspace has grown a `define_id_type!`-style system: garde rules still apply to any *newly-defined* ID type. This template's own `common-types` doesn't have one yet — `services/example` uses plain `Uuid` fields.
- **Handler location**: `services/<name>/src/feature/<feature>/handler.rs` (public) and `handler_internal.rs` (service-to-service, if this workspace has an internal tier)
- **Migrations**: `services/<name>/migrations/*.sql` — append-only, never modify existing files
- **OpenAPI**, only if a service documents one: `utoipa` with `#[utoipa::path(...)]` on handlers, `ToSchema` on DTOs, registered in an `ApiDoc` struct. `services/example` doesn't enable this.
- **No raw DB rows in responses**: always a dedicated Response DTO
- **Task runner**: `just check` and `just test` gate all merges

## When To Run

Run this pass when:

- The user says "done", "ready", "validate", "final check", or similar completion language.
- New handlers, migrations, queries, DTOs, or OpenAPI schemas were added.
- Before `git commit` or `gh pr create`.

Skip (or truncate) when:

- Only comments/docs changed.
- Only test fixtures or non-Rust config changed.
- The user explicitly scoped the request ("just run tests", "only check SQL").

## Step 1 — Detect Scope

Before running any audits, figure out *what* changed. Use git, not guesses.

```bash
git status --short
git diff --stat $(git merge-base HEAD main)..HEAD
git diff --name-only $(git merge-base HEAD main)..HEAD
```

Classify the changed files:

| File pattern | Audit to trigger |
|---|---|
| `services/*/src/feature/*/handler*.rs` | `rust-quality` #34 (handler hygiene) + #19 (garde) + OpenAPI (if applicable) |
| `services/*/migrations/*.sql` | `sql-analyzer` + data-migration review |
| `services/*/src/**/*_query*.rs`, `repository.rs` | `sql-analyzer` (query efficiency) |
| Any new `pub struct` with `#[derive(...Deserialize...)]` | `rust-quality` #19 (garde) |
| Any `define_id_type!(...)` call, if this workspace has that macro | `rust-quality` #19 (garde) |
| `services/*/src/server.rs` (ApiDoc), if this service documents OpenAPI | OpenAPI duplicate-id check |

If nothing in the table matches, report that no validation is needed and stop.

## Step 2 — Delegate: Garde Validation

If Step 1 flagged a new `Deserialize` struct, a new typed ID, or a handler, invoke
`rust-quality`'s garde-validation check:

```
Skill(skill="rust-quality", args="run check 19 (garde validation) on current branch")
```

This single check already covers everything this step used to reimplement inline: every
new deserializable struct derives `Validate` with a `#[garde(...)]` rule on every field,
typed-ID nil rejection (if this workspace has typed IDs), and — critically — that every
handler actually *calls* `.validate()?` before delegating, not just that the type has
rules. Trust its output; do not re-derive these checks here.

## Step 3 — Delegate: SQL Audit

If Step 1 flagged migrations or queries, invoke the `sql-analyzer` skill:

```
Skill(skill="sql-analyzer", args="audit changes on current branch vs main")
```

After it returns, add these workspace-specific checks on top of its output:

- **Idempotency**: every `CREATE` / `ALTER` in a new migration uses `IF NOT EXISTS` / `IF EXISTS` — re-running the migration must not fail.
- **Atomicity**: each migration is a single logical transaction, or splits are justified in a comment (e.g., `CREATE INDEX CONCURRENTLY` which *cannot* run in a transaction — this is the explicit exception).
- **Never modify existing migration files** — confirm no SQL file under `migrations/` that existed on `main` was edited. If so, block and tell the user to add a new migration instead.
- **Data migration review**: for every schema change that adds a NOT NULL column, changes a type, or renames a column, explicitly state whether a backfill is required. If yes, confirm the backfill SQL is present (either in the same migration or a follow-up) and is safe under concurrent writes.

## Step 4 — OpenAPI Audit (only if the project documents an OpenAPI surface)

Skip this step entirely for a service that doesn't enable `utoipa`/`#[utoipa::path]`
annotations — this template's own `services/example` doesn't. Where a project does:

- **Every new handler has `#[utoipa::path(...)]`** with: method, path, `request_body` (if applicable), `responses` covering success + every error variant the handler can return, `tag`, `operation_id`.
- **Every new DTO derives `ToSchema`** (request bodies, response bodies, error responses, path/query params also need `IntoParams`).
- **Registered in `ApiDoc`**: grep the service's `server.rs` for the `#[openapi(paths(...), components(schemas(...)))]` derive and confirm each new path fn and schema type is listed. Handler paths must be fully qualified (`crate::feature::<f>::handler::<fn>`).
- **No duplicate `operation_id`** across the service. Extract all `operation_id = "..."` values from the spec (or from `#[utoipa::path]` annotations) and report any collisions:

```bash
rg -oN 'operation_id\s*=\s*"([^"]+)"' -r '$1' services/<svc>/src | sort | uniq -d
```

Any duplicate is a blocker — OpenAPI clients will be broken.

## Step 5 — Delegate: Handler Hygiene

If Step 1 flagged handlers, invoke `rust-quality`'s handler-hygiene check:

```
Skill(skill="rust-quality", args="run check 34 (handler hygiene) on current branch")
```

This covers thin-handler boundary violations (no direct DB/third-party calls from a
handler), response-DTO purity (no raw row, no leaked internal data), and correct HTTP
status codes. Trust its output — do not duplicate its checks. If this workspace has grown
an authorization layer, also run whatever check documents *that* pattern; it isn't part of
this template.

## Step 6 — Delegate: Doc Coverage

Documentation is a gated quality dimension, not a follow-up.

If Step 1 flagged **any** new or modified Rust item, invoke `rust-documenter`:

```
Skill(skill="rust-documenter", args="audit doc coverage on current branch")
```

Its contract for this workspace (AGENTS.md "Rustdoc on every function"): every `pub`,
`pub(crate)`, **and private** function carries a `///` summary; `# Errors` on every
`Result`-returning fn; `# Panics`/`# Safety` where they apply. Trivial accessors and
documented trait-impl methods are the only exemption. Spot-check the diff:

```bash
# functions in the diff whose preceding line is neither a doc comment nor an attribute
git diff main...HEAD --name-only -- '*.rs' | while read -r f; do
  [ -f "$f" ] || continue
  awk -v F="$f" 'prev !~ /\/\/\/|^\s*#\[/ && /^\s*(pub |pub\(crate\) )?(async )?fn [a-z_]+/ \
    { print F ":" NR ": " $0 } { prev = $0 }' "$f"
done
```

## Step 7 — Build & Test Gate

Run the workspace gates in parallel:

```bash
just check
just test
```

Both must pass cleanly. A warning is not a pass — zero-panic and async-safety lints are *denied* in this workspace, so clippy warnings on new code are blockers.

## Step 8 — Summary Report

Produce a single final report using this exact structure:

```
## Validation Summary

**Scope:** <N handlers, M migrations, K new DTOs changed>

### Garde & Input Validation (via rust-quality #19)
- [PASS|FAIL] All new structs derive garde::Validate
- [PASS|FAIL|SKIPPED] All new typed IDs validated at DTO boundaries (skip if this workspace has no typed-ID system)
- [PASS|FAIL] All handlers call .validate() before service calls
- <rust-quality #19 findings inline>

### SQL (via sql-analyzer)
- [PASS|FAIL|SKIPPED] Migrations idempotent & atomic
- [PASS|FAIL|SKIPPED] No modified existing migrations
- [PASS|FAIL|SKIPPED] Data backfill reviewed
- <sql-analyzer findings inline>

### OpenAPI (only if the project documents one — see Step 4)
- [PASS|FAIL|SKIPPED] All handlers annotated
- [PASS|FAIL|SKIPPED] All DTOs derive ToSchema
- [PASS|FAIL|SKIPPED] All paths/schemas registered in ApiDoc
- [PASS|FAIL|SKIPPED] No duplicate operation_id

### Handler Hygiene (via rust-quality #34)
- <rust-quality #34 findings inline>

### Docs (via rust-documenter)
- [PASS|FAIL|SKIPPED] Every new fn (pub, pub(crate), private) has a `///` summary
- [PASS|FAIL|SKIPPED] `# Errors` present on every Result-returning fn
- [PASS|FAIL|SKIPPED] No dated changelog comments introduced (Reintroduction Test)
- <documenter findings inline>

### Build Gate
- [PASS|FAIL] just check
- [PASS|FAIL] just test

### Verdict
[READY TO SHIP | BLOCKED] — <one-sentence reason>
```

If the verdict is `BLOCKED`, list the specific file:line locations that need fixing. Do not say "done" or "ready" unless every check is PASS or explicitly-justified SKIPPED.

## Out of Scope

This skill does **not**:

- Create commits — always defer to the user for that.
- Push, open PRs, or modify remote state.
- Fix issues it finds — it reports, the user (or a follow-up request) fixes.
- Audit code that existed on `main` before this branch — only new/changed code.

## Notes on Sub-Skill Invocation

- Each sub-skill loads its own instructions into context. This skill invokes `rust-quality` twice (check #19, then check #34) alongside `sql-analyzer` and `rust-documenter` — that's expected, since #19 and #34 are distinct checks within the same skill, not a repeat. Avoid re-running the *same check number* twice in a single pass.
- Sub-skills cannot re-invoke this skill (no re-entry). That's fine — this is a one-way tree.
- If a sub-skill fails to run (e.g., permission denied), report it as `SKIPPED — <reason>` and continue. Do not halt the whole validation on a single sub-skill failure.
