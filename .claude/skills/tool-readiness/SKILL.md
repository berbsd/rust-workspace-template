---
name: tool-readiness
description: "Use when installing a new tool in a repo, changing a tool's configuration, wiring or editing a git hook, or checking that an existing tool is actually doing anything — a linter, formatter, secret scanner, spell checker, type checker, security scanner or test runner. Triggers on \"add a tool\", \"install a linter\", \"wire it into pre-commit\", \"update the hook\", \"is this gate running\", \"does this check actually fail\", \"why didn't this catch it\", \"0 rules\", \"no leaks found\", \"passes on everything\", \"hook not firing\", \"exclusions not applying\", \"same tool different repo\", \"tool version skew\", \"parallel or piped\", or any suspicion that a tool reports success without inspecting anything. Also use before committing a change to lefthook.yml, .gitleaks.toml, .typos.toml, or any tool config."
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Task
---

# Tool Readiness

Targeted audits for the tooling layer — the linters, scanners, formatters and hooks that are supposed to catch mistakes before they land. Each check is a focused prompt that verifies one way a tool can be present, configured, wired, and still inspect nothing.

A tool that reports success without inspecting anything is worse than no tool, because it buys silence. The suite goes green, the hook prints a tick, and nobody looks again. **A passing gate is not evidence.** Every check here targets a failure that survives a green run.

The four questions this skill exists to answer, for every tool in every repo:

| | Question | Checks |
|---|---|---|
| **a** | Is it **set up** correctly? | 1, 2, 6, 10 |
| **b** | Is it **enabled** — does it actually fail? | 1, 3, 5 |
| **c** | Is it **hooked** where appropriate? | 4, 7, 8 |
| **d** | Is it **consistent** across repos? | 9 |

## Available Checks

| # | Check | Type | Prompt File |
|---|-------|------|-------------|
| 1 | **Config Load Verification** | Audit / Config changes | `prompts/01-config-load.md` |
| 2 | **Invocation-Mode Drift** | Audit / Config changes | `prompts/02-invocation-mode.md` |
| 3 | **Gate Falsifiability** | Audit | `prompts/03-gate-falsifiability.md` |
| 4 | **Hook Installation & Reach** | Audit / Config changes | `prompts/04-hook-reach.md` |
| 5 | **Tool Availability & Fail-Closed** | Audit / Code changes | `prompts/05-tool-availability.md` |
| 6 | **Suppression Scope** | Config changes / Audit | `prompts/06-suppression-scope.md` |
| 7 | **Failure Latency & Ordering** | Config changes | `prompts/07-failure-latency.md` |
| 8 | **Mutation Safety** | Audit / Config changes | `prompts/08-mutation-safety.md` |
| 9 | **Cross-Repo Consistency** | Audit / Config changes | `prompts/09-cross-repo-consistency.md` |
| 10 | **Version Pinning & Skew** | Audit / Config changes | `prompts/10-version-skew.md` |

## Check Categories

