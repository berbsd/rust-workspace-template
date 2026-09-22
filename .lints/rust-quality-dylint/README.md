# rust-quality dylint lints

Custom [`dylint`](https://github.com/trailofbits/dylint) lints for `rust-quality`
checks that need real type/trait information — the kind of thing `ast-grep`
(see `.ast-grep/` at the repo root) cannot see because it only matches source
syntax. See `docs/specs/2026-09-21-rust-quality-mechanization-design.md` for
why these checks are split across the two tools this way.

## Why this is a separate Cargo workspace

Every crate in here compiles as a `cdylib` against `rustc`'s **unstable**
internal APIs (`rustc_hir`, `rustc_middle`, `rustc_lint`, ...), which exist
only under a nightly toolchain with the `rustc-dev`/`rust-src`/
`llvm-tools-preview` components — a completely different, incompatible
toolchain from the root workspace's `rust-toolchain.toml` (a stable release
with none of those components). This directory is **never** added to the root
`[workspace.members]` glob; if it were, `cargo check --workspace` at the repo
root would try to build it with the wrong toolchain and fail outright,
independent of any lint bug. Verify this stayed true after any change here:

```sh
cd /path/to/repo/root && cargo metadata --no-deps --format-version 1 \
  | python3 -c "import json,sys; print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"
# must list only common-types / example / example-host / any real service —
# never a crate from this directory.
```

## Install

`bin/bootstrap` installs `cargo-dylint`/`dylint-link` — project-local, at
`./.cargo/bin/`, not the global `~/.cargo/bin` — so this workspace's dylint
version stays scoped to this checkout rather than shared machine-wide.
`.envrc` adds `./.cargo/bin` to `PATH`. To install by hand instead:

```sh
cargo install cargo-dylint dylint-link --locked --root ../../.cargo
```

`rust-toolchain.toml` in this directory pins the exact nightly these crates
were built against (see its comment for what breaks the pin). Install it and
its components once:

```sh
rustup toolchain install nightly-2026-05-28 \
  --component rustc-dev --component rust-src --component llvm-tools-preview
```

## A gotcha: `RUSTUP_TOOLCHAIN` silently overrides this directory's pin

If your shell (direnv, mise, or similar) exports `RUSTUP_TOOLCHAIN` for the
root workspace's toolchain — this repo's tooling does — that environment
variable **wins over `rust-toolchain.toml` for a bare `cargo test`/`cargo
build`** run inside this directory. You will not get a clear error: `cargo
test` here will instead fail with `can't find crate for rustc_driver` (or
similar), because it silently tried to build against the wrong toolchain.
Verified empirically while building the first lint in this directory — this
is exactly the "same tool, different repo, silently does nothing/fails oddly"
failure mode the `tool-readiness` skill exists to catch.

Fix: always pass the toolchain explicitly when developing a lint directly
(`cargo dylint --lib <name>`, run below, is unaffected by this — only the
`cargo test` dev loop needs it):

```sh
cargo +nightly-2026-05-28 test
```

## Running

**Run a lint's own test suite** (from inside the lint crate, e.g.
`swallowed_errors/`):

```sh
cargo +nightly-2026-05-28 test
```

**Check the real workspace** (from the repo root — this is the exact command
`lefthook.yml`'s `rust-quality-dylint` pre-commit command runs; the matching
`.github/workflows/ci.yml` step exists but is currently disabled with
`if: false` — see that file):

```sh
cargo dylint --all --path .lints/rust-quality-dylint --pattern '*'
```

(The `*` glob also matches this directory's non-crate entries —
`.cargo`/`target` — producing a harmless `Found no packages in ...` warning
per entry. Cosmetic; exit code and lint results are unaffected. Narrow the
pattern to specific crate names if the noise becomes annoying.)

This deliberately does **not** require adding `[workspace.metadata.dylint]`
to the root `Cargo.toml` — `--path`/`--pattern` point `cargo dylint` at this
sub-workspace's libraries from outside it, so the root manifest stays
untouched until a later wiring pass decides that's worth it.

