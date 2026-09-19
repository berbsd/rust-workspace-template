---
name: sql-analyzer
description: "Use when auditing SQL in this workspace — reviewing migrations, checking query performance, finding N+1 queries, analyzing transactions, reviewing indexes, or running a database health check. Triggers on \"sql audit\", \"migration review\", \"query optimization\", \"sql analysis\", \"database health\", \"index audit\", \"transaction review\", \"N+1 queries\", \"sql best practices\", \"migration safety\", or SQL/database quality requests."
allowed-tools: Read, Glob, Grep, Bash
---

# SQL Analyzer — Migration & Query Auditor

Analyze and optimize SQL migrations and SQLx queries across this workspace's microservices. Runs 27 checks organized into 5 categories: Migration Safety, Query Efficiency, PostgreSQL Best Practices, Security, and Naming & Formatting Conventions.

## Codebase Conventions

This skill targets this workspace, which uses:

- **PostgreSQL** as the database engine
- **SQLx** (Rust) with compile-time query verification
- **Application-layer query timeouts**, if the workspace has them (a wrapper trait or a
  per-query `.timeout(...)` call) — flag a query with none if the codebase otherwise has
  the convention; skip this check for a workspace that has no such convention at all
- **Schema-per-service** pattern — the schema is the **singular** service name (`example.`, …) — no exceptions, even for a service that is later renamed (see Check 27)
- **`uuidv7()`** for sortable primary keys (preferred over `gen_random_uuid()`)
- **Trigger-based `updated_at`** auto-management
- **Keyset pagination** with composite indexes `(created_at DESC, id DESC)`
- **Batch inserts** via dynamic SQL with `PgArguments` (avoids N+1)
- **Migrations** live in `services/<name>/migrations/*.sql`
- **Repositories** live in `services/<name>/src/adapter/repository.rs` or `src/port/*_repository.rs`

## All 27 Checks

| # | Check | Category |
|---|-------|----------|
| 1 | Missing indexes on foreign keys | Migration |
| 2 | Missing indexes on WHERE/ORDER BY columns | Migration |
| 3 | Overly broad transactions in migrations | Migration |
| 4 | Non-idempotent migrations | Migration |
| 5 | Missing NOT NULL constraints | Migration |
| 6 | Missing DEFAULT values on new columns | Migration |
| 7 | Unsafe enum type changes | Migration |
| 8 | Table-rewriting ALTER operations | Migration |
| 9 | Missing updated_at triggers | Migration |
| 10 | Schema isolation violations | Migration |
| 11 | Unnecessary transactions (single-query) | Query |
| 12 | Overly large transactions | Query |
| 13 | Missing timeout trait usage | Query |
| 14 | SELECT * anti-pattern | Query |
| 15 | N+1 query patterns | Query |
| 16 | Unbounded fetch_all without LIMIT | Query |
| 17 | Missing pagination on list endpoints | Query |
| 18 | Leading-wildcard LIKE patterns | Query |
| 19 | JSON instead of JSONB | PostgreSQL |
| 20 | gen_random_uuid() instead of uuidv7() | PostgreSQL |
| 21 | TIMESTAMP instead of TIMESTAMPTZ | PostgreSQL |
| 22 | Missing ON DELETE cascade rules | PostgreSQL |
| 23 | Unnecessary VARCHAR(n) constraints | PostgreSQL |
| 24 | SQL injection vectors | Security |
| 25 | Missing row-level tenant filtering | Security |
| 26 | Destructive migrations without guards | Security |
| 27 | Migration naming & formatting conventions | Naming |

## Execution

### Step 1: Discover Scope

1. List all services by scanning `services/*/migrations/` directories
2. List all repository files by scanning `services/*/src/adapter/repository.rs` and `services/*/src/port/*_repository.rs`
3. Present the discovered services to the user and confirm scope (all services or specific ones)

### Step 2: Run All 27 Checks

Execute every check below sequentially. For each check, scan all in-scope migration and/or repository files as indicated.

---

## Category A: Migration Checks (1–10)