### Silent-failure hunting (highest value — these pass every existing gate)
- **Config Load Verification (#1)** — a config that replaces the ruleset instead of extending it, loads zero rules, and exits 0 on everything
- **Invocation-Mode Drift (#2)** — passing explicit paths silently disables the config's own exclusions; scanning a directory instead of the index silently changes the corpus
- **Gate Falsifiability (#3)** — no gate is trusted until it has been watched failing, with a fixture the tool actually reacts to
- **Tool Availability & Fail-Closed (#5)** — a hook whose binary is missing on a fresh checkout, and whether that fails open

### Wiring (the tool works; nothing calls it)
- **Hook Installation & Reach (#4)** — hooks never installed, globs too narrow to match the commit, no CI backstop behind a bypassable hook
- **Failure Latency & Ordering (#7)** — a 0.02s check that costs 34s because it runs beside a slow one instead of before it
- **Mutation Safety (#8)** — which commands write files or touch the index, and therefore cannot run in parallel

### Truthfulness
- **Suppression Scope (#6)** — allowlists scoped by path rather than content, and suppressions with no reason and no removal condition

### Consistency
- **Cross-Repo Consistency (#9)** — the same tool, invoked differently, configured differently, or missing entirely across sibling repos
- **Version Pinning & Skew (#10)** — a formatter at a different version locally than in CI reformats the world

## Core Principles

1. **A green run is not evidence.** The only evidence a gate works is having watched it fail on a defect and pass once the defect was removed. Report both results, or report the gate as unverified.
2. **Test the tool directly AND through every integration.** Three rungs, all required before declaring a tool working: the binary on a file, the hook manager's entry for it (`lefthook run pre-commit --command <name>`), and a real `git commit` that must be refused. A tool that fails correctly on rung 1 and is never reached on rungs 2 or 3 is a tool nobody is running. Check #3 has the ladder.
3. **Verify the config, not the run.** "No findings" and "no rules loaded" are the same output. Every scanner has a way to print what it actually loaded — use it, and record the number.
4. **Absence must be distinguishable from compliance.** A check whose read failed and a check whose invariant holds must not produce the same result.
5. **Suppress by content, never by path.** Path scoping removes the file from the corpus before any rule runs, so the suppression silences everything in that file, forever, including what you have not written yet. Where a path exclusion is genuinely correct, it needs a reason, a removal condition, and an explicit statement of what it blinds.
6. **Prefer failing closed.** A missing binary, an unreadable config or an unparsable file is a failure, not a skip. If a tool must be optional, say so explicitly and loudly.
7. **Fix the class, not the instance.** If one repo's hook was wrong, check the sibling repos for the same shape before closing the finding.
8. **Never widen a suppression to make a run green.** If a real finding is inconvenient, escalate it — do not broaden an exclude. Broadening is how a gate becomes a placebo.
9. **Record the tool version.** Behaviour differs across versions; a finding without a version is not reproducible.

## Execution

### Step 1: Ask the User What to Run

- **"Run all"** — all 10 checks in the recommended order
- **"New tool"** — checks #1, #3, #4, #5, #9, #10 (the install path)
- **"Config change"** — checks #1, #2, #3, #6 (the edit path)
- **"Hook change"** — checks #4, #7, #8, #3
- **"Consistency sweep"** — check #9, then whichever it flags
- **"Audit tool: [name]"** — all checks scoped to one tool
- Pick specific checks by number

### Step 2: Inventory Before Auditing

Before running any check, build the inventory — every later check refers to it:

```bash
# Hook manager config
for f in lefthook.yml lefthook.yaml .pre-commit-config.yaml .husky/*; do
  [ -e "$f" ] && echo "hooks: $f"
done
# Tool configs
ls -a | grep -iE '^\.?(gitleaks|betterleaks|typos|_typos|eslint|prettier|rustfmt|clippy|trivy|checkov|tflint)'
# What the hooks actually invoke
grep -hoE '^\s+run:.*' lefthook.yml 2>/dev/null | sed 's/^\s*run:\s*//'
```

Record, for each tool: the binary, its version, its config file, its invocation, which hook stage runs it, and its glob.

### Step 3: Execute Checks

For each selected check: read the prompt from `prompts/`, follow it precisely, and record evidence — commands run and their output, not conclusions.

### Step 4: Report

End with the readiness matrix (see check #9's report format) plus, for every gate touched, the falsification evidence from check #3.

## Recommended Order (Full Sweep)

Ordered so each check's findings feed the next:

1. **Config Load Verification (#1)** — if the ruleset is empty nothing else matters
2. **Invocation-Mode Drift (#2)** — determines which config applied during #1
3. **Suppression Scope (#6)** — what the loaded rules are prevented from seeing
4. **Gate Falsifiability (#3)** — now that the corpus and rules are known, prove it fails
5. **Tool Availability & Fail-Closed (#5)** — would it survive a fresh checkout
6. **Hook Installation & Reach (#4)** — does anything call it, on the commits that matter
7. **Mutation Safety (#8)** — which commands write, before touching ordering
8. **Failure Latency & Ordering (#7)** — order them now that mutation constraints are known
9. **Version Pinning & Skew (#10)** — pin what is now known to work
10. **Cross-Repo Consistency (#9)** — propagate every fix to the sibling repos

Checks #1–#3 are the core. If you run nothing else, run those three.

## Parallelization

**Safe to parallelize:** #1 + #10, #6 + #8, #4 + #5. All are read-only.

**Must run sequentially:** #1 → #2 → #3 (falsifiability is meaningless before you know which config loaded), #8 → #7 (ordering depends on which commands mutate), everything → #9 (consistency propagates the other checks' fixes).

## Integration with Other Skills

### infra-quality (a sibling infra repo, if one exists)
If your organization keeps infrastructure config in a separate repo with an equivalent skill, its "Check Falsifiability" check is the infra-specific sibling of #3 here: that one targets deploy-time assertions against live cloud state, this one targets tools in the commit path. Findings from #6 here often belong in that repo's `SECURITY.md`.

### rust-quality (this workspace)
Its lint-suppression principle ("never silence lints without permission") is the same rule as #6 here, applied to source rather than tool config.
