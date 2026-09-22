---
name: rust-quality
description: "Use when auditing a Rust codebase for quality or maintenance issues — skipped tests (check 33, MANDATORY on every run), flaky tests, dead code, magic numbers, lock contention, undocumented invariants, dependency review, missing regression tests, scattered constants, undocumented settings, unnecessary clones, zero-copy opportunities, SQL value domains, or relay event coverage. Triggers on \"quality audit\", \"code quality\", \"maintenance sweep\", \"sql enum\", \"postgres enum\", \"check constraint\", \"value domains\", \"relay events\", \"relay nudge\", \"sse events\", \"live refresh\", \"stale ui\", or any of those symptoms in Rust code. Check 33 (no skipped tests) is mandatory and runs on every invocation regardless of what was asked."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Task
---

# Rust Quality & Maintenance

Run targeted code quality and maintenance checks on Rust codebases. Each check is a focused prompt that scans, analyzes, and fixes a specific category of maintenance debt.

> ## Check 33 is pre-commit enforced (CI enforcement exists but is off)
>
> `#[ignore]` and the silent-env-skip shape are mechanized
> (`.ast-grep/rules/no-skipped-tests.yml`, `no-silent-env-skip.yml` — see
> `docs/specs/2026-09-21-rust-quality-mechanization-design.md`) and wired into
> `lefthook.yml`'s `ast-grep` pre-commit command. A real violation fails the
> commit on its own, *provided* hooks are installed and nobody commits with
> `--no-verify`. The matching `.github/workflows/ci.yml` step exists but is
> currently disabled (`if: false`) — deliberately, to cut CI overhead for a
> single-developer repo — so there is no un-bypassable backstop the way there
> would be with both layers on: a bypassed hook, a bot commit, or a web-UI
> edit reaches `main` with nothing catching it until someone runs this skill
> or `cargo clippy` by hand. The old version of this banner ("run it every
> time, whatever else was asked") stays retired since lefthook covers the
> common case, but that residual gap is real — re-enabling the CI step
> closes it completely.
>
> `#[ignore]` is banned in this workspace. The only exception is a doctest that
> genuinely cannot execute, and even there prefer `no_run`. There is no
> "requires a database" exemption: `#[sqlx::test]` provisions a throwaway
> database per test.
>
> One shape is **not** mechanized and stays a manual check when this skill
> runs #33 directly: a test module gated behind a `#[cfg(feature = "...")]`
> that isn't on by default — confirming that needs the crate's default-feature
> set, which a structural rule can't see. See `33-no-skipped-tests.md`.

## Available Checks

| # | Check | Type | Prompt File | Mechanized |
|---|-------|------|-------------|------------|
| 1 | **Flaky Test Detection** | Code changes | `prompts/01-flaky-tests.md` | — |
| 2 | **Regression Test Generation** | Code additions | `prompts/02-regression-tests.md` | — |
| 3 | **Magic Number Audit** | Code changes | `prompts/03-magic-numbers.md` | — |
| 4 | **Const Organization** | Code changes | `prompts/04-const-organization.md` | — |
| 5 | **Invariant Analysis** | Documentation | `prompts/05-invariant-analysis.md` | — |
| 6 | **Settings Documentation** | Documentation | `prompts/06-settings-documentation.md` | — |
| 7 | **Crate Evaluation** | Analysis | `prompts/07-crate-evaluation.md` | — |
| 8 | **Lock Analysis** | Code changes | `prompts/08-lock-analysis.md` | — |
| 9 | **Dead Code Removal** | Code changes | `prompts/09-dead-code-removal.md` | — |
| 10 | **Clone Analysis** | Code changes | `prompts/10-clone-analysis.md` | — |
| 11 | **Zero-Copy Opportunities** | Code changes | `prompts/11-zero-copy-opportunities.md` | — |
| 12 | **Function Decomposition** | Code changes | `prompts/12-function-decomposition.md` | — |
| 13 | **Error Type Pattern** | Code changes | `prompts/13-error-pattern.md` | — |
| 14 | **Structured Logging Compliance** | Code changes | `prompts/14-structured-logging.md` | — |
| 15 | **Service Skeleton Consistency** | Analysis / Code changes | `prompts/15-skeleton-consistency.md` | — |
| 16 | **Internal Route Isolation** | Code changes / Audit | `prompts/16-route-isolation.md` | — |
| 17 | **Swallowed Errors** | Code changes | `prompts/17-swallowed-errors.md` | Partial — pattern 1 only, `dylint`, enforced in `lefthook.yml` (CI step exists, disabled) |
| 18 | **Missing Metrics** | Code changes / Audit | `prompts/18-missing-metrics.md` | — |
| 19 | **Garde Validation Coverage** | Code changes | `prompts/19-garde-validation.md` | — |
| 20 | **Cache Eviction Hygiene** | Code changes / Audit | `prompts/20-cache-eviction.md` | — |
| 21 | **Security Audit** | Code changes / Audit | `prompts/21-security.md` | — |
| 22 | **Path Conventions** | Code changes | `prompts/22-path-conventions.md` | — |
| 23 | **Error Messages** | Code changes | `prompts/23-error-messages.md` | — |
| 24 | **Secret Types** | Code changes / Audit | `prompts/24-secret-types.md` | — |
| 25 | **Service-to-Service Client Wiring** | Code changes / Audit | `prompts/25-s2s-client-wiring.md` | — |
| 26 | **OpenAPI Security Accuracy** | Code changes / Audit | `prompts/26-openapi-security-accuracy.md` | — |
| 27 | **Entity Field Naming (`created_by`)** | Code changes / Audit | `prompts/27-entity-field-naming.md` | — |
| 28 | **Keyset Cursor & Pagination Contract** | Code changes / Audit | `prompts/28-cursor-pagination.md` | — |
| 29 | **Authorization Guard (`require_*`) Consistency** | Code changes / Audit | `prompts/29-require-guard-consistency.md` | — |
| 30 | **SQL Value Domains (`TEXT` + `CHECK`)** | Code changes / Audit | `prompts/30-sql-value-domains.md` | — |
| 31 | **Relay Event Coverage** | Code changes / Audit | `prompts/31-relay-event-coverage.md` | — |
| 32 | **Cross-Crate Duplication** | Code changes / Audit | `prompts/32-cross-crate-duplication.md` | — |
| 33 | **No Skipped Tests** | Code changes / Audit | `prompts/33-no-skipped-tests.md` | Yes — `ast-grep`, enforced in `lefthook.yml` (CI step exists, disabled) |
| 34 | **Handler Hygiene** | Code changes / Audit | `prompts/34-handler-hygiene.md` | — |

"Mechanized" means a rule/lint exists, is proven against a fixture, and (for
#17 and #33) is wired into `lefthook.yml`'s pre-commit hook — a real
violation fails the commit without this skill being invoked at all. The
matching `.github/workflows/ci.yml` steps exist but are currently disabled
(`if: false`, to cut CI overhead for a single-developer repo) — see that
file's comment for the tradeoff and how to re-enable. See `.ast-grep/README.md`
and `.lints/rust-quality-dylint/README.md` for exact commands to run either by
hand.

## Check Categories

### Code Changes (modify existing code)
- **Flaky Tests (#1)** — Fix timing dependencies, shared state, non-deterministic tests
- **Magic Numbers (#3)** — Extract literals to named constants
- **Const Organization (#4)** — Centralize scattered constants into `const.rs` modules
- **Lock Analysis (#8)** — Replace locks with atomics/lock-free structures where beneficial
- **Dead Code Removal (#9)** — Remove `#[allow(dead_code)]` and unused code
- **Clone Analysis (#10)** — Identify unnecessary clones, suggest borrowing alternatives
- **Zero-Copy Opportunities (#11)** — Eliminate unnecessary data copying with slices, `Cow`, streaming
- **Function Decomposition (#12)** — Split oversized/multi-purpose functions; remove `allow(clippy::too_many_lines)` silencers; ensure rustdoc on every function
- **Error Type Pattern (#13)** — Enforce thiserror + ApiErrorMapping + garde Validation variant; no leakage in error formatters
- **Structured Logging Compliance (#14)** — Convert string-interpolated logs to structured fields; instrument boundary functions; ban secrets in fields
- **Service Skeleton Consistency (#15)** — Audit drift in `main.rs`, `config.rs`, and the metrics module across services
- **Internal Route Isolation (#16)** — Audit router composition; ensure internal routes never leak onto the public listener
- **Swallowed Errors (#17)** — Find `let _ =`, `.ok()`, empty `Err(_) =>`, `.map_err(|_| ...)` and similar patterns that hide failures; propagate or log explicitly
- **Missing Metrics (#18)** — Cross-reference each service's `metrics.rs` against its boundary functions; flag operations that aren't observable
- **Garde Validation Coverage (#19)** — Every `Deserialize` struct also derives `Validate`; every field has a rule; typed-ID fields reject `Uuid::nil()` via `validate_not_nil`
- **Cache Eviction Hygiene (#20)** — Outbound-client `TtlCache`s expose `evict_*` methods; mutation paths invalidate same-pod entries; shared-subscription cache invalidation is flagged as a bug (one-pod-only is worse than TTL drift)
- **Security Audit (#21)** — Flag secrets/PII in logs or error bodies, missing auth on mutating routes, timing-unsafe comparisons, unsafe CORS, weak crypto
- **Path Conventions (#22)** — Enforce workspace import/module-path conventions; no UFCS where a `use` import reads better
- **Error Messages (#23)** — Canonical wording, casing, and punctuation across `thiserror` `#[error("...")]` strings
- **Secret Types (#24)** — Lift secret-bearing `String`/`Vec<u8>` config and adapter fields into `secrecy::SecretString`/`SecretBox`
- **Service-to-Service Client Wiring (#25)** — Audit inter-service HTTP client construction, retries, and typed error mapping
- **OpenAPI Security Accuracy (#26)** — utoipa security annotations match the handler's actual auth layer
- **Entity Field Naming (`created_by`) (#27)** — Keep creator/editor/deleter field names identical across services and entities
- **Keyset Cursor & Pagination Contract (#28)** — Verify cursor encode/decode and pagination response shape match the shared `common-types` contract
- **Authorization Guard (`require_*`) Consistency (#29)** — Keep `require_*` guard signatures and shared precondition logic from drifting or duplicating across services
- **SQL Value Domains (#30)** — Closed value sets persist as `TEXT` + a named `CHECK`, never `CREATE TYPE ... AS ENUM`; the Rust enum (`#[sqlx(type_name = "text")]`) is the type-safe boundary; schema invariants live in `CHECK`, not only in Rust
- **Relay Event Coverage (#31)** — Every `*RelayEvent` variant has a producer (defined-but-unpublished = dead event / stale SPA); every state-mutating operation that changes SPA-rendered data publishes its `try_publish_relay` nudge with the correct `Audience`; multi-scope services emit on every scope arm
- **No Skipped Tests (#33)** — `#[ignore]` is banned; a skipped test reports as healthy infrastructure while asserting nothing. Convert DB tests to `#[sqlx::test]`, service tests to `wiremock`; delete only with a stated reason. Also flags test modules behind non-default features, and silent skips (an early `return` when an env var is unset). Doctests are the only exception, and prefer `no_run`
- **Cross-Crate Duplication (#32)** — Logic copied across sibling crates (transport plumbing, decode-and-map blocks, small helpers); consolidate behind one definition, parameterising what actually differs. Report the drifts between copies, not the line count
- **Handler Hygiene (#34)** — Handlers stay thin (extract, validate, delegate, respond); no direct DB/third-party calls from a handler; no raw DB row or leaked internal data in a response; correct HTTP status codes; `{Resource}ListParams` naming

### Code Additions (add new code)
- **Regression Tests (#2)** — Generate regression tests from branch fixes

### Documentation / Analysis (no code changes unless requested)
- **Invariant Analysis (#5)** — Document critical invariants and their maintenance
- **Settings Documentation (#6)** — Catalog all configurable settings
- **Crate Evaluation (#7)** — Evaluate whether a crate fits the project

## Core Principles

1. **Compile after every change.** Every code modification must leave the project in a compiling state. Run `cargo check` after each fix, `cargo build` after completing a check.
2. **Run tests after code changes.** If the check modifies code (not just documentation), run `just check` (or, for a faster inner loop, `just db-ensure && cargo nextest run --all-features` plus `cargo test --doc --all-features` for doctests — `cargo nextest` does not execute doctests). Bare `cargo test` skips `db-ensure` and fails every `#[sqlx::test]` with "DATABASE_URL must be set" unless a database is already exported.
3. **Explain the reasoning.** For every change, explain *why* — not just what changed. This turns the audit into documentation.
4. **Stay grounded in the workspace.** Only reference dependencies, tools, and patterns actually present in the project. Read `Cargo.toml` before making recommendations.
5. **Respect existing patterns.** Match the project's naming conventions, module structure, and code style. Don't impose external conventions.
6. **Flag uncertainty.** If a change might affect behavior (e.g., removing code that *might* be used via proc macros), flag it for human review rather than silently changing it.
7. **Never skip a test.** `#[ignore]` is banned; check 33 runs on every invocation of this skill and is not subject to the user's selection. Do not add one, and do not leave one you find — convert it (`#[sqlx::test]` for a database, `wiremock` for a service) or delete it with the reason in the commit. A skipped test is worse than a missing one: it appears in the run, prints a reassuring line, and contributes to a green summary while asserting nothing.
8. **Never silence lints without permission.** Do not add `#[allow(clippy::...)]`, `#[allow(dead_code)]`, `#[allow(unused)]`, `#[expect(...)]`, or any other suppression attribute to make a warning go away — fix the underlying issue. If a lint genuinely does not apply (rare — e.g. macro-generated code), stop and ask the user for explicit confirmation, including the warning text and why fixing it isn't appropriate. Silencing without permission is a policy violation, not a stylistic choice.

## Execution

### Step 1: Ask the User What to Run

**Check 33 runs regardless of the answer** — see the banner at the top of this
file. Run it first, report it, then run whatever the user selected below.

Present the remaining checks and ask the user to choose:

- **"Run all"** — Execute every check (#1–#32, #34; #33 always runs first regardless) in the recommended order below
- **"Run code quality"** — Checks #3, #4, #8, #9, #22 (magic numbers, const org, locks, dead code, path conventions)
- **"Run test quality"** — Checks #1, #2 (flaky tests, regression tests)
- **"Run performance"** — Checks #10, #11 (clone analysis, zero-copy)
- **"Run error handling"** — Checks #13, #17, #23, #34 (error pattern, swallowed errors, error messages, handler hygiene)
- **"Run security"** — Checks #21, #24, #26 (security audit, secret types, openapi security accuracy)
- **"Run API contract"** — Checks #19, #27, #28, #29 (garde validation, entity field naming, cursor pagination, guard consistency)
- **"Run service consistency"** — Checks #15, #16, #18, #25, #31, #32 (skeleton consistency, route isolation, missing metrics, s2s client wiring, relay event coverage, cross-crate duplication)
- **"Run documentation"** — Checks #5, #6 (invariants, settings)
- **"Evaluate crate: [name]"** — Check #7 only, for a specific crate
- **Pick specific checks** — Any combination by number

### Step 2: Scan the Project

Before running any check, gather context:

1. Read the workspace `Cargo.toml` to understand dependencies and workspace structure
2. Identify the crate(s) or service(s) to audit (ask if workspace has multiple)
3. Note the project's conventions: naming style, module structure, test organization

### Step 3: Execute Checks

For each selected check:

1. Read the prompt file from `prompts/` directory
2. Follow the prompt's instructions precisely
3. After code changes: run `cargo check` then `just check` (or `just db-ensure && cargo nextest run --all-features && cargo test --doc --all-features` for a faster inner loop)
4. After documentation: present the output for review
5. Summarize what was found and changed

### Step 4: Report

After all checks complete, provide a summary:

- Number of issues found per check
- Changes made (files modified, lines added/removed)
- Items flagged for human review
- Suggested follow-up actions

## Recommended Order (Full Sweep)

When running all checks, execute in this order for best results:

0. **No Skipped Tests (#33)** — always, and first, even though it's now
   CI/pre-commit enforced (see the banner above) — a full sweep should still
   confirm the suite it reasons about actually runs, rather than trusting
   that nobody committed with `--no-verify`. Run
   `ast-grep scan -c .ast-grep/sgconfig.yml services crates hosts` first and
   triage its output.
1. **Magic Number Audit (#3)** — Clearest immediate improvements
2. **Const Organization (#4)** — Naturally follows magic number extraction
3. **Dead Code Removal (#9)** — Reduces surface area before deeper analysis
4. **Path Conventions (#22)** — Same hygiene cluster as dead code removal;
   cleans imports before the checks below start reading them
5. **Flaky Test Detection (#1)** — Strengthen the safety net
6. **Regression Test Generation (#2)** — Add tests for recent fixes
7. **Lock Analysis (#8)** — Optimize synchronization
8. **Clone Analysis (#10)** — Remove unnecessary clones
9. **Zero-Copy Opportunities (#11)** — Eliminate unnecessary data copying
10. **Function Decomposition (#12)** — Split oversized/multi-purpose functions and ensure rustdoc coverage
11. **Error Type Pattern (#13)** — Bring error enums onto thiserror + ApiErrorMapping
12. **Error Messages (#23)** — Canonicalize wording on the error enums #13 just normalized
13. **Secret Types (#24)** — Wrap secret-bearing fields in `SecretString` before auditing where they leak
14. **Handler Hygiene (#34)** — Thin handlers, no leaked responses, correct status codes — the handler-boundary counterpart to #13
15. **Structured Logging Compliance (#14)** — Convert string-interpolated logs and instrument boundaries
16. **Security Audit (#21)** — Output-boundary discipline; benefits from #23/#24 having already normalized error and secret handling
17. **OpenAPI Security Accuracy (#26)** — Checks annotations against the auth layer #21 just audited
18. **Service Skeleton Consistency (#15)** — Audit and converge `main.rs`/`config.rs`/`metrics.rs` across services
19. **Internal Route Isolation (#16)** — Verify internal routes never reach the public listener
20. **Authorization Guard Consistency (#29)** — Guard-shape audit, pairs with the route/security checks just run
21. **Swallowed Errors (#17)** — Surface hidden failures before deeper analysis.
    Pattern 1 (`let _ = <Result>`) has a proven `dylint` lint, enforced in
    `lefthook.yml` (CI step exists, disabled) — run
    `cargo dylint --all --path .lints/rust-quality-dylint --pattern '*'` from
    the repo root first and triage its output; patterns 2-9 are still fully
    this prompt's judgment call.
22. **Missing Metrics (#18)** — Audit observability coverage of boundary operations
23. **Garde Validation Coverage (#19)** — Verify input validation completeness, especially typed-ID nil rejection where this workspace has one
24. **Entity Field Naming (#27)** — Same field-level audit class as #19, different concern
25. **Keyset Cursor & Pagination Contract (#28)** — Verify the pagination contract `common-types` defines is honored everywhere it's used
26. **Cache Eviction Hygiene (#20)** — Audit outbound-client caches for eviction surface and cross-replica honesty
27. **Service-to-Service Client Wiring (#25)** — Client-construction audit, pairs with the cache-eviction check on the same outbound clients
28. **SQL Value Domains (#30)** — Verify closed value sets use `TEXT` + `CHECK` and that schema invariants aren't Rust-only
29. **Relay Event Coverage (#31)** — Verify every mutation that should nudge the SPA does
30. **Cross-Crate Duplication (#32)** — Broadest audit; best run once the per-domain checks above have surfaced the patterns worth deduplicating
31. **Invariant Analysis (#5)** — Document critical assumptions
32. **Settings Documentation (#6)** — Catalog configuration surface
33. **Crate Evaluation (#7)** — Best after cleanup reveals actual needs

This order: clean up first, strengthen tests second, optimize allocations/copies
third, normalize errors/secrets/security fourth, audit service-level consistency
and contracts fifth, then analyze and document last.

## Parallelization

Independent checks can be run concurrently using the Task tool with subagents:

**Safe to parallelize:**
- Magic Numbers (#3) + Dead Code Removal (#9)
- Flaky Tests (#1) + Lock Analysis (#8)
- Clone Analysis (#10) + Zero-Copy (#11) (complementary but independent)
- Invariant Analysis (#5) + Settings Documentation (#6)
- Path Conventions (#22) + Dead Code Removal (#9) (disjoint concerns, both pure cleanup)
- Error Messages (#23) + Missing Metrics (#18) (independent per-file scans)
- Entity Field Naming (#27) + Garde Validation Coverage (#19) (both field-level audits, no overlapping edits)
- OpenAPI Security Accuracy (#26) + Cache Eviction Hygiene (#20) (independent audit-only checks)

**Must run sequentially:**
- Magic Numbers (#3) → Const Organization (#4) (depends on #3's output)
- Clone Analysis (#10) → Zero-Copy (#11) → Lock Analysis (#8) (overlapping changes)
- Any code-changing check → Regression Tests (#2) (needs stable code first)
- Error Type Pattern (#13) → Error Messages (#23) → Handler Hygiene (#34) (same error enums; sequential edits avoid conflicting diffs)
- Secret Types (#24) → Security Audit (#21) (wrap secrets in the type system before auditing where the now-wrapped values leak)
- Security Audit (#21) → OpenAPI Security Accuracy (#26) (accuracy check depends on #21's auth-layer findings)

## Integration with Other Skills

### rust-documenter
After running Invariant Analysis (#5) or Settings Documentation (#6), consider running the `rust-documenter` skill to ensure the new documentation follows rustdoc conventions.

