# {{PROJECT_NAME}}

Rust 2024 microservices workspace: a `justfile`-driven, lint-heavy, feature-first-service
setup generated from [rust-workspace-template](https://github.com/berbsd/rust-workspace-template).

## What's here

- **`crates/common-types/`** — the one shared library: the API error envelope
  (`ApiErrorBody` + `ApiErrorMapping` + `impl_api_error_response!`) and keyset pagination
  (`PaginatedResponse`, `Cursor`, `KeysetCursor`). Deliberately minimal — no auth types, no
  typed ids, no validation framework. Add to it only once two services genuinely need the
  same code.
{% if keep_example -%}
- **`services/example/`** — a real, self-contained CRUD service (`feature/widget`) built
  directly on `axum`/`sqlx`/`tokio` — no shared bootstrap or middleware framework to depend
  on. `main.rs` → `lib.rs`'s `router(pool)` → `feature/widget/{model,handler,repository,error}.rs`
  is the whole shape — read it for the wire-level conventions (handler → repository, the
  error envelope, garde validation, keyset pagination). Scaffold a new service or feature
  with the `create-rust-service` skill rather than copying this file layout; it generates a
  richer feature-first shape (`config.rs`, `module.rs`, per-feature `domain`/`port`/
  `service`/`adapter`). 10 integration tests exercise `services/example` end-to-end against
  a real Postgres.
- **`hosts/example-host/`** — a binary that nests `example::router(pool)` under a path
  prefix, demonstrating how multiple services would compose into one deployable process to
  save on always-warm compute cost. Every service also stands alone — a host is optional.
{% else -%}
- **`services/`** / **`hosts/`** — empty until you add a first service or host with the
  `create-rust-service` skill; see `AGENTS.md` for the conventions each one follows.
{% endif -%}
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
- **Tooling**: `justfile` (`just check`/`fmt`/`test`/`db-ensure`/…), `lefthook.yml`
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
- No auth.{% if keep_example %} `services/example` has no bearer-token or session layer —
  every route is public.{% endif %} Add whatever auth this project needs; there's no
  framework assumption to work around.
- `jobs/` and `workers/` directories — created on demand; see `AGENTS.md`.

## Getting started

```sh
./bin/bootstrap       # installs rustup toolchain, just, lefthook, taplo, typos, etc.
./bin/doctor          # confirms everything installed cleanly and is up to date
just check            # format check, clippy, tests, typos, cargo-deny
{% if keep_example -%}
just run example       # run the example service locally
{% endif -%}
```

See `AGENTS.md` for the working rules and workspace conventions this project follows.

## License

Licensed under {{LICENSE}}; see [LICENSE](LICENSE).
