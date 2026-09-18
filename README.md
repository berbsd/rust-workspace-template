# {{PROJECT_NAME}}

Rust 2024 microservices workspace template: a `justfile`-driven, lint-heavy,
feature-first-service starter, extracted from a production platform and stripped down to
its generic bones — no shared service framework, no product-specific code, just the
tooling and one real, working example.

This file is itself a template — the `{{...}}` placeholders throughout the workspace get
filled in when a new project is generated from it.

## What's here

- **`crates/common-types/`** — the one shared library: the API error envelope
  (`ApiErrorBody` + `ApiErrorMapping` + `impl_api_error_response!`) and keyset pagination
  (`PaginatedResponse`, `Cursor`, `KeysetCursor`). Deliberately minimal — no auth types, no
  typed ids, no validation framework. Add to it only once two services genuinely need the
  same code.
- **`services/example/`** — a real, self-contained CRUD service (`feature/widget`) built
  directly on `axum`/`sqlx`/`tokio` — no shared bootstrap or middleware framework to depend
  on. `main.rs` → `lib.rs`'s `router(pool)` → `feature/widget/{model,handler,repository,error}.rs`
  is the whole shape; copy it for your first real feature. 9 integration tests exercise it
  end-to-end against a real Postgres.
- **`hosts/example-host/`** — a binary that nests `example::router(pool)` under a path
  prefix, demonstrating how multiple services would compose into one deployable process to
  save on always-warm compute cost. Every service also stands alone — a host is optional.
- **`.claude/skills/`** — engineering-discipline skills: `rust-quality` (including
  handler hygiene, check #34), `rust-documenter` (rustdoc, and OpenAPI/utoipa/Scalar
  annotations if the service adds them — see its `references/openapi-annotations.md`),
  `rust-tracing-instrument`, `sql-analyzer`, `tool-readiness`, `validate-implementation`.
  Some `rust-quality` checks describe patterns this template doesn't have yet (an outbox,
  a JWT revocation cache) — skip those until the workspace grows them.
- **`.mcp.json`** — registers the [Context7](https://context7.com) MCP server, so Claude
  Code (or any other MCP-compatible client) gets live, version-accurate library/framework
  documentation lookups natively, without shelling out to a CLI. Works unauthenticated out
  of the box, at lower rate limits; `export CONTEXT7_API_KEY=<key>` before launching your
  client for higher limits. Claude Code prompts for approval the first time a project's
  `.mcp.json` loads — decline it if you'd rather not use it.
- **Tooling**: `justfile` (`just check`/`fmt`/`test`/`db-ensure`/…), `lefthook.yml`
  (pre-commit secret scan, formatting, conventional commits — subject/body capped at 72
  chars via `bin/check-commit-message`, since `cog.toml` has no config surface for that),
  `bin/bootstrap` (installs the whole toolchain), `bin/doctor` (checks it's all still
  installed, current, and wired up — run it any time, especially after `bootstrap`),
  `docker/Dockerfile` (parameterized by `--build-arg SERVICE=`), `.github/workflows/ci.yml`.

## Not included

- **No shared service framework.** The source platform this template was extracted from
  has one — a `service-builder`-style crate providing auth/admin/idempotency middleware,
  telemetry, health probes, and a typed-id/JWT/outbox ecosystem around it. Porting that
  whole framework is a bigger, separate undertaking than a starter template needs; this
  template gives you the shape (`main` → `router(pool)` → `feature/`) and lets you add
  shared infrastructure as your own services actually need it.
- `bin/deploy` / `bin/deploy_all` are stubs — deploy mechanics are specific to your
  infrastructure. Fill them in once you have a target.
- No auth. `services/example` has no bearer-token or session layer — every route is
  public. Add whatever auth your project needs; there's no framework assumption to work
  around.
- `jobs/` and `workers/` directories — created on demand; see `CLAUDE.md`.

## Getting started

```sh
./bin/bootstrap       # installs rustup toolchain, just, lefthook, taplo, typos, etc.
./bin/doctor          # confirms everything installed cleanly and is up to date
just check            # format check, clippy, tests, typos, cargo-deny
just run example       # run the example service locally
```

See `CLAUDE.md` for the working rules and workspace conventions this template enforces.

## License

Licensed under the Apache License, Version 2.0 (the "License"); see [LICENSE](LICENSE).
This covers the template's own content (scaffolding, skills, example code). A project
generated from this template picks its own license when scaffolded — see the `license`
field in its `Cargo.toml`.
