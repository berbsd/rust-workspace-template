# Magic Number Audit and Const Extraction

## Prompt

Scan this codebase for magic numbers and replace them with named constants. For each magic number found, explain what it represents, suggest an appropriate const name following Rust naming conventions, and place the const in the most logical scope.

## What Qualifies as a Magic Number

**Replace these:**
- Numeric literals representing domain-specific values (buffer sizes, timeouts, thresholds, limits, port numbers)
- String literals representing configuration keys, API paths, header names
- Duration values with unexplained magnitudes
- Array/vector capacities chosen for specific reasons
- Bit masks and shift amounts with domain meaning
- Protocol-specific constants (status codes, field sizes, version numbers)

**Leave these alone:**
- `0` and `1` for initialization, incrementing, boolean-like usage
- Mathematical constants (`std::f64::consts::PI`, etc.)
- Array indices where the meaning is clear from context
- Test values in test functions (unless duplicated across many tests)
- Pattern matching on well-known enum values

## Naming Conventions

Follow Rust conventions for constants:

- `SCREAMING_SNAKE_CASE` for all `const` items
- Name describes **what it represents**, not just its value
- Include units in the name when applicable: `TIMEOUT_SECS`, `BUFFER_SIZE_BYTES`, `MAX_RETRIES`

```rust
// Before: scattered magic numbers
fn process_batch(items: &[Item]) -> Result<(), Error> {
    if items.len() > 1000 {
        return Err(Error::BatchTooLarge);
    }
    let mut buffer = Vec::with_capacity(8192);
    for chunk in items.chunks(64) {
        if process_chunk(chunk, &mut buffer)?.confidence < 0.85 {
            retry_chunk(chunk, Duration::from_secs(30))?;
        }
    }
    Ok(())
}

// After: self-documenting constants
const MAX_BATCH_SIZE: usize = 1000;
const PROCESSING_BUFFER_CAPACITY: usize = 8192;
const CHUNK_SIZE: usize = 64;
const MIN_CONFIDENCE_THRESHOLD: f64 = 0.85;
const RETRY_TIMEOUT: Duration = Duration::from_secs(30);

fn process_batch(items: &[Item]) -> Result<(), Error> {
    if items.len() > MAX_BATCH_SIZE {
        return Err(Error::BatchTooLarge);
    }
    let mut buffer = Vec::with_capacity(PROCESSING_BUFFER_CAPACITY);
    for chunk in items.chunks(CHUNK_SIZE) {
        if process_chunk(chunk, &mut buffer)?.confidence < MIN_CONFIDENCE_THRESHOLD {
            retry_chunk(chunk, RETRY_TIMEOUT)?;
        }
    }
    Ok(())
}
```

## Scope Placement

Place constants at the narrowest scope that covers all usage:

1. **Function-local** — Used in one function only → `const` inside the function
2. **Module-level** — Used across functions in one file → top of the file
3. **Crate-level** — Used across modules → in a `const.rs` or `constants.rs` module (see check #4)
4. **Workspace-level** — Used across crates → in a shared crate (e.g., `common-types`)

## Visibility

- Use `pub(crate)` for constants shared within a crate
- Use `pub` only for constants that are part of the public API
- Keep function-local constants private (no visibility modifier needed)

## Execution Steps

1. **Scan** — Search all `.rs` files for numeric/string literals that look like domain values
2. **Catalog** — List each magic number with: file, line, current value, inferred meaning
3. **Name** — Propose a `const` name for each, following Rust conventions
4. **Place** — Determine the appropriate scope for each constant
5. **Replace** — Extract the constant and replace all occurrences
6. **Verify** — Run `cargo check` after each file, `cargo test` after all changes
7. **Report** — Summarize: number of magic numbers found, constants created, files modified

## Watch Out For

- **Duplicate values with different meanings** — `1024` might mean "max items" in one place and "buffer size" in another. These need different constant names.
- **Values used in const contexts** — Some expressions require `const` evaluation. Ensure the extracted constant works in the same context.
- **Values in macros** — Magic numbers inside macro invocations may need special handling.
- **Test fixtures** — Magic numbers in tests are less critical but still worth naming if they represent domain concepts.
