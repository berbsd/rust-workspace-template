---
name: rust-quality
description: "Use when auditing a Rust codebase for quality or maintenance issues — skipped tests (check 33, MANDATORY on every run), flaky tests, dead code, magic numbers, lock contention, undocumented invariants, dependency review, missing regression tests, scattered constants, undocumented settings, unnecessary clones, zero-copy opportunities, SQL value domains, or relay event coverage. Triggers on \"quality audit\", \"code quality\", \"maintenance sweep\", \"sql enum\", \"postgres enum\", \"check constraint\", \"value domains\", \"relay events\", \"relay nudge\", \"sse events\", \"live refresh\", \"stale ui\", or any of those symptoms in Rust code. Check 33 (no skipped tests) is mandatory and runs on every invocation regardless of what was asked."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Task
---

# Rust Quality & Maintenance

Run targeted code quality and maintenance checks on Rust codebases. Each check is a focused prompt that scans, analyzes, and fixes a specific category of maintenance debt.

> ## ⛔ Check 33 is MANDATORY and runs every time
>
> **`prompts/33-no-skipped-tests.md` is not part of the menu.** Run it on every
> invocation of this skill, whatever the user asked for, and report its findings
> even when they asked for something unrelated. It is the one check whose
> absence is self-concealing: a skipped test reports as healthy infrastructure
> while asserting nothing, so no other check — and no green suite — can reveal
> it.
>
> `#[ignore]` is banned in this workspace. The only exception is a doctest that
> genuinely cannot execute, and even there prefer `no_run`. There is no
> "requires a database" exemption: `#[sqlx::test]` provisions a throwaway
> database per test.
>
> This is non-negotiable. Do not ask whether to run it; do not skip it because
> the user scoped the request narrowly; do not defer it as a follow-up.

## Available Checks

| # | Check | Type | Prompt File |
|---|-------|------|-------------|
| 1 | **Flaky Test Detection** | Code changes | `prompts/01-flaky-tests.md` |
| 2 | **Regression Test Generation** | Code additions | `prompts/02-regression-tests.md` |
| 3 | **Magic Number Audit** | Code changes | `prompts/03-magic-numbers.md` |
| 4 | **Const Organization** | Code changes | `prompts/04-const-organization.md` |
| 5 | **Invariant Analysis** | Documentation | `prompts/05-invariant-analysis.md` |
| 6 | **Settings Documentation** | Documentation | `prompts/06-settings-documentation.md` |
| 7 | **Crate Evaluation** | Analysis | `prompts/07-crate-evaluation.md` |
| 8 | **Lock Analysis** | Code changes | `prompts/08-lock-analysis.md` |
| 9 | **Dead Code Removal** | Code changes | `prompts/09-dead-code-removal.md` |
| 10 | **Clone Analysis** | Code changes | `prompts/10-clone-analysis.md` |
| 11 | **Zero-Copy Opportunities** | Code changes | `prompts/11-zero-copy-opportunities.md` |
| 12 | **Function Decomposition** | Code changes | `prompts/12-function-decomposition.md` |
| 13 | **Error Type Pattern** | Code changes | `prompts/13-error-pattern.md` |
| 14 | **Structured Logging Compliance** | Code changes | `prompts/14-structured-logging.md` |
| 15 | **Service Skeleton Consistency** | Analysis / Code changes | `prompts/15-skeleton-consistency.md` |
| 16 | **Internal Route Isolation** | Code changes / Audit | `prompts/16-route-isolation.md` |
| 17 | **Swallowed Errors** | Code changes | `prompts/17-swallowed-errors.md` |
| 18 | **Missing Metrics** | Code changes / Audit | `prompts/18-missing-metrics.md` |
| 19 | **Garde Validation Coverage** | Code changes | `prompts/19-garde-validation.md` |
| 20 | **Cache Eviction Hygiene** | Code changes / Audit | `prompts/20-cache-eviction.md` |
| 21 | **Security Audit** | Code changes / Audit | `prompts/21-security.md` |
| 22 | **Path Conventions** | Code changes | `prompts/22-path-conventions.md` |
| 23 | **Error Messages** | Code changes | `prompts/23-error-messages.md` |
| 24 | **Secret Types** | Code changes / Audit | `prompts/24-secret-types.md` |
| 25 | **Service-to-Service Client Wiring** | Code changes / Audit | `prompts/25-s2s-client-wiring.md` |
| 26 | **OpenAPI Security Accuracy** | Code changes / Audit | `prompts/26-openapi-security-accuracy.md` |
| 27 | **Entity Field Naming (`created_by`)** | Code changes / Audit | `prompts/27-entity-field-naming.md` |
| 28 | **Keyset Cursor & Pagination Contract** | Code changes / Audit | `prompts/28-cursor-pagination.md` |
| 29 | **Authorization Guard (`require_*`) Consistency** | Code changes / Audit | `prompts/29-require-guard-consistency.md` |
| 30 | **SQL Value Domains (`TEXT` + `CHECK`)** | Code changes / Audit | `prompts/30-sql-value-domains.md` |
| 31 | **Relay Event Coverage** | Code changes / Audit | `prompts/31-relay-event-coverage.md` |
| 32 | **Cross-Crate Duplication** | Code changes / Audit | `prompts/32-cross-crate-duplication.md` |
| 33 | **No Skipped Tests** | Code changes / Audit | `prompts/33-no-skipped-tests.md` |
| 34 | **Handler Hygiene** | Code changes / Audit | `prompts/34-handler-hygiene.md` |

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
2. **Run tests after code changes.** If the check modifies code (not just documentation), run `cargo test` to verify nothing broke.
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

