# Gate Falsifiability

Prove every gate can fail, with a fixture the tool actually reacts to. A gate that has never been watched failing is unverified, no matter how carefully it is configured.

## Why

Checks #1 and #2 establish that rules loaded and the config applies. Neither proves the tool will *act*. The only evidence for that is watching it fail on a planted defect and pass once the defect is removed.

The trap that makes this harder than it sounds: **your canary may be allowlisted.** Verified on 2026-09-09, all three of these produced `exit 0` from a correctly configured secret scanner:

- `AKIAIOSFODNN7EXAMPLE` — AWS's own documented example key, which gitleaks deliberately ignores. A false negative that would have masked a genuinely broken gate.
- A **randomly generated** `AKIA` + 16 chars — the AWS rule needs more context than a well-shaped string, so a random key alone triggers nothing.
- The same random key while the config loaded **0 rules** — the real bug, invisible behind the two fixture failures above.

The fixture that did work, and which `infra/.gitleaks.toml` prescribes in its own header:

```bash
printf 'k="sk_live_51QwErTyUiOpAsDfGhJkLzXcV"' > /tmp/canary/canary.txt
```

Three failed canaries in a row before finding one that fires is the normal experience. Budget for it, and never conclude "the gate works" from a fixture you have not seen fire somewhere.

## The verification ladder — all three rungs, in order

A tool is not "working" until it has been proven at every layer that stands between it and a commit. Each rung tests something the one below cannot, and passing a lower rung says nothing about a higher one.

| Rung | Command | What only this rung proves |
|---|---|---|
| **1. Direct** | `typos <file>` / `betterleaks git . --staged` | the binary runs, the config loads, and the tool reacts to the defect |
| **2. Through the manager** | `lefthook run pre-commit --command <name>` | the hook entry's `run:`, `glob:` and `exclude:` select this file and the exit code propagates |
| **3. Through the real gate** | `git commit -m "canary"` | the manager is installed into `.git/hooks` and the commit is actually **refused** |

**Report a gate as working only with all three recorded.** The gaps between rungs are real failures, each observed in this codebase:

- **1 passes, 2 fails** — the tool works; the glob does not match the file, or the config's exclusions are bypassed by `{staged_files}` (check #2), so the hook entry never inspects it.
- **2 passes, 3 fails** — the command works; `lefthook install` was never run in this checkout, so `.git/hooks/pre-commit` does not exist and every commit sails through. `bin/preflight` guards exactly this: *"An installed binary is not an installed hook … without it every tool above is present and NOTHING runs at commit time."*
- **3 "passes" vacuously** — the commit succeeded because the gate did not run, not because the content was clean. Always assert on `git log --oneline -1`: HEAD must be **unchanged** after a canary commit attempt.

Rung 3 is the one people skip because it is inconvenient — it needs a throwaway file and a reset. Skip it and you are testing a command, not a gate.

Run the ladder on **both** polarities: with the defect (all three must fail) and without it (all three must pass). Six results per gate.

## Scope

Every gate in the commit path, and every scanner in CI:

- Each command in `lefthook.yml` `pre-commit` / `pre-push` / `commit-msg`
- Each scanner step in `.github/workflows/*`
- Repo-specific assertion suites (`bin/validate`, `bin/post-deploy.d/*`)

## Patterns to flag

### 1. A gate never observed failing

Any gate with no recorded falsification. The remedy is the four-step cycle, per gate:

1. Plant the defect it exists to catch.
2. Run it. Confirm **FAIL**, and read the message — it must name the defect.
3. Remove the defect.
4. Run it. Confirm **PASS**.

Record both results. A gate reported as working on the strength of step 4 alone is not verified.

### 2. A fixture the tool ignores by design

Before trusting a negative result, prove the fixture fires *somewhere*:

```bash
mkdir -p /tmp/canary && printf 'k="sk_live_51QwErTyUiOpAsDfGhJkLzXcV"' > /tmp/canary/c.txt
betterleaks dir /tmp/canary -c .gitleaks.toml    # must report 1 leak
```

If the fixture does not fire in a deliberately unconfigured context, it is a bad fixture — not a passing gate. Known-bad fixture families: vendor "EXAMPLE" credentials, RFC example values, `test`/`dummy`/`changeme` strings, and anything already in the repo's allowlist.

### 3. A gate that cannot distinguish "violated" from "could not test"

The most dangerous shape, because it renders as a pass:

```bash
value="$(some-tool ... 2>/dev/null || true)"
[ "$value" = "$expected" ] && pass      # empty → neither pass nor fail
```

Every read needs three outcomes: matches (PASS), differs (FAIL), unreadable (FAIL or a loud WARN — never silence). An auth failure, a missing binary and a satisfied invariant must never look alike.

### 4. Suppressions that swallow the canary

After adding any suppression, re-run the canary. A suppression written to silence one false positive routinely silences the class. Check #6 covers scoping; this check covers *noticing*.

### 5. A gate that runs but whose result is discarded

```bash
grep -nE '\|\| true|; *true|continue-on-error|--exit-code[= ]0|set \+e' lefthook.yml .github/workflows/*.yml
```

Each of these turns a failing gate into a green one. Some are deliberate — every one needs a comment saying why, or it is a finding.

## Fixing

For every gate, produce the evidence table below. Where a gate cannot be falsified at all — no defect can be constructed — that is itself the finding: either the gate asserts nothing, or the invariant is not testable and the gate is decoration.

**Test under `bash`, not the interactive shell.** An unmatched glob is fatal in zsh and expands literally in bash; a probe that "fails correctly" in zsh may be testing the shell.

## Verification

```bash
lefthook run pre-commit --command <name>   # with defect staged: non-zero
lefthook run pre-commit --command <name>   # defect removed: zero
git commit -m "canary"                     # the real path — must be REJECTED
git log --oneline -1                       # HEAD must be unchanged
```

Run the real `git commit` at least once per gate. `lefthook run` exercises the command; only a commit proves the hook is wired to the commit (see check #4).

## Report format

| Gate | Defect planted | Observed | Reverted | Fixture verified elsewhere? |
|---|---|---|---|---|
| typos | `teh`/`availabe`/`seperate` | FAIL, exit 2, 0.01s | PASS | n/a — dictionary hit |
| betterleaks | `sk_live_…` staged | FAIL, exit 1 | PASS | yes — fired via `dir` on unconfigured tmp |
| betterleaks | random `AKIA…` | **PASS — bad fixture** | — | no — discarded |

List discarded fixtures too. They are the evidence that a negative result was investigated rather than assumed.
