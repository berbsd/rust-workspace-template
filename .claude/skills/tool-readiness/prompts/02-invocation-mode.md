# Invocation-Mode Drift

Find tools whose configuration silently stops applying because of *how* they are invoked. The config is correct, the tool is correct, and the two never meet.

## Why

Most scanners behave differently when handed explicit paths than when asked to walk a tree, and the difference is usually undocumented. The tool does not warn. It just uses a different corpus, or ignores half its own config.

This template's own config carries a live example, and it's worth reading as the model case:

- **`typos {staged_files}` ignores `files.extend-exclude`.** `.typos.toml`'s header comment spells this out directly: it mirrors `lefthook.yml`'s per-command `exclude:` lists because passing `{staged_files}` explicitly bypasses `[files] extend-exclude` entirely. This template excludes `.claude/` in *both* places — `.typos.toml` for a bare `typos` run, and `lefthook.yml`'s `typos:` command for the hook — because the skill/prompt files under `.claude/` quote short module-path-like tokens that read as misspelt English words. Drop the exclude from only one of the two files and it silently stops applying under whichever invocation mode that file doesn't cover.
- **`gitleaks dir` and `gitleaks git` scan different things.** `dir` ignores `.gitignore` entirely, so it walks files a repo has deliberately excluded (a local `.env`, a gitignored fixtures directory) and can report "findings" nobody can action, or — worse — skip a config-based allowlist that only applies to the `git` mode. This template's own `.gitleaks.toml` header documents exactly this: "Scan with: `betterleaks git .` ... NOT with: `betterleaks dir .`."

The general shape: **whenever a hook passes `{staged_files}`, assume the tool's own path-based config is off until proven otherwise.**

## Scope

- Every `run:` line in `lefthook.yml` — especially those containing `{staged_files}`
- Every CI invocation of the same tool in `.github/workflows/*`
- Any `Makefile` / `justfile` / `package.json` script that runs the same tool

The bug is a *disagreement* between these, so all three must be read together.

## Patterns to flag

### 1. `{staged_files}` alongside a path-based config

For every hook command that passes explicit paths, and whose config contains a path-scoped key (`extend-exclude`, `ignore`, `exclude`, `paths`), prove which one wins:

```bash
# A: whole tree — config honoured
typos --format brief | wc -l
# B: explicit path, the way the hook invokes it — config may be bypassed
typos <a-file-the-config-excludes> --format brief | head
```

Different results mean the config does not apply under the hook. Mirror the exclusions into the hook manager's own `exclude:` key and **keep the two lists in step** — note in a comment that the config file is what a bare run uses.

### 2. Whole-tree vs index vs working-tree

Three different corpora, routinely confused:

| Mode | Corpus | Honours `.gitignore`? |
|---|---|---|
| `betterleaks git .` | tracked content + history | yes |
| `betterleaks git . --staged` | the index only | n/a |
| `betterleaks dir .` | the filesystem | **no** |

A hook must scan the index — that is what is being committed. A repo sweep must scan tracked content and history. Using `dir` for either produces findings nobody can action and trains people to ignore the tool.

### 3. CI and hook disagreeing

```bash
grep -rhoE '(typos|gitleaks|betterleaks|eslint|prettier)[^|&;]*' .github/workflows/*.yml | sort -u
grep -hoE '^\s+run:.*' lefthook.yml | sed 's/^\s*run:\s*//'
```

If CI scans the whole tree and the hook scans staged files with different exclusions, CI will fail on things the hook passed. That is not defence in depth — it is a gate that only fires after the work is pushed.

### 4. Explicit path defeating a per-directory config

Tools that walk up from each file to find config (`rustfmt` and `.rustfmt.toml`, eslint flat config) behave differently again: passing a path still finds the config, but passing a path *outside* the project may silently pick up a different one. Confirm which file was used.

## Fixing

1. Establish which corpus the hook should scan — for a commit gate, the index.
2. Make the config apply under that invocation. Either stop passing explicit paths, or mirror the exclusions into the hook manager's `exclude:`.
3. Leave a comment at both sites saying they must stay in step, and why.
4. Prove it with the two-sided test: an excluded file must pass, a genuinely bad file must still fail. Both, or the change is unverified.

## Verification

```bash
git add <a-file-the-config-excludes>
lefthook run pre-commit --command <name>     # must PASS

printf 'teh availabe seperate\n' > /tmp/c.md && git add -f /tmp/c.md
lefthook run pre-commit --command <name>     # must FAIL
```

Testing only the pass side proves nothing — an over-broad exclude passes everything.

## Report format

| Tool | Hook invocation | Config key | Applies under hook? | Evidence |
|---|---|---|---|---|
| typos | `typos {staged_files}` | `files.extend-exclude` | **No** | whole-tree 0 findings; explicit path 6 findings in an excluded file |
| betterleaks | `git . --staged` | `[[allowlists]]` | Yes | canary caught, allowlisted string ignored |
