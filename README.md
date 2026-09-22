# rust-workspace-template

Rust 2024 microservices workspace template: a `justfile`-driven, lint-heavy,
feature-first-service starter, extracted from a production platform and stripped down to
its generic bones — no shared service framework, no product-specific code, no bundled
example, just the tooling. Scaffold your first service with the `create-rust-service`
skill once generated.

Most files here (`Cargo.toml`, `AGENTS.md`, …) are themselves templates — the `{{...}}`
placeholders throughout get filled in by
[`cargo generate`](https://github.com/cargo-generate/cargo-generate) when a new project
is generated from this repo (see `cargo-generate.toml`). This README is the one
exception: it documents the template itself for people browsing this repo, and is
swapped out for `README.project.md` — written for the generated project, not for this
one — during generation (see `hooks/post.rhai`).

## What's here

- **`crates/common-types/`** — the one shared library: the API error envelope
  (`ApiErrorBody` + `ApiErrorMapping` + `impl_api_error_response!`) and keyset pagination
  (`PaginatedResponse`, `Cursor`, `KeysetCursor`). Deliberately minimal — no auth types, no
  typed ids, no validation framework. Add to it only once two services genuinely need the
  same code.
- **`services/`**, **`hosts/`**, **`jobs/`**, **`workers/`** — empty (each just carries a
  `.gitkeep` so their `Cargo.toml` glob has something to match — see that file's header
  comment). Scaffold a first service or feature with the `create-rust-service` skill,
  which generates a feature-first shape (`main.rs` → `lib.rs`'s `router(pool)` →
  per-feature `config.rs`/`module.rs`/`domain`/`port`/`service`/`adapter`) rather than
  hand-copying a layout — see `AGENTS.md` for the full shape and conventions.
- **`.claude/skills/`** — engineering-discipline skills: `create-rust-service`
  (scaffolds a new service or feature slice), `rust-quality` (including handler hygiene,
  check #34), `rust-documenter` (rustdoc, and OpenAPI/utoipa/Scalar annotations if the
  service adds them — see its `references/openapi-annotations.md`),
  `rust-tracing-instrument`, `sql-analyzer`, `tool-readiness`, `validate-implementation`.
  Some `rust-quality` checks describe patterns this template doesn't have yet (an outbox,
  a JWT revocation cache) — skip those until the workspace grows them.
- **`.mcp.json`** — registers the [Context7](https://context7.com) MCP server, so Claude
  Code (or any other MCP-compatible client) gets live, version-accurate library/framework
  documentation lookups natively, without shelling out to a CLI. Works unauthenticated out
  of the box, at lower rate limits; `export CONTEXT7_API_KEY=<key>` before launching your
  client for higher limits. Claude Code prompts for approval the first time a project's
  `.mcp.json` loads — decline it if you'd rather not use it.
- **Tooling**: `justfile` (`just check`/`fmt`/`test`/`db-ensure`/…), `.lefthook.yml`
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
- No auth. Add whatever auth your project needs; there's no framework assumption to work
  around.

## Getting started

```sh
./bin/bootstrap       # installs rustup toolchain, just, lefthook, taplo, typos, etc.
./bin/doctor          # confirms everything installed cleanly and is up to date
just check            # format check, clippy, tests, typos, cargo-deny
```

See `AGENTS.md` for the working rules and workspace conventions this template enforces.

## License

Licensed under the Apache License, Version 2.0 (the "License"); see [LICENSE](LICENSE).
This covers the template's own content (scaffolding, skills, tooling). A project
generated from this template picks its own license when scaffolded — see the `license`
field in its `Cargo.toml`.
