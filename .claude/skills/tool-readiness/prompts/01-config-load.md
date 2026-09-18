# Config Load Verification

Prove each tool loaded a non-empty ruleset. Find configs that replace the built-in rules instead of extending them, that fail to load, or that are never discovered at all.

## Why

"No findings" and "no rules loaded" are the same output. A scanner with zero rules prints a cheerful summary, exits 0, and inspects nothing.

Verified across sibling repos on 2026-09-09:

- **`web/.gitleaks.toml` loaded 0 rules.** It declared an `[allowlist]` but no `[extend] useDefault = true`, so gitleaks *replaced* its 417 built-in rules with the file's own — and the file defined none. `betterleaks config check` reported `OK: 0 rules`. The pre-commit hook had been exiting 0 on every secret since the config was written. `infra` and `api`, with the same tool, reported `OK: 417 rules`.
- **The same config could not be diagnosed by running it.** A scan printed `no leaks found` in 6ms and exited 0 — indistinguishable from a clean repo. Only `config check` separated the two.
- **A legacy key silently became a hard error.** Mixing the deprecated `[allowlist]` with `[[allowlists]]` makes gitleaks refuse to load: `[allowlist] is deprecated, it cannot be used alongside [[allowlists]]`. That one fails loudly, which is the correct behaviour and the contrast worth noticing — most load failures do not.

## Scope

Every tool config in the repo, and the hook line that invokes each one:

- `.gitleaks.toml` / `.betterleaks.toml`
- `.typos.toml` / `_typos.toml`
- `.eslintrc*` / `eslint.config.*`, `.prettierrc*`
- `.rustfmt.toml`, `.clippy.toml`
- `.trivyignore`, `.checkov.yml`, `.tflint.hcl`
- `lefthook.yml` — for which config each command actually points at

## Patterns to flag

### 1. A config that replaces rather than extends

The single highest-value check in this skill. For any tool with built-in rules:

```bash
betterleaks config check -c .gitleaks.toml     # expect "OK: 417 rules", never 0
```

If the count is 0, or far below the sibling repos', the gate is inert. For gitleaks the fix is `[extend] useDefault = true` at the top of the file.

Generalise the idea: every scanner with a default ruleset has a mode that prints what it loaded. Find it and record the number. If a tool has no such mode, treat that as a finding of its own — you cannot verify it, so it must be backed by check #3's canary on every change.

### 2. A config that is never discovered

A config in the wrong place, or under a filename the tool does not look for, is not an error — the tool silently uses defaults.

```bash
# What does the tool think its config is?
typos --help | grep -i config
gitleaks detect --help | grep -i config
```

Confirm the discovered path is the file you edited. Note that some tools accept several names (`.typos.toml`, `_typos.toml`, `typos.toml`) and some prefer one and fall back to another (`betterleaks` reads `.betterleaks.toml` first, then `.gitleaks.toml`).

### 3. Deprecated or mutually exclusive keys

```bash
betterleaks config check -c .gitleaks.toml   # surfaces deprecation errors
```

Singular `[allowlist]` vs plural `[[allowlists]]`, old severity names, renamed fields. Prefer the current spelling everywhere; a config that loads today on a deprecated key is a config that stops loading on the next upgrade.

### 4. A config that parses but means nothing

An exclusion for a path that no longer exists, a rule id that was renamed, a language key for a language the repo dropped. These do not error — they simply never match. Cross-check every referenced path:

```bash
python3 - <<'PY'
import re, pathlib, sys
cfg = pathlib.Path('_typos.toml').read_text()
for p in re.findall(r'"([^"]+)"', cfg):
    if '/' in p or p.endswith(('.json','.ts','.yaml')):
        base = p.split('*')[0].rstrip('/')
        if base and not list(pathlib.Path('.').glob(base + '*')):
            print('stale exclude:', p)
PY
```

## Fixing

1. Add the extend/inherit directive so built-in rules survive.
2. Re-run the config check and record the new count.
3. **Expect the first honest scan to find things.** Enabling 417 rules on a repo that was never scanned surfaced 26 findings in `web`. Triage every one before suppressing any — see check #6 for how to scope the suppression.
4. Add the verification command to the config's own header comment, with the expected number, so the next editor can re-check in one line.

## Verification

```bash
betterleaks config check -c .gitleaks.toml    # expect 417 rules
typos --files | wc -l                          # expect a plausible corpus size, never 0
```

A corpus of 0 files is the same class of bug one level up: the rules loaded, but nothing was fed to them.

## Report format

| Tool | Config | Rules loaded | Corpus | Verdict |
|---|---|---|---|---|
| betterleaks | `.gitleaks.toml` | **0** → 417 | 11.59 MB | was inert since config was authored |
| typos | none | built-in | 201 files | OK |

State the verification command and expected value for each row.
