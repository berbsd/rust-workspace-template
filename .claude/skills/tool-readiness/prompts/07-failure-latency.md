# Failure Latency & Ordering

Make the failure path fast. Find hook suites where a check that fails in milliseconds still costs the developer the full runtime of the slowest command.

## Why

`parallel` and `piped` optimise opposite cases. Parallel minimises the **pass** path; piped-with-priority minimises the **fail** path. When one command dominates the runtime, parallelism has nothing left to win and early exit has everything.

Measured in `infra` on 2026-09-09:

| Command | Time |
|---|---|
| `typos` | 0.02s |
| `betterleaks` | 0.17s |
| `bin/validate` | 36.16s |

Sequential total 36.35s, parallel 36.16s — **0.19s saved, 0.5%**, inside `bin/validate`'s own run-to-run variance. Then the same staged typo, timed both ways:

| Mode | Failure path | Happy path |
|---|---|---|
| `parallel: true` | **34.37s** — `typos` failed at 0.01s, validate ran to completion anyway | ~36.2s |
| `piped: true` + `priority` | **0.06s** — stopped at typos | 35.95s |

Same defect, **570× difference**, with no measurable cost to the pass path. The slow path is the one developers experience repeatedly while fixing something.

## Scope

- `parallel:` / `piped:` / `priority:` in every hook stage
- Per-command runtime, measured — not estimated
- The dependency graph from check #8 (which commands mutate, and therefore cannot be reordered freely)

## Patterns to flag

### 1. Parallel where one command dominates

```bash
for c in <each command>; do
  s=$(date +%s.%N); <command> >/dev/null 2>&1; e=$(date +%s.%N)
  printf "%-20s %.2fs\n" "$c" "$(echo "$e-$s"|bc)"
done
```

If the longest command is more than ~10× the sum of the rest, parallelism is buying nothing. Switch to `piped: true` and order cheapest-first.

### 2. Alphabetical ordering by default

**Lefthook orders commands alphabetically unless `priority:` is set.** This is the trap: `betterleaks, terraform-validate, typos` puts the 36s command *second* and the 0.02s command last. Piped mode without explicit priorities can be slower on the failure path than parallel.

```yaml
pre-commit:
  piped: true
  commands:
    typos:
      priority: 1        # 0.02s
    betterleaks:
      priority: 2        # 0.17s
    terraform-validate:
      priority: 3        # 36s
```

Order by measured cost ascending, subject to check #8's mutation constraints.

### 3. A slow whole-project check in `pre-commit`

If a check does not depend on what was staged, it belongs in `pre-push`. `web` splits on exactly this line — pre-commit stays in the ~10s range on staged files; typecheck, full test suite and production build run once per push.

### 4. Measuring the happy path only

A hook suite tuned on its pass time is tuned for the case that costs nothing. Always time both, with a real defect staged.

## Fixing

1. Measure every command. Record the numbers in a comment — they justify the ordering and let the next person re-derive it.
2. Choose `piped: true` when one command dominates; keep `parallel: true` only when times are comparable **and** check #8 confirms nothing mutates.
3. Set explicit `priority:` — never rely on alphabetical order.
4. Move stage-inappropriate checks (see check #4).
5. Re-time the failure path and record the improvement.

## Verification

```bash
# failure path — plant a defect the FIRST check catches
printf '# teh seperate availabe\n' > bin/canary-tmp.sh && git add -f bin/canary-tmp.sh
s=$(date +%s.%N); lefthook run pre-commit 2>&1 | tail -6; e=$(date +%s.%N)
printf "fail path: %.2fs\n" "$(echo "$e-$s"|bc)"

# happy path — same file, defect removed
printf '# a separate available canary\n' > bin/canary-tmp.sh && git add -f bin/canary-tmp.sh
s=$(date +%s.%N); lefthook run pre-commit >/dev/null 2>&1; e=$(date +%s.%N)
printf "pass path: %.2fs\n" "$(echo "$e-$s"|bc)"

git reset -q HEAD bin/canary-tmp.sh; rm -f bin/canary-tmp.sh
```

Read the summary, not just the timing: it names which commands ran. In piped mode the ones after the failure should be absent.

## Report format

| Stage | Mode | Order | Pass path | Fail path (defect in cheapest) |
|---|---|---|---|---|
| pre-commit | `parallel: true` | n/a | 36.2s | **34.37s** |
| pre-commit | `piped` + priority | typos→betterleaks→validate | 35.95s | **0.06s** |

Report measured seconds. An ordering change without before/after numbers is a preference, not a finding.