Scan: `services/*/migrations/*.sql`

### Check 1 — Missing Indexes on Foreign Keys

**What:** Foreign key columns without a corresponding index cause slow JOINs and cascade deletes.

**How:**
1. Grep all migration files for `REFERENCES` clauses
2. Extract the column name and table being referenced
3. For each FK column, search for a `CREATE INDEX` on that column in the same or later migration
4. Flag any FK column that has no index

**Report format:**
```
⚠ [service/migration_file.sql] Column `table.column` has FK to `other_table(id)` but no index
```

### Check 2 — Missing Indexes on WHERE/ORDER BY Columns

**What:** Columns frequently used in WHERE or ORDER BY clauses in repository queries but not indexed.

**How:**
1. From repository files, extract column names used in `WHERE`, `ORDER BY`, and `GROUP BY` clauses
2. From migration files, extract all `CREATE INDEX` definitions
3. Cross-reference: flag columns queried frequently but never indexed
4. Ignore primary key columns (auto-indexed)

**Report format:**
```
⚠ [service] Column `table.column` used in WHERE clause (repository.rs:42) but no index found in migrations
```

### Check 3 — Overly Broad Transactions in Migrations

**What:** Migrations that combine DDL (schema changes) with large DML (data backfills) in one implicit transaction. This holds locks for too long on production databases.

**How:**
1. Scan each migration file for both DDL (`CREATE TABLE`, `ALTER TABLE`, `CREATE INDEX`) and DML (`INSERT INTO`, `UPDATE`, `DELETE FROM`) statements
2. Flag migrations that contain both DDL and DML (signs of schema change + data backfill in same transaction)
3. Also flag migrations with more than 5 DDL statements (overly broad schema changes)
4. Look for explicit `BEGIN`/`COMMIT` wrapping multiple heavy operations

**Report format:**
```
⚠ [service/migration.sql] Mixes DDL (ALTER TABLE) with DML (UPDATE) — consider splitting into separate migrations
⚠ [service/migration.sql] Contains 8 DDL statements — consider breaking into smaller migrations
```

### Check 4 — Non-Idempotent Migrations

**What:** Migrations that will fail on re-run because they lack `IF NOT EXISTS` / `IF EXISTS` / `OR REPLACE` guards.

**How:**
1. Scan for `CREATE TABLE` without `IF NOT EXISTS`
2. Scan for `CREATE INDEX` without `IF NOT EXISTS`
3. Scan for `CREATE TYPE` without checking for existence first
4. Scan for `DROP TABLE` / `DROP INDEX` without `IF EXISTS`
5. Scan for `CREATE FUNCTION` without `OR REPLACE`
6. Note: `ALTER TABLE` statements are inherently non-idempotent and should be noted but not flagged as errors

**Report format:**
```
⚠ [service/migration.sql:12] `CREATE TABLE users` — missing `IF NOT EXISTS`
```

### Check 5 — Missing NOT NULL Constraints

**What:** Columns that likely should be NOT NULL but aren't constrained. Focus on columns that are always populated in INSERT queries.

**How:**
1. From migration files, find all column definitions that allow NULL (no `NOT NULL`)
2. From repository files, check INSERT statements to see which of those columns are always provided (never NULL)
3. Flag columns that are always populated but not constrained
4. Exclude: JSONB metadata columns, optional metadata fields, nullable-by-design columns

**Report format:**
```
⚠ [service] Column `table.column` is nullable but always populated in INSERT (repository.rs:55) — consider adding NOT NULL
```

### Check 6 — Missing DEFAULT Values on New Columns

**What:** `ALTER TABLE ADD COLUMN` without a `DEFAULT` value will fail if the table has existing rows and the column is `NOT NULL`, or produce unexpected NULLs.

**How:**
1. Scan all migrations for `ALTER TABLE ... ADD COLUMN`
2. Flag any `ADD COLUMN ... NOT NULL` without a `DEFAULT`
3. Also flag `ADD COLUMN` without `NOT NULL` or `DEFAULT` (will silently be NULL for existing rows — intentional?)

