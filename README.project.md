# {{PROJECT_NAME}}

Rust 2024 microservices workspace: a `justfile`-driven, lint-heavy, feature-first-service
setup generated from [rust-workspace-template](https://github.com/berbsd/rust-workspace-template).

## What's here

- **`crates/common-types/`** — the one shared library: the API error envelope
  (`ApiErrorBody` + `ApiErrorMapping` + `impl_api_error_response!`) and keyset pagination
  (`PaginatedResponse`, `Cursor`, `KeysetCursor`). Deliberately minimal — no auth types, no
  typed ids, no validation framework. Add to it only once two services genuinely need the
  same code.
- **`services/`**, **`hosts/`**, **`jobs/`**, **`workers/`** — empty (each just carries a
  `.gitkeep` so their `Cargo.toml` glob has something to match) until you add a first one
  with the `create-rust-service` skill; see `AGENTS.md` for the shape and conventions each
  follows.
- **`.claude/skills/`** — engineering-discipline skills: `create-rust-service`
  (scaffolds a new service or feature slice), `rust-quality` (including handler hygiene,
  check #34), `rust-documenter` (rustdoc, and OpenAPI/utoipa/Scalar annotations if the
  service adds them — see its `references/openapi-annotations.md`),
  `rust-tracing-instrument`, `sql-analyzer`, `tool-readiness`, `validate-implementation`.
  Some `rust-quality` checks describe patterns this workspace doesn't have yet (an outbox,
  a JWT revocation cache) — skip those until the workspace grows them.
- **`.mcp.json`** — registers the [Context7](https://context7.com) MCP server, so Claude
  Code (or any other MCP-compatible client) gets live, version-accurate library/framework
  documentation lookups natively, without shelling out to a CLI. Works unauthenticated out
  of the box, at lower rate limits; `export CONTEXT7_API_KEY=<key>` before launching your
  client for higher limits. Claude Code prompts for approval the first time this project's
  `.mcp.json` loads — decline it if you'd rather not use it.
- **Tooling**: `justfile` (`just check`/`fmt`/`test`/`db-ensure`/…), `.lefthook.yml`
  (pre-commit secret scan, formatting, conventional commits — subject/body capped at 72
  chars via `bin/check-commit-message`, since `cog.toml` has no config surface for that),
  `bin/bootstrap` (installs the whole toolchain), `bin/doctor` (checks it's all still
  installed, current, and wired up — run it any time, especially after `bootstrap`),
  `docker/Dockerfile` (parameterized by `--build-arg SERVICE=`), `.github/workflows/ci.yml`.

## Not included

- **No shared service framework.** Every service is `main` → `router(pool)` → `feature/`,
  built directly on `axum`/`sqlx`/`tokio`. Add shared infrastructure as your services
  actually need it rather than adopting one up front.
- `bin/deploy` / `bin/deploy_all` are stubs — deploy mechanics are specific to your
  infrastructure. Fill them in once you have a target.
- No auth. Add whatever auth this project needs; there's no framework assumption to work
  around.

## Getting started

```sh
./bin/bootstrap       # installs rustup toolchain, just, lefthook, taplo, typos, etc.
./bin/doctor          # confirms everything installed cleanly and is up to date
just check            # format check, clippy, tests, typos, cargo-deny
```

See `AGENTS.md` for the working rules and workspace conventions this project follows.

## Testing

Integration tests and doc tests run against a real Postgres, not a mock — `just
db-ensure` starts (or reuses) a local `sqlx-test-pg` Docker container and waits until
it's accepting connections, exporting the `DATABASE_URL` tests read. `just check`
already calls it for you; reach for these directly only when iterating on tests in
isolation:

```sh
just db-ensure          # start/reuse local Postgres, wait until ready (needs Docker)
just test               # full suite: cargo nextest run --all-features
just test-unit          # library unit tests only — fast, no Docker required
just test-db            # integration tests only — needs Docker running
just test-crate NAME    # tests for one crate/service
just pg-local-stop       # stop the local Postgres container when you're done
```

## License

Licensed under {{LICENSE}}; see [LICENSE](LICENSE).