**Check only this sub-workspace's own source** (a much faster inner loop
while iterating on a lint, since it only checks the lint crates themselves,
not the real target workspace):

```sh
cd .lints/rust-quality-dylint && cargo dylint --lib <name>
```

## A gotcha: `declare_late_lint!`'s level must be `Deny`, not `Warn`

Since this is wired into `lefthook.yml` as a commit gate, every lint here
needs `cargo dylint` to actually **fail** (nonzero exit) when it fires — and
`cargo check` only fails on an error-level diagnostic. A lint declared
`Warn` prints its message but the check still exits 0, so a `Warn`-level
lint here is silent decoration, not a gate. Verified the hard way: the first
version of `swallowed_errors` was `Warn`, passed `lefthook run pre-commit`
against a file that should have failed it, and only got caught by manually
diffing a direct `cargo dylint` invocation's exit code against lefthook's.
Declare every lint here `Deny`.

## Zero-panic policy

`.lints/rust-quality-dylint/Cargo.toml`'s `[workspace.lints.clippy]` denies
`unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented`/`unreachable`
(warns on `indexing_slicing`), mirroring the root workspace's own zero-panic
block — every lint crate here needs `[lints] workspace = true` to inherit it
(see `swallowed_errors/Cargo.toml`). This matters more here than in an
ordinary crate: a panic inside a lint pass crashes `cargo check`/`cargo
dylint` for whichever workspace is being linted, not just this crate's own
build. Verified these actually deny by temporarily adding a `.unwrap()` call
and confirming `cargo +nightly-2026-05-28 clippy` rejects it.

**Not currently enforced automatically** — `lefthook.yml`'s
`rust-quality-dylint` command runs `cargo dylint`, which is a plain `cargo
check` with the custom lint loaded, not `cargo clippy`. These `clippy::`
lints only fire when clippy actually runs against this sub-workspace, which
nothing does yet. Run `cargo +nightly-2026-05-28 clippy` by hand inside a
lint crate to check.

## Updating a `.stderr` fixture ("blessing")

There is no `--bless` flag for the `#[test] fn ui()` harness (unlike
`ast-grep test --update-all` on the other side of this mechanization effort).
When a lint's expected output changes, `cargo +nightly-... test` prints
`Actual stderr saved to <tmp path>` on failure — copy that file over the
crate's `ui/<name>.stderr` and **read the diff** before committing; never
blind-accept it.

```sh
cargo +nightly-2026-05-28 test 2>&1 | grep 'Actual stderr saved to'
cp <path from above> ui/main.stderr
cargo +nightly-2026-05-28 test   # confirm it now passes
```

## Why no `clippy_utils` dependency

`cargo dylint new` scaffolds a `clippy_utils` git dependency pinned to a
specific commit by default. That commit's pinned rev did not compile against
this directory's pinned nightly (`rustc_ast` had moved on) — a real instance
of the version-skew this whole scaffold exists to isolate against, one level
down. None of the lints here currently need `clippy_utils`'s helpers (e.g.
`is_type_diagnostic_item`) badly enough to chase down a compatible rev — each
implements the small amount of `rustc_middle::ty` inspection it needs
directly (see `swallowed_errors/src/lib.rs`'s `is_result_type`). Revisit if a
future lint's needs make that no longer true.

## Current lints

| Crate | Check | What it does | What it does NOT catch |
|---|---|---|---|
| `swallowed_errors` | #17, pattern 1 | `let _ = <expr>` where `<expr>`'s type is `Result<_, _>`, with no type ascription | The type-ascription escape hatch (`let _: Result<T, E> = expr`) also requires an adjacent comment per the full check — this lint does not verify a comment is present. Patterns 2-9 of check #17 (unconsumed `.ok()`/`.err()`, `.map_err(\|_\| ..)`, empty `Err(_)` arms, `if let Ok` with no `else`, `.unwrap_or` on `Result`, dropped `tokio::spawn` handles, unpropagated `error!`/`warn!`) are not implemented — see `.claude/skills/rust-quality/prompts/17-swallowed-errors.md` for those, still LLM-judged. |