**Report format:**
```
🔴 [service/migration.sql:8] `ALTER TABLE ADD COLUMN status NOT NULL` — no DEFAULT; will fail on non-empty table
⚠ [service/migration.sql:12] `ALTER TABLE ADD COLUMN bio` — no NOT NULL or DEFAULT; existing rows will be NULL
```

### Check 7 — Unsafe Enum Type Changes

**What:** PostgreSQL enum modifications have restrictions. Adding values can't be done inside a transaction (pre-v12). Removing values is not supported natively.

**How:**
1. Scan for `ALTER TYPE ... ADD VALUE` — flag if inside a `BEGIN`/`COMMIT` block
2. Scan for any attempt to remove or rename enum values
3. Scan for `CREATE TYPE ... AS ENUM` that redefine an existing type — check for `DROP TYPE` + `CREATE TYPE` migration pattern

**Report format:**
```
⚠ [service/migration.sql:5] `ALTER TYPE ... ADD VALUE` inside transaction — may fail on PostgreSQL < 12
```

### Check 8 — Table-Rewriting ALTER Operations

**What:** Certain ALTER TABLE operations rewrite the entire table, locking it for the duration. Dangerous on large tables.

**How:**
1. Scan for `ALTER TABLE ... ALTER COLUMN ... TYPE` (column type change = full rewrite)
2. Scan for `ALTER TABLE ... ADD COLUMN ... NOT NULL DEFAULT` on PG < 11 (rewrite on older versions)
3. Scan for `ALTER TABLE ... SET NOT NULL` (requires full table scan for validation)

**Report format:**
```
🔴 [service/migration.sql:3] `ALTER COLUMN status TYPE varchar` — rewrites entire table; will lock table during migration
```

### Check 9 — Missing updated_at Triggers

**What:** This codebase uses trigger-based `updated_at` auto-management. Tables with an `updated_at` column must have a corresponding trigger.

**How:**
1. Scan migration files for tables that define an `updated_at` column
2. For each such table, search all migrations for a `CREATE TRIGGER` on that table that calls `set_updated_at()` or similar
3. Flag tables with `updated_at` but no trigger

**Report format:**
```
⚠ [service] Table `schema.table_name` has `updated_at` column but no auto-update trigger
```

### Check 10 — Schema Isolation Violations

**What:** Each service should use its own PostgreSQL schema. Tables created in the `public` schema may collide across services.

**How:**
1. For each service, identify the expected schema (from `CREATE SCHEMA` or schema-qualified table names)
2. Flag any `CREATE TABLE` that uses `public.` or no schema qualification
3. Note: the `identity` service may use `public` schema — check existing convention

**Report format:**
```
⚠ [service/migration.sql:4] `CREATE TABLE users` — no schema qualification; should be `identity.users` or similar
```

---

## Category B: Query Checks (11–18)

Scan: `services/*/src/**/*.rs` (repository and handler files)

### Check 11 — Unnecessary Transactions (Single-Query)

**What:** Wrapping a single query in `pool.begin()` / `tx.commit()` adds overhead with zero benefit. A single query is already atomic.

**How:**
1. Find all `pool().begin()` or `db.pool().begin()` usages
2. Count the number of query executions between `begin()` and `commit()`
3. Flag transactions that contain only a single query

**Report format:**
```
⚠ [service/repository.rs:42] Transaction wraps only 1 query — remove transaction for better performance
```

### Check 12 — Overly Large Transactions

**What:** Transactions spanning many queries hold locks longer and increase contention. Consider whether all queries truly need atomicity.

**How:**
1. Find all transaction blocks (`begin()` to `commit()`)
2. Count queries within each transaction
3. Flag transactions with more than 5 queries
4. Assess whether all queries need to be atomic or can be split

**Report format:**
```
⚠ [service/repository.rs:80] Transaction contains 8 queries — consider splitting into smaller atomic units
```

### Check 13 — Missing Timeout Trait Usage

