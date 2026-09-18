# Cross-Repo Consistency

Compare the same tool across every repo. Find tools invoked differently, configured differently, named differently, or missing entirely — and propagate each fix to all of them.

## Why

Knowledge does not travel between repos on its own. A trap documented carefully in one repo's config header stays there while a sibling repo walks straight into it.

Verified on 2026-09-09 across `infra`, `api` and `web`:

- **`infra` and `api` both loaded 417 gitleaks rules. `web` loaded 0.** The `[extend] useDefault = true` requirement was documented at length in `infra`'s own `.gitleaks.toml` header and in its CLAUDE.md. `web` had never received it, and its secret gate had been inert since the config was authored.
- **`api` knew that `typos {staged_files}` bypasses `extend-exclude` and mirrored its excludes into lefthook.** `web` passed `{staged_files}` too, relying on a `_typos.toml` that therefore did nothing.
- **`web` used `gitleaks protect --staged` with no `--redact`;** the other two used `betterleaks git . --staged --no-banner --redact`. Same job, three differences: tool, subcommand form, and whether a caught secret is printed in cleartext to the terminal and any log capturing the failed commit.

None of these were disagreements about policy. All three were one repo not having heard.

## Scope

Every sibling repo. Establish the list first rather than assuming three:

```bash
for d in ~/repos/*/; do
  [ -d "$d/.git" ] && echo "$(basename $d): $(ls $d | grep -cE 'lefthook|package.json|Cargo.toml')"
done
```

Include repos with **no** hook manager — a missing config is the loudest inconsistency and the easiest to overlook.

## Patterns to flag

### 1. The readiness matrix

Build it mechanically. This is the check's core artefact:

```bash
for r in infra api web; do
  d=~/repos/$r; [ -d "$d" ] || continue
  echo "── $r ──"
  ( cd "$d"
    printf "  gitleaks rules: "; betterleaks config check -c .gitleaks.toml 2>&1 | grep -oE "[0-9]+ rules" || echo "NO CONFIG"
    printf "  typos:          "; typos >/dev/null 2>&1 && echo "clean" || echo "FINDINGS"
    printf "  secret scan:    "; betterleaks git . --no-banner --redact 2>&1 | grep -oiE "no leaks found|leaks found: [0-9]+"
    printf "  hook invocations:\n"; grep -hoE '^\s+run:.*' lefthook.yml 2>/dev/null | sed 's/^\s*run:\s*/    /' )
done
```

Any row that differs is a finding until justified.

### 2. Same tool, different invocation

Diff the actual command strings. Legitimate differences exist (`bin/validate` is infra-specific; `cargo` commands are api-specific), but the *shared* tools — secret scanner, spell checker, commit-message linter — should be byte-identical.

### 3. Same tool, different config filename

`api` uses `.typos.toml`; `web` uses `_typos.toml`. Both are valid names, so this is cosmetic — but record it, because it makes the next mechanical sweep miss a file. Prefer one name across repos when the churn is free; do not rename purely for tidiness if history cost is real.

### 4. A trap documented in one repo and nowhere else

The highest-value pattern here. For each repo, read the *comments* in its tool configs and hook file, and ask whether each warning applies to the siblings.

```bash
grep -hnE '^\s*#.*(NEVER|never|⚠|WARNING|must|silently)' \
  ~/repos/*/.gitleaks.toml ~/repos/*/lefthook.yml 2>/dev/null | sort -u
```

Every such warning is a lesson someone paid for. If it is true in one repo and the mechanism exists in another, it belongs in both.

### 5. A gate present in one repo and absent in another

Not every tool suits every repo — `rustfmt` has no place in a Vue app. But a **secret scanner, a spell checker and a commit-message linter apply everywhere.** Absence of one of those is a finding, not a choice.

## Fixing

1. Build the matrix and record it before changing anything.
2. For each divergence, decide: converge, or document why this repo differs. Both are acceptable; silence is not.
3. Apply the fix to **every** repo in the same pass. A fix applied to one repo recreates the original problem.
4. Copy the *comment* along with the config — the reasoning is the durable part.
5. Re-run the matrix and record the after state.

## Verification

Re-run the matrix block above. Every shared row identical, or each difference carrying a comment naming the reason.

Then, per repo, the canary from check #3 — consistency of configuration is not evidence of function.

## Report format

| | infra | api | web |
|---|---|---|---|
| Secret scanner | `betterleaks git . --staged --redact` | same | **`gitleaks protect`, no `--redact`** |
| Rules loaded | 417 | 417 | **0** |
| typos | yes | yes + `exclude:` mirror | yes, **excludes inert** |
| commit-msg | `cog verify` | `cog verify` | `cog verify` |
| Concurrency | `parallel: true` | unset | `parallel: false` |

Bold every cell that differs. End with the after-matrix and the per-repo canary results.
