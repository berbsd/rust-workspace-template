# Verification — always runs

**Not optional**, the same way `rust-quality`'s check #33 runs regardless of
what the user picked. Run this after prompts 1–11 for a new service, and
after `13-add-feature.md` for a new feature on an existing one. A service
that "looks scaffolded" but was never actually compiled and exercised is not
done — it's a pile of files shaped like Rust.

## Step 1: Static checks

```bash
cargo +nightly fmt -p {{service}}
cargo check -p {{service}} --all-targets --all-features
cargo clippy -p {{service}} --no-deps --all-targets --all-features
```

Zero warnings. This workspace's `[workspace.lints]` denies `unwrap_used`,
`expect_used`, `panic`, and friends outside `#[cfg(test)]` — a clippy error
here almost always means a template step's `{{placeholder}}` substitution
introduced one of those, not that the lint config needs an `#[allow(...)]`.

## Step 2: Placeholder grep

```bash
grep -rn '{{' services/{{service}}/ && echo "UNRESOLVED — fix before continuing" || echo "clean"
```

Any hit is a broken generation, not a TODO to leave for later.

## Step 3: Full test suite

```bash
just db-ensure
cargo nextest run -p {{service}} --all-features
cargo test --doc -p {{service}} --all-features
```

`0 skipped` in nextest's `Summary` line. This runs both `service.rs`'s
domain-rule unit tests (no database) and `tests/{{feature}}_it.rs`'s
end-to-end tests (real throwaway Postgres) — both must pass, not just one.

## Step 4: Live smoke test

Static checks and tests prove the code is correct; this step proves the
*process* actually starts and answers real HTTP requests — the one thing
nothing above exercises (nextest never boots `main.rs`).

```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/postgres"
export PORT=8089
cargo run -p {{service}} &
SERVICE_PID=$!
sleep 2

curl -sf localhost:8089/health
curl -sf -X POST localhost:8089/{{feature_plural}} \
  -H 'content-type: application/json' \
  -d '{{create_request_body_example}}' | tee /tmp/{{feature}}_smoke.json

ID=$(python3 -c "import json;print(json.load(open('/tmp/{{feature}}_smoke.json'))['id'])")
curl -sf "localhost:8089/{{feature_plural}}/$ID"
curl -sf localhost:8089/{{feature_plural}}
curl -sf -X DELETE "localhost:8089/{{feature_plural}}/$ID" -o /dev/null -w '%{http_code}\n'

kill $SERVICE_PID
```

Every `curl -f` must exit `0` (a non-2xx status makes `-f` fail loudly,
which is the point — a silent `curl` without `-f` would hide a `500`).
Confirm the delete's status line reads `204`.

## Step 5: Workspace-level gate

```bash
just check
```

The same gate every other change in this workspace passes before landing —
format, lint, the full test suite (including every other service, so a
change to a shared crate this new service now depends on hasn't broken
anything else), typos, `cargo-deny`.

## Report format

```
services/{{service}}/ — N files created (prompts 1–11)
  Static checks:   clean (fmt, check, clippy)
  Unit tests:      N passed, 0 skipped
  Integration tests: N passed, 0 skipped
  Smoke test:      POST 201, GET 200, GET (list) 200, DELETE 204, GET (post-delete) 404
  just check:      passed
Open items for the user:
  - {{list anything Step 1 of SKILL.md left undecided — auth layer, extra config, a second feature}}
```
