# Mutation Safety

Classify every hook command as read-only or mutating, and verify the concurrency setting matches. Parallelism is safe if and only if nothing writes.

## Why

`parallel: true` is safe **if and only if** every command in the stage is read-only. Two commands that rewrite the same staged files race on the file contents; two that run `git add` race on `.git/index.lock`, which is a single global lock.

This template's own `lefthook.yml` is the case in point: `rustfmt {staged_files}` + `stage_fixed: true` mutates the staged files and re-stages them, and its own comment says exactly why the whole `pre-commit` stage is `piped: true` rather than `parallel: true` — running a mutating command concurrently with anything else would race on file contents and on `.git/index.lock`. A repo whose hook commands are *all* read-only (a linter that only reports, a spell-checker, a secret scanner) can safely run them in parallel; the moment even one command writes back to the working tree or the index, that stops being true for the whole stage.

The sharper version of this same bug: a workspace-wide formatter invoked without file scope (`cargo fmt` instead of `rustfmt {staged_files}`) reformats **every** dirty file in the tree, and `stage_fixed` then stages all of them — a commit meant to touch one file can silently absorb another session's uncommitted work, under a message describing none of it. The fix is file-scoping the formatter, not a concurrency setting.

## Scope

- Every `run:` line, classified read-only or mutating
- `stage_fixed:` — lefthook staging files on the command's behalf
- Any `git add`, `--write`, `--fix`, `-i`, `--in-place` in a hook
- The `parallel:` / `piped:` setting for each stage

## Patterns to flag

### 1. A mutating command in a parallel stage

```bash
grep -nE 'run:.*(--fix|--write|-i |--in-place|git add)|stage_fixed' lefthook.yml
grep -nE '^\s*(parallel|piped):' lefthook.yml
```

Any overlap between a mutating command and `parallel: true` is a finding. There is no safe subset — lefthook does not serialise index access for you.

### 2. A workspace-wide formatter in a staged-files hook

The shape described above. Flag any formatter invoked without file scope (`cargo fmt`, `prettier --write .`, `eslint --fix .`) in a hook that also stages what it changed. Scope it to `{staged_files}`.

### 3. Two formatters over the same files

`eslint --fix` and `prettier --write` on the same glob will fight even sequentially if their rules disagree; in parallel they lose writes outright. Sequential plus a shared config (`eslint-config-prettier`) is the fix.

### 4. `stage_fixed` without understanding the scope

`stage_fixed: true` stages whatever the command changed. Combined with an unscoped command it stages the world. Combined with a scoped one it is exactly right. The setting is not the bug; the pairing is.

### 5. Mutation in an assertion-only location

A directory or stage whose contract is "assert, do not act" should contain nothing that writes. If a repo has a convention like this (a `checks.d/`-style directory of independent verification scripts, or similar), the tell for a violation is the same either way: *a member that always succeeds is doing something other than asserting.* A wrapper script that calls into it may legitimately mutate elsewhere (writing a report, updating a cache) — know which layer is allowed to write, and don't let a hook stage blur the two.

## Fixing

1. Classify every command. Put the classification in a comment beside the concurrency setting — that comment is what stops someone "optimising" it later.
2. Set `parallel: true` only for an all-read-only stage.
3. Scope every formatter to `{staged_files}`.
4. Where two mutating commands touch the same files, order them deterministically and say why.

## Verification

```bash
# Prove a parallel stage is read-only: nothing changes after a run
git status --porcelain > /tmp/before
lefthook run pre-commit >/dev/null 2>&1
git status --porcelain > /tmp/after
diff /tmp/before /tmp/after && echo "read-only confirmed" || echo "MUTATED — parallel unsafe"
```

Run it against a realistic staged change, not a clean tree — a formatter with nothing to fix mutates nothing and proves nothing.

## Report format

| Command | Writes files? | Touches index? | Safe in parallel? |
|---|---|---|---|
| `betterleaks git . --staged` | no | no | yes |
| `typos {staged_files}` | no | no | yes |
| `rustfmt {staged_files}` + `stage_fixed` | **yes** | **yes** | **no** |

Conclude with the stage's setting and whether it matches the table.
