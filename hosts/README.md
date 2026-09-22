# `hosts/`

A **host** is a binary that composes one or more services' `router(pool)` functions into
a single deployable process. It exists purely to save cost on **GCP Cloud Run**: every
always-on Cloud Run service is billed compute even at zero traffic, so a handful of
low-traffic services bundled into one host means one always-warm container instead of
several.

Composition buys nothing functional — a host is wiring only, no domain logic. Every
service also stands alone (`cargo run -p <service>`), and nothing about a service assumes
it's running inside a host. Reach for a host only when Cloud Run cost, not code
structure, is the driver.

## Shape

`hosts/<name>/src/main.rs` builds a pool per mounted service (each service still owns its
own schema — nothing stops them sharing one physical database for a small deployment),
calls each service's `router(pool)`, and `.nest("/prefix", ...)`s them together under one
`axum::serve`.

Auto-discovered via `hosts/*` in the root `Cargo.toml`; builds through the same
`docker/Dockerfile` as any service (`--build-arg SERVICE=<host-name>`).

## Adding a host

Only when you actually have two or more low-traffic services worth bundling for Cloud Run
cost. Scaffold `hosts/<name>/`, add its path to root `[workspace.dependencies]`, and add
the services it mounts as dependencies.