- **"Run all"** — Execute all 11 checks in the recommended order
- **"Run code quality"** — Checks #3, #4, #8, #9 (magic numbers, const org, locks, dead code)
- **"Run test quality"** — Checks #1, #2 (flaky tests, regression tests)
- **"Run performance"** — Checks #10, #11 (clone analysis, zero-copy)
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
3. After code changes: run `cargo check` and `cargo test`
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

0. **No Skipped Tests (#33)** — always, and first. Every check below reasons
   about a suite it assumes runs; this is the one that establishes whether it
   does. Finding dead code or missing metrics in a service whose tests never
   execute is measuring the wrong thing.
1. **Magic Number Audit (#3)** — Clearest immediate improvements
2. **Const Organization (#4)** — Naturally follows magic number extraction
3. **Dead Code Removal (#9)** — Reduces surface area before deeper analysis
4. **Flaky Test Detection (#1)** — Strengthen the safety net
5. **Regression Test Generation (#2)** — Add tests for recent fixes
6. **Lock Analysis (#8)** — Optimize synchronization
7. **Clone Analysis (#10)** — Remove unnecessary clones
8. **Zero-Copy Opportunities (#11)** — Eliminate unnecessary data copying
9. **Function Decomposition (#12)** — Split oversized/multi-purpose functions and ensure rustdoc coverage
10. **Error Type Pattern (#13)** — Bring error enums onto thiserror + ApiErrorMapping
11. **Handler Hygiene (#34)** — Thin handlers, no leaked responses, correct status codes — the handler-boundary counterpart to #13
12. **Structured Logging Compliance (#14)** — Convert string-interpolated logs and instrument boundaries
13. **Service Skeleton Consistency (#15)** — Audit and converge `main.rs`/`config.rs`/`metrics.rs` across services
14. **Internal Route Isolation (#16)** — Verify internal routes never reach the public listener
15. **Swallowed Errors (#17)** — Surface hidden failures before deeper analysis
16. **Missing Metrics (#18)** — Audit observability coverage of boundary operations
17. **Garde Validation Coverage (#19)** — Verify input validation completeness, especially typed-ID nil rejection where this workspace has one
18. **Cache Eviction Hygiene (#20)** — Audit outbound-client caches for eviction surface and cross-replica honesty
19. **SQL Value Domains (#30)** — Verify closed value sets use `TEXT` + `CHECK` and that schema invariants aren't Rust-only
20. **Invariant Analysis (#5)** — Document critical assumptions
21. **Settings Documentation (#6)** — Catalog configuration surface
22. **Crate Evaluation (#7)** — Best after cleanup reveals actual needs

This order: clean up first, strengthen tests second, optimize allocations/copies third, then analyze and document.

## Parallelization

Independent checks can be run concurrently using the Task tool with subagents:

**Safe to parallelize:**
- Magic Numbers (#3) + Dead Code Removal (#9)
- Flaky Tests (#1) + Lock Analysis (#8)
- Clone Analysis (#10) + Zero-Copy (#11) (complementary but independent)
- Invariant Analysis (#5) + Settings Documentation (#6)

**Must run sequentially:**
- Magic Numbers (#3) → Const Organization (#4) (depends on #3's output)
- Clone Analysis (#10) → Zero-Copy (#11) → Lock Analysis (#8) (overlapping changes)
- Any code-changing check → Regression Tests (#2) (needs stable code first)

## Integration with Other Skills

### rust-documenter
After running Invariant Analysis (#5) or Settings Documentation (#6), consider running the `rust-documenter` skill to ensure the new documentation follows rustdoc conventions.