**What:** This codebase has custom timeout traits (`fetch_one_timeout`, `execute_timeout`, etc.). Raw SQLx methods bypass application-level timeouts.

**How:**
1. Grep for `.fetch_one(`, `.fetch_optional(`, `.fetch_all(`, `.execute(` in repository files
2. Exclude usages on transactions (`&mut *tx`) — transaction queries use the tx_ext traits differently
3. Flag any direct pool query that uses raw SQLx methods instead of the `_timeout` variants

**Report format:**
```
⚠ [service/repository.rs:23] `.fetch_one(pool)` — use `.fetch_one_timeout(pool)` for application-layer timeout
```

### Check 14 — SELECT * Anti-Pattern

**What:** `SELECT *` fetches all columns, including potentially large ones (JSONB, TEXT). Explicit column lists are preferred.

**How:**
1. Grep for `SELECT *` or `SELECT \*` in SQL strings within Rust files
2. Also check for `RETURNING *`
3. `RETURNING *` after INSERT/UPDATE is acceptable if the struct needs all columns
4. Flag `SELECT *` in general queries — especially in list/search endpoints

**Report format:**
```
⚠ [service/repository.rs:15] `SELECT * FROM users` — specify explicit columns to avoid fetching unnecessary data
```

### Check 15 — N+1 Query Patterns

**What:** Executing database queries inside a loop creates N+1 performance problems.

**How:**
1. Find all `for` and `while` loops in repository files
2. Check if any loop body contains `.fetch_one`, `.fetch_optional`, `.fetch_all`, `.execute`, or `sqlx::query`
3. Also check for `.map()`, `.for_each()`, or async stream iterations containing queries
4. Suggest batch alternatives (IN clause, JOIN, CTE, dynamic VALUES)

**Report format:**
```
🔴 [service/repository.rs:67] Query inside `for` loop — N+1 pattern. Use batch query with IN clause or JOIN
```

### Check 16 — Unbounded fetch_all Without LIMIT

**What:** `fetch_all` without a LIMIT clause can return unbounded rows, causing memory issues.

**How:**
1. Find all `.fetch_all` usages
2. Check if the associated SQL string contains `LIMIT`
3. Flag queries without LIMIT that aren't inherently bounded (e.g., filtering by a unique key is fine)
4. Also check for queries filtered by FK (could still return many rows)

**Report format:**
```
⚠ [service/repository.rs:90] `fetch_all` without LIMIT — could return unbounded rows. Add LIMIT or use pagination
```

### Check 17 — Missing Pagination on List Endpoints

**What:** Handler functions that return lists should implement pagination (keyset or offset-based).

**How:**
1. Find repository methods that return `Vec<T>` or similar collections
2. Check if the corresponding SQL uses pagination (LIMIT + OFFSET, or keyset with WHERE + ORDER BY + LIMIT)
3. Cross-reference with handler/route files to identify list endpoints
4. This codebase prefers keyset pagination with `(created_at DESC, id DESC)`

**Report format:**
```
⚠ [service/repository.rs:100] `list_widgets()` returns Vec without pagination — add keyset pagination
```

### Check 18 — Leading-Wildcard LIKE Patterns

**What:** `LIKE '%term'` or `LIKE '%term%'` with a leading wildcard cannot use B-tree indexes and causes full table scans.

**How:**
1. Grep for `LIKE '%` or `ILIKE '%` patterns in SQL strings
2. Suggest alternatives: full-text search (`tsvector`), trigram index (`pg_trgm`), or reversed index for suffix matching

**Report format:**
```
⚠ [service/repository.rs:45] `LIKE '%' || $1 || '%'` — leading wildcard prevents index usage. Consider pg_trgm or full-text search
```

---

## Category C: PostgreSQL Best Practices (19–23)

Scan: `services/*/migrations/*.sql`

### Check 19 — JSON Instead of JSONB

**What:** The `JSON` type stores text verbatim and can't be indexed. `JSONB` is binary, indexable, and faster for queries.

