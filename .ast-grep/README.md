# ast-grep rules

Structural (AST-based) lint rules for patterns `clippy` can't express — workspace
conventions like "no `#[ignore]`" rather than general Rust correctness. See
`docs/specs/2026-09-21-rust-quality-mechanization-design.md` for why these live
here instead of staying LLM-audited prompts in `.claude/skills/rust-quality/`.

## Install

`bin/bootstrap` installs this — project-local, at `./.cargo/bin/ast-grep`
(via `cargo install ast-grep --locked --root ./.cargo`), not the global
`~/.cargo/bin` or a Homebrew formula, so this workspace's tool version stays
scoped to this checkout. `.envrc` adds `./.cargo/bin` to `PATH`. To install
by hand instead:

```sh
cargo install ast-grep --locked --root ./.cargo
```

The CLI binary is `ast-grep` (the short alias `sg` still works but prints a
deprecation warning as of ast-grep 0.45 — this repo's docs/scripts use
`ast-grep`, not `sg`).

## Layout

- `sgconfig.yml` — project config (`ruleDirs`, `testConfigs`). Lives in this
  directory, **not** the repo root, to keep tool config grouped the way
  `.claude/` and `.github/` already are — but `ast-grep` only auto-discovers
  `sgconfig.yml` in the current working directory, so every invocation below
  passes `-c .ast-grep/sgconfig.yml` explicitly. Dropping that flag is the
  most likely way this gate goes silently unwired — it fails loudly
  (`No ast-grep project configuration is found`) rather than passing empty,
  but a copy-pasted lefthook/CI command that forgets it will error, not skip.
- `rules/*.yml` — one rule per file, `id` matching the filename.
- `rule-tests/*-test.yml` + `rule-tests/__snapshots__/` — `valid`/`invalid`
  code snippets per rule, pinned snapshots for the `invalid` cases (ast-grep's
  own test framework — analogous to a dylint lint's `ui/*.stderr`). Never hand
  edit `__snapshots__/`; regenerate with `--update-all` and inspect the diff.

## Running

Wired into `lefthook.yml`'s `ast-grep` pre-commit command (scoped to staged
files) — a real violation blocks the commit. The matching CI step in
`.github/workflows/ci.yml`'s `check` job exists but is currently disabled
(`if: false` — too much CI overhead for a single-developer repo when
lefthook already runs this on every commit; see that file's comment on the
step for how to re-enable). To run it by hand:

```sh
# From the repo root:
ast-grep test -c .ast-grep/sgconfig.yml                       # run the rule test suite
ast-grep scan -c .ast-grep/sgconfig.yml services crates hosts  # scan the real workspace
```

A rule change or addition always re-runs `ast-grep test` first — a rule with
no test proving it fires is exactly the failure mode check #33 itself exists
to catch, applied to tooling instead of test code.

## Current rules

| Rule | Check | What it catches |
|---|---|---|
| `no-skipped-tests.yml` | #33 | `#[ignore]` / `#[ignore = "..."]` anywhere in `.rs` source |
| `no-silent-env-skip.yml` | #33 | `if env::var($X).is_err() { ...return... }` inside a `#[test]`/`#[tokio::test]`/`#[*::test]`-attributed function — scoped to test functions specifically so a legitimate production feature-flag guard isn't flagged |

Not covered by either rule (deliberately — see `33-no-skipped-tests.md`'s "What
this check does NOT flag" and the residual note added to that prompt): a test
module gated behind a `#[cfg(feature = "...")]` that isn't on by default.
Confirming that requires cross-referencing the crate's default-feature set,
which is outside what a structural rule can see — stays a documented residual
for the LLM/human to check.
