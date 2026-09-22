# Cross-Crate Duplication

Find logic copied across sibling crates or services — transport plumbing,
decode-and-map blocks, validation shapes, small helpers — and consolidate it
behind one definition. Focus on the copies that have already drifted, because
drift is the cost this check exists to catch.

## Why

Duplication inside one file is visible: a reviewer sees both copies. Across
crates it is not. Nobody opens `user-client` and `widget-client` side by side,
so the copies diverge silently and each divergence looks intentional in
isolation.

The workspace carried a worked example. Nine `*-client` crates held twenty-one
copies of the same twelve lines — check status, read the body, log it, read the
bytes, deserialize — and they had already drifted in exactly the ways copies
do:

- some logged the error body, some did not;
- neighbouring call sites mapped an identical body-read failure to `Transport`
  in one and `ResponseParse` in the other;
- one added a `404`-means-absent branch that its siblings would also have
  wanted.

None of that was a bug on the day it was written. It became one when a reader
had to know which of nine spellings a given service used.

## What blocks consolidation, and what to do about it

The reason a copy survives is almost never the logic. It is one small piece
that differs, and the fix is to make that piece the parameter.

| The copies differ in… | Consolidate by… |
|---|---|
| Their error type (each crate owns its enum) | A trait with the constructors they share, generic over `E` |
| A log string or metric label | A `context: &str` parameter |
| Whether they decode a body or only check status | Two functions, not one with a `bool` |
| One caller's extra branch (a `404` that means "absent") | Let it match on the shared function's error and handle it there |
| A type they return | Generics **only** where the type is genuinely the caller's; see below |

**When a caller cannot use the whole helper, find the smaller piece it can.**
`gemini-client` reads a `429` body to compute a retry delay — not a decode, not
an error to propagate, and its error enum does not fit the shared trait. The
answer was not to leave the copy: the four-line lossy body read came out as its
own function and now serves all three call sites.

## What is *not* duplication

Do not consolidate on shape alone. Two blocks that look alike but answer
different questions should stay apart — merging them produces a helper with a
mode flag, which is worse than the copies.

Be especially careful with **generics as a duplication dodge**. Making a
function generic over its return type removes the second definition but also
removes the compiler's check that both sides agree; the drift moves from
"two structs to keep in step" to "a runtime deserialize failure nobody sees
until production". If a wire type is shared across crates, define it once
(e.g. in `common-types`) and import it; reach for `T: DeserializeOwned` only
where the type really is the caller's private business.

## Auditing

1. Pick a distinctive line — a log message, an error string, a comment — that
   the suspected copies share:

   ```bash
   grep -rl "failed to read error response body" crates/ services/
   for f in $(grep -rl "<marker>" crates/); do
     echo "$(grep -c '<marker>' "$f")  $f"; done | sort -rn
   ```

2. Count per crate, not per repository. A concentration (six copies in one
   crate) is a different problem from a spread (one each in nine) — the first
   is a crate that needed a local helper, the second is a workspace gap.

3. **Diff the copies against each other**, not against an ideal. Every place
   they differ is either a drift to fix or a parameter to extract, and you
   cannot tell which without reading them.

4. Decide where the single definition lives — the categories below are
   illustrative; name this workspace's actual crates, not these examples:
   - transport, retry, decode → a shared HTTP-client crate, if one exists
   - service scaffolding, extractors, middleware → a shared service-framework crate, if one exists
   - wire types crossing one service's boundary → that `<svc>-client`, if this workspace has service-to-service client crates
   - workspace-wide types → `crates/common-types`

5. Convert one crate first and run its tests before scripting the rest. A
   regex over the remaining call sites will silently miss the ones that differ
   — which are exactly the interesting ones, so re-run the count afterwards and
   read whatever is left by hand.

## Fixing

```bash
cargo build --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features
cargo nextest run --workspace --lib --all-features
```

Expect unused imports after removing a copy (`tracing::{error, warn}` usually);
`cargo fix --allow-dirty --lib` clears them. Watch for a helper appended after
a `#[cfg(test)] mod tests` block — clippy's `items_after_test_module` catches
it, and it is easy to introduce when scripting an edit onto the end of a file.

## Report format

A table of `marker | crates | copies before | copies after | blocked by`, then:

- **Drifts found** — where the copies disagreed, and which behaviour won. This
  is the finding; the line count is not.
- **Not consolidated** — each remaining copy with the reason it differs, so the
  next reader does not re-derive it.
- **New shared surface** — what was added and where, since a helper nobody can
  find gets copied again.