**How:**
1. Grep migration files for column definitions using `JSON` type (not `JSONB`)
2. Match pattern: column name followed by `JSON` (not `JSONB`)

**Report format:**
```
⚠ [service/migration.sql:8] Column `metadata` uses JSON type — use JSONB for indexing and query performance
```

### Check 20 — gen_random_uuid() Instead of uuidv7()

**What:** `gen_random_uuid()` (UUIDv4) is random and scatters B-tree index inserts. `uuidv7()` is time-sortable and friendlier to indexes.

**How:**
1. Grep migration files for `gen_random_uuid()`
2. Check if it's used as a primary key default — those should prefer `uuidv7()`
3. Non-PK uses (e.g., tokens, nonces) may legitimately want random UUIDs — note but don't flag

**Report format:**
```
⚠ [service/migration.sql:3] PK default `gen_random_uuid()` — consider `uuidv7()` for time-sortable, index-friendly IDs
```

### Check 21 — TIMESTAMP Instead of TIMESTAMPTZ

**What:** `TIMESTAMP` (without time zone) silently drops timezone information. Always use `TIMESTAMPTZ` for correctness.

**How:**
1. Grep migration files for `TIMESTAMP` that is NOT followed by `TZ` or `WITH TIME ZONE`
2. Case-insensitive match
3. Exclude comments

**Report format:**
```
⚠ [service/migration.sql:6] Column uses `TIMESTAMP` — use `TIMESTAMPTZ` to preserve timezone information
```

### Check 22 — Missing ON DELETE Cascade Rules

**What:** Foreign keys without explicit `ON DELETE` behavior default to `RESTRICT`, which can cause unexpected constraint violations during cleanup.

**How:**
1. Find all `REFERENCES` clauses in migration files
2. Check if each has an explicit `ON DELETE` clause (CASCADE, SET NULL, SET DEFAULT, or RESTRICT)
3. Flag any FK without explicit ON DELETE — the default (RESTRICT) should be intentional, not accidental

**Report format:**
```
⚠ [service/migration.sql:12] FK `widget_id REFERENCES widgets(id)` — no explicit ON DELETE. Add ON DELETE CASCADE/SET NULL/RESTRICT
```

### Check 23 — Unnecessary VARCHAR(n) Constraints

**What:** PostgreSQL `TEXT` and `VARCHAR` have identical performance. `VARCHAR(n)` adds an arbitrary length check that often causes problems when requirements change.

**How:**
1. Grep migration files for `VARCHAR(` definitions
2. Flag any `VARCHAR(n)` usage
3. Suggest `TEXT` with a `CHECK (length(col) <= n)` constraint if a length limit is truly needed (easier to modify later)

**Report format:**
```
⚠ [service/migration.sql:5] Column uses `VARCHAR(255)` — prefer TEXT (same performance, no arbitrary limit)
```

---

## Category D: Safety & Security Checks (24–26)

Scan: `services/*/src/**/*.rs` and `services/*/migrations/*.sql`

### Check 24 — SQL Injection Vectors

**What:** String concatenation or interpolation in SQL queries bypasses parameterized query protection.

