# Function Decomposition

Find oversized or multi-purpose functions, decompose them, and ensure every function carries a rustdoc summary. Enforce the workspace rule that size/complexity lints must not be silenced with `#[allow(...)]`.

## What to find

Scan all `*.rs` files in `crates/` and `services/`. Skip `target/`, generated code, and `tests/` modules.

For each function (`fn`, `async fn`, methods on `impl` blocks):

1. **Measure body size.** Count lines from the opening `{` to the closing `}`, excluding blank lines and comments. Flag functions with more than **50 effective lines**.
2. **Detect silencer attributes.** Flag any function or module carrying:
   - `#[allow(clippy::too_many_lines)]`
   - `#[allow(clippy::cognitive_complexity)]`
   - `#[allow(clippy::too_many_arguments)]`
   - `#[allow(clippy::cyclomatic_complexity)]`
   These are blockers — the rule is to fix the function, not silence the lint.
3. **Detect multi-purpose functions.** Heuristics for "doing more than one thing":
   - Multiple distinct top-level sections separated by blank lines or block comments.
   - More than one `match`/`if-let-else` chain on different domain values.
   - A mix of I/O (DB, HTTP, file) and non-trivial computation in the same body.
   - Sections that could be named (e.g. "validate inputs", "load data", "transform", "persist").
4. **Detect missing rustdoc.** Flag any `fn` (including private) without a `///` summary line. Exemptions:
   - Methods inside a `trait` impl whose trait already documents them.
   - Trivial accessors of the form `fn x(&self) -> &T { &self.x }` (one line, no logic).
   - Functions inside `#[cfg(test)]` modules.

## Decomposing oversized functions

Decompose by extracting cohesive sections into named helpers. Common patterns:

- **Validate → Load → Compute → Persist** — split each phase into its own function. The original becomes an orchestrator that calls them in sequence.
- **Branch-heavy `match`** — extract each arm's body into a function named after what that arm does.
- **Loop bodies > 10 lines** — extract the per-iteration work into a helper; keep the loop itself a one-liner.
- **Setup + work + teardown** — extract setup and teardown so the work is the visible body.

Helpers should:

- Have a clear, action-named identifier (`build_invoice_line`, not `helper_1`).
- Take only the inputs they actually need (avoid passing whole `&AppState` if a single field is enough).
- Carry a rustdoc summary describing what they do and any error/panic contracts.

Orchestrator functions (handlers, top-level service methods, `main`) are allowed to remain longer than helpers, but their body should read as a sequence of named steps — each line a function call or a short conditional, not implementation detail.

## Removing silencers

When you encounter `#[allow(clippy::too_many_lines)]` or similar:

1. Decompose the function until the lint passes naturally.
2. Remove the `#[allow]` attribute.
3. Confirm `cargo clippy --workspace --all-targets` passes with no warnings.

If decomposition is genuinely impractical (rare — e.g. a generated dispatch table), replace the blanket `#[allow]` with a `// JUSTIFY:` comment explaining why and ask for human review before keeping it.

## Adding rustdoc

For every function flagged as missing docs, add a summary line. For non-trivial ones, follow the rust-documenter rules:

- One-sentence summary in third-person present tense (`Returns ...`, `Parses ...`, `Computes ...`).
- `# Errors` section if the function returns `Result` — list each error variant.
- `# Panics` section if it can panic — list conditions.
- `# Safety` section for `unsafe fn`.

For genuinely trivial private helpers, a one-line `/// Returns the foo for bar.` is enough — but it must exist.

## Verification

After changes:

```bash
cargo clippy --workspace --all-targets --no-deps
cargo check --workspace
cargo nextest run --workspace --all-features
```

All three must pass with zero warnings on the affected crates.

## Report format

Group findings by file:

```
crates/foo/src/bar.rs
  - fn `process_order` (lines 42–137, 81 effective): oversized + multi-purpose
    Suggested split: validate_order, charge_card, persist_receipt, notify_user
  - fn `helper_thing` (line 200): missing rustdoc
  - fn `legacy_pipeline` (line 250): silenced with allow(clippy::too_many_lines) — must remove
```

End with a summary line: `N functions over 50 lines, M silencer attributes, K functions missing rustdoc`.
