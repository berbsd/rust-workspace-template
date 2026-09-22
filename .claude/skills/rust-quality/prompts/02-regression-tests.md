# Regression Test Generation from Branch Fixes

## Prompt

Look at the fixes committed in the current branch and propose a list of regression tests to add. For each test, explain what bug it would catch if the fix were accidentally reverted, and provide a recommended implementation.

## How It Works

1. **Identify the base branch** — Determine what branch the current branch diverged from (usually `main` or `develop`)
2. **Analyze the diff** — Examine each commit on the current branch, focusing on bug fixes
3. **Understand the failure mode** — For each fix, determine what went wrong before the fix
4. **Generate tests** — Write tests that would fail if the fix were reverted

## What to Look For in Diffs

### Boundary condition fixes
- Added bounds checks, `is_empty()` guards, overflow protection
- Changed `<` to `<=` or `>` to `>=`
- Added `None`/`Err` handling where there was an `unwrap()`

### Error handling additions
- New `match` arms or `if let` branches
- Added error variants
- Changed `unwrap()` to `?` or proper error handling

### Off-by-one fixes
- Index arithmetic changes
- Range endpoint changes (exclusive vs inclusive)
- Loop bound changes

### Edge case handling
- Empty input handling
- Zero/negative value handling
- Unicode/special character handling
- Maximum value / overflow handling

## Test Structure

Each regression test should:

1. Have a descriptive name: `regression_<what_it_guards_against>`
2. Include a comment explaining the original bug
3. Reproduce the exact conditions that triggered the bug
4. Assert the correct behavior (not just "doesn't panic")

```rust
#[test]
fn regression_empty_input_should_not_panic() {
    // Previously caused index-out-of-bounds in parse_header().
    // Fixed in commit abc1234.
    let result = parse_header(b"");
    assert!(result.is_err());
}

#[test]
fn regression_single_byte_input_returns_incomplete() {
    // Edge case discovered alongside the empty input fix.
    let result = parse_header(b"\x01");
    assert_eq!(result, Err(ParseError::Incomplete));
}
```

## Execution Steps

1. **Get the diff** — Run `git log --oneline main..HEAD` and `git diff main...HEAD` to see all changes
2. **Identify fixes** — Look for commits with "fix", "bug", "handle", "guard", "check" in messages, and diffs that add error handling or boundary checks
3. **Propose tests** — For each fix, write a regression test with explanation
4. **Present for review** — Show the proposed tests to the user before adding them
5. **Add tests** — Place them in the appropriate test module (unit tests near the code, integration tests in `tests/`)
6. **Verify** — Run `just check` (or `cargo nextest run --all-features` after `just db-ensure`) to confirm the new tests pass
