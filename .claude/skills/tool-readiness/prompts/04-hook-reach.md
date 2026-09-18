# Hook Installation & Reach

Find gates that are correctly configured and never run — hooks that were never installed, globs too narrow to match the commit, and stages that fire too late to matter.

## Why

A tool can load its rules, apply its config, and fail correctly on a canary, and still never see your commit. Configuration and invocation are separate failures.

Verified in this codebase on 2026-09-09: committing `kb/CANARY-TMP.md` ran `typos` and `betterleaks` but **not** `terraform-validate`, because that command's glob is `*.tf`, `*.tfvars`, `bin/**`, `.github/workflows/*.yml`, `.trivyignore` and a docs path matches none of them. That glob is deliberate and documented — but the same repo's comment records the earlier version of the bug: with a `*.tf`-only glob, *"a commit touching only `bin/` or `.github/` ran NO validation locally — and CI triggers on `pull_request` only, so it ran none there either until a PR was opened."* Two gates, both green, neither executed.

## Scope

- `.git/hooks/` — what is actually installed, versus what the manager config declares
- Every `glob:` and `exclude:` in `lefthook.yml`
- Hook stage assignment: `pre-commit` vs `pre-push` vs `commit-msg`
- `.github/workflows/*` triggers — `on: push` vs `on: pull_request` vs `paths:` filters

## Patterns to flag

### 1. Hooks not installed

The config is committed; the hooks are per-checkout. A fresh clone has none until someone runs the installer.

```bash
ls -la .git/hooks/ | grep -v '\.sample'
grep -l lefthook .git/hooks/* 2>/dev/null || echo "lefthook NOT installed in .git/hooks"
```

Fix by making installation part of the repo's setup script (`bin/preflight`, `bin/bootstrap`, a `postinstall`), not a line in a README that a new checkout will skip. Then verify a fresh clone gets them.

### 2. A glob too narrow for the change

For each command with a `glob:`, ask what commit shape it misses:

```bash
# Which staged files would each glob actually match?
git diff --cached --name-only
```

The asymmetry that matters: **a glob that is too wide costs runtime; a glob that is too narrow costs coverage, silently.** Default to wide. A check that is cheap (a spell checker at 0.02s) should usually have no glob at all.

### 3. A CI trigger that never fires for the change

```bash
grep -A6 '^on:' .github/workflows/*.yml
```

`on: pull_request` alone means a direct push to a branch runs nothing. `paths:` filters reproduce the narrow-glob bug at the CI layer. If a hook is the only gate and hooks are bypassable (`--no-verify`), then a `paths`-filtered CI job is not a backstop.

### 4. No backstop behind a bypassable hook

Every hook can be skipped with `--no-verify`. For any gate whose failure would be serious — a secret reaching history above all — there must be a CI job that runs on the same content and cannot be skipped locally.

```bash
grep -rl 'gitleaks\|betterleaks' .github/workflows/ || echo "no CI backstop for secret scanning"
```

### 5. Wrong stage

- **A secret scan belongs in `pre-commit`.** At `pre-push` the secret is already in a commit, and removing it means history surgery.
- **A slow whole-project check belongs in `pre-push`.** At `pre-commit` it taxes every commit.
- **A commit-message check belongs in `commit-msg`.** Nowhere else can see the message.

Flag any assignment that contradicts these.

## Fixing

1. Install hooks from the repo's setup script; verify on a throwaway clone.
2. Widen or remove globs that do not need to be narrow; document any glob that stays narrow with what it deliberately skips.
3. Add a CI backstop for every serious gate, triggered on `push` as well as `pull_request`.
4. Move any misplaced stage, and comment the reason at the new site.

## Verification

```bash
# Does a real commit of a docs-only change run what you expect?
printf 'canary\n' > kb/CANARY-TMP.md && git add kb/CANARY-TMP.md
lefthook run pre-commit 2>&1 | tail -8    # read WHICH commands ran, not just the exit code
git reset -q HEAD kb/CANARY-TMP.md && rm -f kb/CANARY-TMP.md
```

The summary lists each command that executed. A gate you expected and do not see listed did not run — that is the finding. Repeat for each distinct commit shape the repo produces: source-only, config-only, docs-only, deletion-only.

## Report format

| Gate | Stage | Glob | Ran on docs-only commit? | Ran on source-only? | CI backstop |
|---|---|---|---|---|---|
| typos | pre-commit | none | yes | yes | no |
| terraform-validate | pre-commit | `*.tf`,`bin/**`,… | **no** (by design) | yes | `pull_request` only |
| betterleaks | pre-commit | none | yes | yes | `secrets.yml` |

Flag every **no** that is not deliberate and documented.
