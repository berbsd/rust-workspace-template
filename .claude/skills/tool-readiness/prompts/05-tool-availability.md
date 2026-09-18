# Tool Availability & Fail-Closed

Find gates that depend on a binary a fresh checkout does not have, and determine whether a missing tool fails the commit or waves it through.

## Why

A hook that invokes a missing binary either fails every commit for a reason nobody understands, or — far worse — is skipped and reports success. Which one it does is rarely chosen deliberately.

The install path is the repo's setup script, and the distinction matters: in `infra`, `bin/bootstrap` provisions cloud prerequisites while `bin/preflight` installs local dev tooling — `betterleaks` and `lefthook` belong to the latter. A tool wired into a hook but absent from the setup script works on the machine that added it and nowhere else.

## Scope

- Every binary named in `lefthook.yml`, CI workflows, `justfile` / `Makefile` / npm scripts
- The repo's setup script(s) — `bin/preflight`, `bin/bootstrap`, `postinstall`, `mise.toml`, `Brewfile`
- `.github/workflows/*` — which tools CI installs explicitly

## Patterns to flag

### 1. A hook binary the setup script does not install

```bash
# every binary a hook invokes
grep -hoE '^\s+run:\s*\S+' lefthook.yml | awk '{print $2}' | sort -u
# is each installed by setup?
#
# Two traps this snippet works around, both hit while writing this check:
#   - a nonexistent file in grep's list makes it exit non-zero and mask a real
#     match, reporting every tool as absent;
#   - zsh does not word-split an unquoted variable, so a newline-joined file
#     list collapses into one bogus filename. Run tool checks under `bash`.
# Repo-local scripts (bin/validate, ./scripts/...) are not installable tools —
# skip them, or every sweep reports the repo's own scripts as missing.
for b in $(grep -hoE '^\s+run:\s*\S+' lefthook.yml | awk '{print $2}' \
           | grep -vE '^(\./|bin/|scripts/)' | xargs -n1 basename | sort -u); do
  found=""
  for f in bin/preflight bin/bootstrap Brewfile mise.toml .tool-versions package.json; do
    [ -f "$f" ] && grep -q -- "$b" "$f" && { found=$f; break; }
  done
  [ -n "$found" ] && echo "  in setup ($found): $b" || echo "  NOT in setup: $b"
done
```

Every hook binary must be installed by something a new contributor runs once. A `README` instruction is not that thing.

### 2. Missing tool — verify, do not assume, which way it fails

Determine the actual behaviour with the manager on PATH and the tool removed:

```bash
rm -rf /tmp/nopath && mkdir -p /tmp/nopath
for b in lefthook git; do ln -s "$(command -v $b)" /tmp/nopath/$b; done
env PATH=/tmp/nopath:/usr/bin:/bin lefthook run pre-commit --command <name>; echo "exit=$?"
```

Strip the manager from PATH too and you measure nothing — the first attempt at
this returned `env: lefthook: No such file or directory`, exit 127, which says
only that the test was wrong.

**Measured for lefthook 2.1.12 on 2026-09-09: it fails CLOSED.** A missing
binary produces `sh: <tool>: command not found`, exit status 127 from the
command, and lefthook exits non-zero — the commit is blocked.

That result reorders this check's priorities, and the reordering is the point:

- **A missing *binary* is not a silent failure** under lefthook. It is loud,
  and safe. What it costs is a bad message — `command not found` does not tell
  a new contributor to run `brew install typos-cli`.
- **A missing *hook* is the silent one.** If `lefthook install` was never run,
  `.git/hooks/pre-commit` does not exist, nothing executes, and every commit
  passes. This is rung 3 of check #3's ladder failing while rungs 1 and 2 pass.

So the setup script's load-bearing job is **installing the hooks**, and its
tool-checking is ergonomics on top. Verify a repo has *some* mechanism that runs
`lefthook install`; do not assume a tool-version table implies it.

Confirm the behaviour for whatever manager the repo uses rather than carrying
this result across — a `.husky` script or a hand-written hook may well differ.

### 3. A tool present only via a package manager's local install

`npx <tool>` and `pnpm exec <tool>` resolve from `node_modules`, so they work only after an install and can silently fetch a *different* version from the network when absent. Pin them (see check #10) and ensure the install step precedes the hook.

### 4. Setup script that does not verify

A setup script that installs without checking is a setup script that half-succeeds. It should end by asserting every tool resolves and printing versions:

```bash
for b in lefthook betterleaks typos cog; do
  command -v "$b" >/dev/null && printf '  %-14s %s\n' "$b" "$("$b" --version 2>&1 | head -1)" \
    || { echo "MISSING: $b"; exit 1; }
done
```

## Fixing

1. Ensure the repo has a mechanism that runs `lefthook install`, wired into the
   onboarding step it already has — `bin/preflight`, `bin/bootstrap`, or
   `package.json`'s `prepare` for a pnpm/npm repo. Do not introduce a new script
   nobody will run. This is the fix that matters; the rest is ergonomics.
2. Add every hook binary to the setup script.
3. Make the setup script verify and print versions at the end.
4. Decide fail-open vs fail-closed **explicitly** for each tool, and comment the decision. Default to closed.
5. Ensure CI installs the same set — a gate that exists only locally is not a gate.

## Verification

```bash
bin/preflight                                  # or the repo's equivalent
PATH=/usr/bin:/bin lefthook run pre-commit     # must fail loudly, not pass
```

Simulating absence is the whole point; do not skip it because the tool is installed on your machine.

## Report format

| Binary | In setup script? | CI installs? | Missing-tool behaviour | Verdict |
|---|---|---|---|---|
| betterleaks | yes (`bin/preflight`) | yes | fails closed | OK |
| typos | **no** | no | **fails open** | add to preflight; guard the hook |
