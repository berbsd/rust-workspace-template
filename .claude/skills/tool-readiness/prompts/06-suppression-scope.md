# Suppression Scope

Find suppressions that silence more than the thing they were written for — above all, allowlists scoped by path rather than by content.

## Why

The path-scoped suppression is the single most damaging pattern in this category, because it looks like precision and behaves like a blindfold.

This template's own `.gitleaks.toml` documents the mechanism in its own header: betterleaks compiles allowlist `paths` into its **prefilter** — the same skip expression that drops `node_modules` and binaries — so matching files are removed from the corpus *before any rule runs*, and the allowlist's own `regexes` are never consulted. `matchCondition = "AND"` does not change this. The result is a scan with zero applicable rules over those files, reporting "no leaks found" and exiting 0. That's why the header states the rule flatly: never give an allowlist a `paths` key; scope by content instead.

The consequence is not just today's blindness. A path suppression also silences **every secret added to that file in future**, by anyone, forever.

The content-scoped alternative from the same file, anchored to the whole secret so only the exact literal is ignored:

```toml
[[allowlists]]
description = "Documentation placeholder credential in postgres:// examples"
regexTarget = "secret"
regexes = ['''^pass$''']
```

A real password containing `pass` still fires.

## Scope

- `paths` / `files` / `extend-exclude` / `ignore` keys in every tool config
- `.gitleaksignore`, `.trivyignore`, baseline files
- Inline suppressions: `gitleaks:allow`, `# noqa`, `// eslint-disable`, `#[allow(...)]`, `# nosec`, `#tfsec:ignore`, `#checkov:skip`

## Patterns to flag

### 1. Path-scoped suppression of a content problem

```bash
grep -nE '^\s*paths\s*=|extend-exclude|^\s*exclude:' .gitleaks.toml _typos.toml .typos.toml 2>/dev/null
```

For each, ask: **is this file excluded because of what it *is*, or because of what it currently *contains*?**

- Legitimate: generated output, vendored third-party text, binary fixtures, lockfiles, a language the tool has no dictionary for. The file will never be a place you write a secret by hand.
- Illegitimate: any exclusion added to make a specific finding go away. Re-scope it by content.

### 2. Suppression with no reason and no removal condition

Every suppression needs both: why it is safe, and what would make it removable. Without the second it is permanent by default.

```bash
grep -B2 -nE 'paths|regexes|extend-exclude' .gitleaks.toml | grep -c description
```

### 3. A fixed list that will rot

Listing exact strings is right for stable inputs (test fixtures) and wrong for volatile ones (UI copy). A list of today's user-facing labels becomes stale on the next wording change, leaving entries that suppress nothing and hide nothing. For volatile classes, scope by *shape* and document the limitation explicitly — including what a real secret matching that shape would look like.

### 4. Broadening under pressure

The tell is a suppression added in the same change as a failing run. Check history:

```bash
git log -p --follow .gitleaks.toml | grep -E '^\+.*(paths|extend-exclude)' | head
```

Never widen a suppression to make a run green. Triage the finding; escalate if inconvenient.

### 5. Suppressions that swallow the canary

After every suppression change, re-run check #3's canary. This is the only mechanical defence against an exclusion that turned out broader than intended.

## Fixing

1. Re-scope each content suppression from `paths` to an anchored content regex (`^…$`).
2. Give every remaining suppression a `description` naming the reason and the removal condition.
3. Re-run the canary and record it.
4. Re-run the full scan and record the finding count — it should drop to zero *with rules still loaded* (check #1), not because the corpus shrank.

## Verification

```bash
betterleaks config check -c .gitleaks.toml     # rules still loaded — expect 417
betterleaks git . --no-banner --redact         # expect "no leaks found"
betterleaks dir /tmp/canary -c .gitleaks.toml  # canary must STILL fire
```

All three, in that order. The third is the one people skip and the one that catches an over-broad suppression.

## Report format

| Suppression | Scope | Why | Removal condition | Canary still fires? |
|---|---|---|---|---|
| `^pass$` | content | placeholder in `postgres://` docs, fixed in history | never (history immutable) | yes |
| `**/locales/fr.json` | path | no French dictionary in typos | if typos gains French | yes |
| `src/**` | **path** | — | — | **NO — blindfold, re-scope** |
