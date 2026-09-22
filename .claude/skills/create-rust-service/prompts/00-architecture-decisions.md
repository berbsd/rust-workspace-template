# Architecture Decisions — always runs first

**Not part of the menu.** Run this before `01-crate-scaffold.md` (new
service) or `05-feature-domain-port.md` (new feature on an existing
service), every time — the same way `rust-quality`'s check #33 always runs
first. Nothing here produces a file; it decides which of the later prompts
apply and in what shape, before any of them write code.

## Why this exists

The rest of this skill's prompts describe one shape in full: Postgres-backed
persistence, full CRUD, no caching. That shape is a **default for the common
case**, not a contract every feature must fill in regardless of what it
actually does. Generating `port.rs` + `adapter/postgres.rs` + a migration for
a feature that stores nothing, or a `delete` handler for a feature that's
strictly append-only, is the same "no unrequested functionality" violation
AGENTS.md bans for hand-written code — a template is not an exemption.
Answer every question below and hold the answers for every later prompt.

## Questions

### 1. Does this feature persist state?

- **Postgres (the default)** — the feature stores rows and needs
  database-backed access. Run `05`–`09` as written: `port.rs` is a storage
  trait, `adapter/postgres.rs` implements it, a migration creates the table.
- **No persistence** — a pure computation, a proxy/aggregation over another
  service's API, or something held only in memory for the process lifetime.
  Then, relative to the default shape:
  - Skip `adapter/postgres.rs` and the migration in
    `07-feature-adapter-postgres.md` entirely — that prompt does not run.
  - `port.rs` either doesn't exist (the service has no external
    collaborator worth abstracting) or names whatever the feature actually
    depends on (an HTTP client trait for a proxying feature) — never invent
    a storage trait for a feature that stores nothing.
  - `domain.rs` still holds the entity/result shape if one exists, without
    `sqlx::FromRow` and without the name `{{Feature}}Row` (that suffix
    means "this is a persistence row" — don't use it for a type that was
    never a database row). Call it `{{Feature}}` or whatever the feature's
    actual internal shape is.
  - `service.rs` depends on whatever real collaborator replaces the
    repository (an `Arc<dyn SomeClient>`, nothing at all for a pure
    computation) — the constructor's shape follows from what Step 1
    gathered, not from `06-feature-service.md`'s literal signature.
  - Say so explicitly in the Step 4 report; don't silently generate
    Postgres plumbing "to match the other features" in the same service.

### 2. Which operations does this feature actually need?

Default CRUD (`create`/`get`/`list`/`delete`) is `services/example`'s
`widget` shape and this skill's own worked examples — not a mandatory
contract. Before running `05`–`09`, decide the real set:

- Read-only (a computed or externally-sourced view)? No `create`/`delete` —
  `port.rs`/`service.rs`/`adapter/http.rs` only ever define `get`/`list`.
- Append-only (an event log, an audit trail)? No `delete`.
- Upsert-only (idempotent by a natural key, not a server-minted id)? One
  `put`-shaped method, not separate `create`/`update`.
- Something narrower than all four verbs above, in general.

Trim every one of `05`–`09`'s templates to the real operation set as you
write them — an unused method that returns `unimplemented!()` to satisfy a
trait nobody calls is worse than not generating the method, and it violates
the zero-panic policy besides (`unimplemented!` is denied workspace-wide).

### 3. Does this feature need caching?

**Default: no.** Add a cache layer only for a concrete, named reason — a
slow or rate-limited upstream call, a hot read path with a measured cost —
never preemptively "in case it helps later." If the answer is yes:

- The cache wraps the *port*: either a decorator implementing the same
  `{{Feature}}Repository` trait around the real one, or an explicit check
  inside `adapter/postgres.rs` (or the outbound client adapter) before the
  query/call runs. Never scatter ad hoc cache calls through `service.rs` —
  that layer stays about domain rules, not caching mechanics.
- Give it a descriptive, `snake_case` name distinct from every other cache
  in the service (`TtlCache::new("{{feature}}_by_id")`, never a bare
  `"cache"`), and an explicit eviction path on every mutation that
  invalidates it. `rust-quality` check #20 is the audit for exactly this
  shape, once one exists in this workspace.
- **This workspace has no caching crate as a dependency today** (see
  `SKILL.md`'s "Out of scope by default"). Adding one is its own decision —
  surface it explicitly and get it confirmed before adding the dependency;
  don't bundle "add `moka`/`ttl-cache` to the workspace" silently into "add
  a feature."

### 4. Anything else this workspace doesn't have yet?

Re-check `SKILL.md`'s "Out of scope by default" list — metrics, auth
guards, structured event publishing. If this feature seems to need one of
those, that is a workspace-level infrastructure decision (add the
prerequisite crate, as its own reviewed change), not something to improvise
inline for one feature because the reference platform (`stillgood/platform`'s
`profile` service) happens to have it.

## Report the decisions before generating anything

State, in one line each, before running `01`/`05` onward:

```
Persistence: Postgres | none — <why>
Operations:  <the real verb set, e.g. "get, list — read-only">
Caching:     none | <what, and the concrete reason>
Other infra needed: none | <name it, flag as a separate decision>
```

Get this confirmed (or confirm it matches the user's explicit request)
before writing a single file — these answers change which files exist at
all, and re-deciding after generating them means deleting work, not editing
it.
