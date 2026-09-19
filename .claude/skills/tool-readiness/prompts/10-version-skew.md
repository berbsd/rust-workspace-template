# Version Pinning & Skew

Find tools whose version is unpinned or differs between a developer's machine, another developer's machine, and CI — where the same input produces different output.

## Why

A gate is only reproducible if everyone runs the same version. Skew produces the most demoralising failure mode in tooling: a commit that passes locally and fails in CI, or a formatter that rewrites files another developer just formatted, generating diff churn nobody authored.

Formatters are the worst case because their output *is* the contract. A spell checker gaining dictionary entries between versions turns a passing repo failing; a linter's new default rule does the same. Neither is a bug — both are unmanaged upgrades.

A version is also part of a finding. A report saying "the scanner missed this" without a version is not reproducible, which is why check #1 records `betterleaks config check` output verbatim — the rule count is a function of the tool version.

## Scope

- Every hook binary and its version
- CI workflow tool installation steps and any `version:` inputs
- `mise.toml`, `.tool-versions`, `Brewfile`, `package.json` `devDependencies` and `packageManager`
- `npx` / `pnpm exec` invocations without a pinned version

## Patterns to flag

### 1. Local vs CI version mismatch

```bash
for b in lefthook betterleaks gitleaks typos cog; do
  command -v "$b" >/dev/null && printf '%-14s %s\n' "$b" "$("$b" --version 2>&1 | head -1)"
done
grep -rhnE 'version:|@v?[0-9]+\.[0-9]+' .github/workflows/*.yml | grep -iE 'gitleaks|typos|lefthook'
```

Any tool whose CI version is `latest`, or unstated, will drift away from local. Pin both to the same value.

### 2. Unpinned `npx`

```bash
grep -nE 'npx [a-z@]|pnpm (dlx|exec)' lefthook.yml package.json 2>/dev/null
```

`npx squawk` resolves to whatever is installed, or fetches the newest from the network when nothing is. Pin the version in `devDependencies` and invoke through it.

### 3. Version-sensitive behaviour, unrecorded

For any tool whose *ruleset* is versioned — a secret scanner's rule count, a spell checker's dictionary, a linter's default set — record the expected value in the config header so a change is visible:

```
# Verify after any edit — expect 417, never 0:
#   betterleaks config check -c .gitleaks.toml
```

A count that moves after an upgrade is then a deliberate review rather than a surprise.

### 4. Version skew across sibling repos

Check #9's matrix, applied to versions. Two repos on different formatter majors will disagree about the same file — most painfully in shared config or copied source.

```bash
for r in $(ls ~/repos/); do
  ( cd ~/repos/$r 2>/dev/null && printf '%-16s typos=%s\n' "$r" "$(typos --version 2>&1)" )
done
```

### 5. Upgrade with no re-verification

A tool upgrade invalidates check #1 and check #3. After bumping any version: re-run the config check, re-run the canary, record both.

## Fixing

1. Pin every tool to an exact version in whatever the repo already uses (`mise.toml`, `Brewfile`, `devDependencies`) — do not introduce a new mechanism.
2. Make CI install that same pinned version explicitly, never `latest`.
3. Record version-sensitive expectations (rule counts) in config headers.
4. On any upgrade, re-run checks #1 and #3 and record the results in the upgrade commit message.

## Verification

```bash
# The versions a gate actually ran with — put these in the report
lefthook version; betterleaks version 2>&1 | head -1; typos --version; cog --version
```

Compare against CI's installed versions from a recent run's logs. They must match.

## Report format

| Tool | Local | CI | Pinned where | Ruleset expectation |
|---|---|---|---|---|
| betterleaks | 8.30.1 | `latest` ⚠ | nowhere | 417 rules |
| typos | 1.50.1 | 1.50.1 | `Brewfile` | n/a |

Flag every `latest`, every blank pin, and every local/CI mismatch.