**How:**
1. Grep Rust files for `format!` or string concatenation (`+`) near `sqlx::query` or SQL strings
2. Check for `&format!("SELECT` patterns
3. Allowlist: dynamic column lists and table names (can't be parameterized) — but flag them for review
4. The batch insert pattern using `write!` for VALUES placeholders (`${n}`) is safe — don't flag placeholder construction

**Report format:**
```
🔴 [service/repository.rs:33] SQL string built with `format!` — potential injection vector. Use parameterized queries ($1, $2)
```

### Check 25 — Missing Row-Level Tenant Filtering

**What:** In a multi-tenant system, queries that don't filter by owner/tenant/org can leak data across tenants.

**How:**
1. Identify tables with tenant-scoping columns (`user_id`, `widget_id`, `org_id`, `owner_id`, `created_by`)
2. Find SELECT queries on those tables
3. Flag queries that don't include the scoping column in their WHERE clause
4. Exclude: admin/system queries, aggregation queries, migration scripts

**Report format:**
```
⚠ [service/repository.rs:78] `SELECT FROM documents` — no `widget_id` filter. Potential cross-tenant data leak
```

### Check 26 — Destructive Migrations Without Guards

**What:** `DROP TABLE`, `DROP COLUMN`, `TRUNCATE`, and `DELETE FROM` (without WHERE) in migrations can cause irreversible data loss.

**How:**
1. Grep migration files for `DROP TABLE`, `DROP COLUMN`, `TRUNCATE`, `DELETE FROM`
2. Check if destructive operations have safety guards:
   - `IF EXISTS` for DROP operations
   - A preceding data migration/backup step
   - A comment explaining why the destruction is safe
3. Flag any unguarded destructive operation

**Report format:**
```
🔴 [service/migration.sql:2] `DROP TABLE sessions` — destructive operation. Add `IF EXISTS` and document data migration plan
⚠ [service/migration.sql:8] `DELETE FROM users` — unbounded DELETE. Add WHERE clause or document intent
```

---

## Category E: Naming & Formatting Conventions (27)

Scan: `services/*/migrations/*.sql`

### Check 27 — Migration Naming & Formatting Conventions

**What:** Every migration must follow the workspace's naming and layout conventions so any
service's schema reads like every other's. This check is about *consistency*, not safety —
idempotency is Check 4 and index *presence* is Checks 1–2; 27 governs only how things are
**named and laid out**. Six sub-rules:

**27.1 — Singular schema = service name.** A service's schema is the **singular** service
directory name — `services/<name>/` → schema `<name>`. This template's own example follows it:
`services/example/` → schema `example` (see `services/example/migrations/0001_schema.sql`).
- **Reserved-word rule:** when the singular name is a SQL reserved word (e.g. a service
  naturally called `order` or `user`), avoid it by renaming the service, not by pluralising
  the schema. Any deviation must carry a comment.
- Flag any `CREATE SCHEMA` or schema-qualified table using a plural or non-service-matching
  schema (e.g. a stray `examples.`, or a schema that doesn't match any `services/<name>/`).

**27.2 — Tables must not repeat the schema.** A table name must not begin with its own schema
followed by `_` — the schema already namespaces it.
- This template's own `example.widgets` is the compliant shape: the schema is `example`, the
  table is `widgets`, and neither repeats the other.
- The violation this check flags: `example.example_widgets` (repeats the schema) — the fix is
  `example.widgets`.
- A table that merely *contains* a similar word is fine: `example.widget_variants` is not a
  violation (it does not *begin with* `example_`).
- Flag every `CREATE TABLE <schema>.<schema>_<rest>`, including on an existing migration that
  already shipped this way — the fix still applies, it's just heavier. **A rename ripples into
  every SQLx query, `FromRow` struct, and FK reference — treat it as a coordinated migration +
  code change, never a migration-only edit.**

**27.3 — Index naming `idx_<table>_<purpose>`.** Indexes are prefixed `idx_`, then the table,
then the indexed columns/purpose: `idx_photos_scope`, `idx_contacts_user_id`.
- Flag the suffix style `<table>_<purpose>_idx` and any index name not matching `^idx_`.

**27.4 — Trigger naming `trg_<table>_updated_at`.** The `updated_at` auto-update trigger is
named `trg_<table>_updated_at`.
- Flag the `update_<table>_updated_at`, `set_<table>_updated_at`, and infix
  `trg_<table>_set_updated_at` variants.

**27.5 — Idempotent DDL (defer to Check 4).** `CREATE … IF NOT EXISTS` for
schemas/tables/indexes/types, `CREATE OR REPLACE FUNCTION`, and `CREATE OR REPLACE TRIGGER`
(or `DROP TRIGGER IF EXISTS` + `CREATE`) are part of the same consistency umbrella. **Do not
re-report here** — Check 4 owns idempotency findings; 27.5 exists only so the convention list
is complete.

**27.6 — Section headers.** A migration with two or more logical sections delimits each with a
full-width rule comment: `-- ` followed by a run of `=`, e.g.
```
-- ============================================================================
-- Contacts
-- ============================================================================
```
- Flag multi-section migrations that use ad-hoc separators (bare `--`, blank lines only, or
  inconsistent rule widths within one file).

**How:**
1. **27.1** — derive the expected schema from the migration's `services/<name>/` path (singular;
   note any deliberate reserved-word rename documented in a comment). Grep `CREATE SCHEMA` and
   `CREATE TABLE <schema>.`; flag any schema that is plural or does not equal the service name.
2. **27.2** — for each `CREATE TABLE <schema>.<table>`, flag when `<table>` starts with
   `<schema>_`. Suggested fix = `<schema>.<table-without-schema-prefix>`.
3. **27.3** — grep `CREATE INDEX [IF NOT EXISTS] <name>`; flag any `<name>` not matching `^idx_`.
4. **27.4** — grep `CREATE TRIGGER <name>`; flag any `<name>` not matching `^trg_.*_updated_at$`.
5. **27.5** — no scan; defer to Check 4.
6. **27.6** — for migrations with 2+ `CREATE TABLE`/logical blocks, confirm `-- =+` delimiter
   lines are present and uniform.

**Report format:**
```
🔴 [example/0002.sql:12] Table `examples.examples` — schema must be singular `example` (matches service dir)
⚠ [example/0003.sql:20] Table `example.example_variants` duplicates schema — rename to `example.variants` (ripples into Rust)
⚠ [example/0004.sql:8]  Index `widgets_pagination_idx` — use prefix form `idx_widgets_pagination`
⚠ [example/0005.sql:44] Trigger `update_widgets_updated_at` — canonical is `trg_widgets_updated_at`
✅ [example/0001.sql:7]  Schema `example` — OK (singular service name)
```

**Severity:**
- 🔴 a table/schema in a **wrong or non-existent schema** (27.1 plural stray, 27.2 that would
  point a FK at a renamed table) — a latent correctness bug.
- ⚠ everything else (index/trigger names, section headers) — consistency debt.

---

## Step 3: Generate Report

After all 26 checks, produce a consolidated report:

### Report Format

```
# SQL Analysis Report — [date]

## Scope
Services analyzed: [list]
Migration files scanned: [count]
Repository files scanned: [count]

## Summary

| Category | Checks | 🔴 Critical | ⚠ Warning | ✅ Pass |
|----------|--------|-------------|-----------|--------|
| Migration Safety (1–10) | 10 | n | n | n |
| Query Efficiency (11–18) | 8 | n | n | n |
| PostgreSQL Best Practices (19–23) | 5 | n | n | n |
| Safety & Security (24–26) | 3 | n | n | n |
| Naming & Formatting (27) | 1 | n | n | n |
| **Total** | **27** | **n** | **n** | **n** |

## Critical Issues (🔴)
[List all critical findings — must be fixed]

## Warnings (⚠)
[List all warnings — should be fixed]

## Passed Checks (✅)
[List checks with no findings]

## Recommendations
[Prioritized list of suggested improvements, grouped by effort level]
```

### Severity Levels

- **🔴 Critical** — Security risk, data loss risk, or guaranteed production failure
  - SQL injection vectors (#24)
  - N+1 in hot paths (#15)
  - Destructive migrations without guards (#26)
  - NOT NULL without DEFAULT on non-empty table (#6)
  - Table-rewriting ALTER on large tables (#8)
- **⚠ Warning** — Performance issue, best practice violation, or maintainability concern
  - Everything else
- **✅ Pass** — Check ran, no issues found

## Verification Checklist

Before presenting the report, verify:

- [ ] All 27 checks were executed
- [ ] Every finding includes file path and line number (or line range)
- [ ] Critical vs warning severity is correctly assigned
- [ ] No false positives from safe patterns (batch insert placeholder construction, transaction queries, admin queries)
- [ ] The summary table counts match the detailed findings
- [ ] Recommendations are actionable and prioritized
