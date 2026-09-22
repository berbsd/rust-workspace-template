# Dead Code Removal with Test-Aware Scoping

## Prompt

Remove all `#[allow(dead_code)]` annotations and the dead code they protect. If any of the dead code is actually needed for tests, wrap it with `#[cfg(test)]` instead. Ensure the codebase compiles cleanly without any `dead_code` suppressions.

## Approach

### Phase 1: Remove Annotations

1. Find all `#[allow(dead_code)]` annotations in the codebase
2. Remove each annotation (but don't delete the code yet)
3. Run `cargo check` to see what the compiler flags as unused

### Phase 2: Evaluate Each Item

For every item the compiler flags as dead code:

**Delete if:**
- Nothing references it (no callers, no trait impls, no macro usage)
- It's been superseded by a newer implementation
- It references deprecated or removed types/APIs

**Wrap with `#[cfg(test)]` if:**
- Used in test functions but not production code
- It's a test utility (helper functions, mock builders, fixture generators)
- It's a debug formatter used only in test assertions

**Keep (and investigate) if:**
- Used via proc macros (the compiler can't see proc macro usage)
- Referenced in build scripts or code generation
- Part of a public API that external consumers might use
- Behind a feature flag that wasn't enabled during the check

```rust
// Before: suppressed warnings, unclear what's actually used
#[allow(dead_code)]
fn format_debug_output(state: &AppState) -> String {
    // ... formatting logic
}

#[allow(dead_code)]
struct LegacyConfig {
    // ... fields from a migration two years ago
}

// After: test utility properly scoped, dead code removed
#[cfg(test)]
fn format_debug_output(state: &AppState) -> String {
    // ... formatting logic, used in integration tests
}

// LegacyConfig deleted entirely: no references anywhere
```

### Phase 3: Verify

1. `cargo check` — zero `dead_code` warnings, zero `#[allow(dead_code)]`
2. `just check` (or `cargo nextest run --all-features`) — all tests pass (test-only code properly scoped)
3. `cargo check --all-features` — check feature-gated code too

## What NOT to Remove

- **Trait implementations** — A struct implementing `Display` might appear unused if it's only used via `format!()` or in test output
- **Proc macro inputs** — `#[derive(...)]` generates code that references fields the compiler doesn't track as "used"
- **FFI exports** — `#[no_mangle]` functions called from C/external code
- **Conditional compilation** — Code behind `#[cfg(target_os = "...")]` for a different platform
- **Public library API** — If this is a library crate, public items are part of the contract even if unused internally

## Also Clean Up

While removing dead code, also address:

- **Unused imports** — `use` statements that no longer have references
- **Unused dependencies** — Crates in `Cargo.toml` that nothing imports (use `cargo-udeps` or `cargo machete` if available)
- **Orphaned test modules** — `#[cfg(test)] mod tests { }` with no test functions inside
- **Commented-out code** — Code that's been commented out for a long time (check git blame for age)

## Execution Steps

1. **Inventory** — Find and count all `#[allow(dead_code)]` annotations
2. **Remove annotations** — Strip all `#[allow(dead_code)]`
3. **Compile** — Run `cargo check` to get the full list of dead code warnings
4. **Evaluate** — For each warning: delete, `#[cfg(test)]`, or investigate
5. **Apply** — Make the changes, compiling after each batch
6. **Clean up** — Remove resulting unused imports and dependencies
7. **Verify** — `cargo check` (zero warnings) + `just check` (all pass)
8. **Report** — Lines removed, items scoped to test, items kept with justification

## Watch Out For

- **Items referencing other dead items** — If A calls B and both are dead, removing A alone won't flag B. Evaluate chains together.
- **Bit rot** — Dead code may not compile if you remove `#[allow(dead_code)]` but leave it in place. Type signatures may reference types that have changed. Delete rather than trying to fix dead code to compile.
- **Integration tests** — Items used in `tests/` directory integration tests won't be flagged by `cargo check` on the library. Check both `cargo check` and `cargo check --tests`.
