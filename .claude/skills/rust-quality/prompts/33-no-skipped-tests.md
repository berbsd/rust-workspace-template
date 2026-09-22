# Check 33 — No Skipped Tests

**Non-negotiable.** A test that does not run is not a test. It is a comment that
costs CI time to skip, and it reports as healthy infrastructure while asserting
nothing.

## The rule

`#[ignore]` is banned in this workspace. The single exception is a **doctest**
that genuinely cannot execute — a fenced block illustrating an API that would
need real credentials or a live external service — and even there prefer
` ```no_run `, which still type-checks and still catches a signature change.

There is no "requires a database" exemption and no "requires a server"
exemption. Both have standard answers here:

| Excuse | The actual answer |
|---|---|
| `"requires TEST_DATABASE_URL"` | `#[sqlx::test(migrations = "./migrations")]` — provisions a throwaway database per test, isolated, no truncation, no races |
| `"Requires running PostgreSQL"` | Same. `just db-ensure` starts the container idempotently |
| `"requires a running service"` | `wiremock::MockServer` — every `*-client` test in this tree already does this |
| `"costs money / needs network"` | The call does not belong in a test. Stub the transport |
| `"stale — needs rewriting"` | Then rewrite it or delete it. A stale ignored test is a lie about coverage |

## Why this is stricter than it looks

A skipped test is *worse* than a missing one. A missing test is visibly missing.
A skipped test appears in the run, prints a reassuring line, and contributes to a
green summary — so nobody counts it, and the gap it leaves is invisible precisely
where someone believed they had cover.

A workspace can accumulate tests behind `#[ignore = "requires
TEST_DATABASE_URL"]` and friends for years without anyone noticing — often
long after the excuse stopped being true (e.g. once `just db-ensure` starts a
throwaway database on demand, "requires a database" is no longer a real
blocker). Nothing in the suite says so, because a skip is indistinguishable
from a pass in the summary line. A regression test written to pin a
deliberate behaviour change is just as vulnerable to this: if it's ignored,
it has never actually executed, no matter how carefully it was written.

## How to run this check

1. **Find every skip.** Both forms below are now mechanized as `ast-grep`
   rules (see `.ast-grep/README.md` — `no-skipped-tests.yml`,
   `no-silent-env-skip.yml`) rather than hand-derived greps:
   ```bash
   ast-grep scan -c .ast-grep/sgconfig.yml services crates hosts
   ```
   This catches `#[ignore]`/`#[ignore = "..."]` anywhere, and the narrower
   silent-env-skip shape (`if env::var($X).is_err() { ...return... }` inside a
   `#[test]`/`#[tokio::test]`/`#[*::test]`-attributed function — scoped that
   way so a legitimate production feature-flag guard isn't flagged).
   **Enforced in `lefthook.yml`'s pre-commit hook** — a real violation
   already fails the commit on its own, as long as hooks are installed and
   nobody bypasses with `--no-verify`. The matching CI step exists in
   `.github/workflows/ci.yml` but is currently disabled (`if: false`, to cut
   CI overhead for a single-developer repo), so this isn't a fully
   un-bypassable gate — running it here during a full sweep is a real second
   check, not just a formality.

   One shape stays a manual grep — the rules can't cross-reference a crate's
   default-feature set, so a test module gated behind a non-default
   `#[cfg(feature = "...")]` isn't caught mechanically:
   ```bash
   grep -rln '#\[cfg(feature' --include="*.rs" services crates | xargs grep -l 'mod tests\|#\[test\]'
   ```
   Confirm by hand whether the named feature is on by default; if not, this
   is the same invisible-skip problem in different clothes (see "What this
   check does NOT flag" below).

2. **Classify each one.** For every hit, answer: *what would it take to make
   this run?* Against the table above, almost always the answer is a mechanical
   conversion, not a redesign.

3. **Convert, do not delete by default.** A DB test becomes `#[sqlx::test]`;
   its setup helper takes `pool: PgPool` instead of building its own client from
   an env var. Drop any hand-rolled `run_migrations` and `TRUNCATE` — `sqlx`
   gives each test a clean database, which is *why* the races that motivated the
   shared-database design disappear.

4. **Delete only with a reason in the commit.** If a test genuinely cannot run —
   it asserts against a retired service, it needs a paid API — deleting it and
   saying so is honest. Leaving it as decoration is not.

5. **Verify it actually ran.** Converting is not enough; confirm the count went
   up. This workspace runs tests through nextest, not bare `cargo test` — its
   summary line reports `skipped`, not `test result: ... N ignored`:
   ```bash
   just db-ensure && cargo nextest run -p <crate> --all-features 2>&1 | grep 'Summary'
   ```
   The `skipped` count in that line must be `0`. A conversion that leaves the
   test filtered out for a different reason (a non-default feature gate, a
   nextest filter in `.config/nextest.toml`) has changed nothing.

## Report format

```
🔴 [service/tests/foo.rs:42] `#[ignore = "requires TEST_DATABASE_URL"]` on
   `test_bar` — convert to `#[sqlx::test]`; `setup()` should take `pool: PgPool`
⚠ [service/tests/baz.rs:88] early `return` when `API_KEY` is unset — a silent
   skip; stub the transport instead
```

## What this check does NOT flag

- ` ```no_run ` doctests — they compile and type-check, which is the point
  (and doctests inside `///` comments are outside `no-skipped-tests.yml`'s
  `.rs`-syntax matching entirely, not just intentionally excluded)
- `#[cfg(feature = "...")]` on a test **module**, *provided* the feature is on by
  default or the workspace build enables it. If it is not, the tests are skipped
  in exactly the way this check exists to catch, and it should be reported — a
  test module gated behind a non-default feature is invisible in exactly the
  same way an `#[ignore]`'d test is: the suite is green without it, and nobody
  notices the gap.
