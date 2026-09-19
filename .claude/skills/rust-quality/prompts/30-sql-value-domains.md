# SQL Value Domains (`TEXT` + `CHECK`, not `CREATE TYPE ... AS ENUM`)

Audit how closed value sets — stages, statuses, kinds, categories — are represented in `migrations/*.sql` and in the Rust types that read them. This workspace's convention is `TEXT` constrained by a `CHECK`, with the Rust enum as the type-safe boundary. This check is about that representation, not about whether a given token set is *correct* (that's a domain question) and not about query safety (that's `sql-analyzer`).

## Why

A Postgres enum is the wrong tool for a value set that changes. Adding a value needs `ALTER TYPE ... ADD VALUE` — it can't be reordered, and it fights the transaction that `migrate!` wraps each migration in. *Removing* a value has no DDL at all: you recreate the type and rewrite every column that uses it. A `CHECK` constraint is dropped and re-added in a single transactional statement, so a value-set change is an ordinary migration file instead of a schema surgery.

The type safety was never Postgres's job here. It lives in the Rust enum (`#[sqlx(type_name = "text")]` + `#[serde(rename_all = "snake_case")]`), which is what actually stops a caller passing a raw string. The `CHECK` only stops garbage reaching the table. Trading a rigid schema type for a flexible one costs nothing the Rust side wasn't already providing.

`services/example`'s own tables don't happen to need a closed value set yet, so there's nothing to point to here as a live example — but the first time a service in this workspace adds one (a `stage`, `status`, or `kind` column), it should follow the convention below, and every service after it should follow the first one's lead.

## The convention

```sql
-- Closed token set: TEXT + CHECK, constraint named so a later migration can address it.
stage TEXT NOT NULL DEFAULT 'active'
    CONSTRAINT widgets_stage_check CHECK (stage IN ('active', 'completed', 'archived')),
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WidgetVisibility { Public, Private }
```

- Tokens are **snake_case** and match the domain's shared vocabulary exactly across every place the concept appears (`completed`, never `complete`).
- The constraint is **named** when the set can plausibly change; an unnamed inline `CHECK` is acceptable only for a set that genuinely can't grow.
- Changing the set = `DROP CONSTRAINT` + `ADD CONSTRAINT` in one new migration; removing a token backfills the affected rows in the same file, before the new constraint lands.
- Non-token invariants belong in `CHECK` too: ranges, interval ordering (`end_at > start_at`), canonical forms (`email = LOWER(TRIM(email))`).

## Flag

- **`CREATE TYPE ... AS ENUM` in a new or recently-added migration** — the primary defect. Report the type, every column bound to it, and the `TEXT` + `CHECK` migration that replaces it.
- **`#[sqlx(type_name = "<schema>.<type>")]`** on a Rust enum — the schema-qualified name is the tell that the column is a Postgres enum. Pair the finding with the migration that created it.
- **Unconstrained `TEXT`** holding a closed set — a column whose only guard is the Rust enum, with no `CHECK` at all. The DB will happily store a typo written by a migration, a backfill script, or `psql`.
- **Unnamed `CHECK` on a set that has already changed** (a later migration re-adding it via Postgres's generated name, or working around not being able to).
- **Token drift** — a `CHECK` list or enum variant whose wire token disagrees with how the same concept is spelled elsewhere in the workspace (a shared domain-types crate, if one exists, or another service's copy of the same enum).
- **A schema invariant enforced only in Rust** where a `CHECK` expresses it directly — range, interval ordering, non-empty-after-trim, canonical casing.

## Do NOT flag

- **Pre-existing enum types in legacy services** as work to do now. They are legacy; note them once as known debt and move on. Migrating one is its own reviewed change, never a drive-by inside a quality sweep. *Do* flag a **new** column or a **new** value being added to them.
- **`#[sqlx(type_name = "text")]` / `"TEXT"`** — that is the convention, not a deviation. Casing of `"text"` is not a finding.
- **`CHECK` on an open set** that's deliberately open (a `ScopeType` whose rustdoc documents that scope kinds grow) — read the rustdoc before calling a missing `CHECK` a defect.
- **Enums in third-party or generated schemas** the platform doesn't own.
- **Existing migration files.** They are immutable history — the fix is always a *new* migration, never an edit. A finding that proposes editing `0001_*.sql` is wrong.

## Report

Per finding: `column or type | file:line | issue (pg-enum / schema-qualified-sqlx-type / unconstrained-text / unnamed-check / token-drift / rust-only-invariant) | fix`. Lead with `CREATE TYPE ... AS ENUM` in new migrations and unconstrained `TEXT` — those are the ones that get expensive later. For each, give the replacement DDL as a new migration file, and state whether a backfill is needed.
