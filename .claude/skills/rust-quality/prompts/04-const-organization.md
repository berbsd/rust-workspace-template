# Const Organization Across Modules

## Prompt

For each folder in this project, check if multiple files define their own constants. If so, create a `const.rs` module that collects all constants used across files in that folder. Only do this when there are genuinely multiple files containing consts; don't create a `const.rs` for folders where constants live in a single file.

## When to Create `const.rs`

**Do create** when:
- 3+ files in a module directory each define `const` items
- The same conceptual constant appears in multiple files (even with different names)
- Constants are `pub` or `pub(crate)` and imported across sibling modules

**Don't create** when:
- Only 1-2 files define constants (leave them in place)
- Constants are function-local and not shared
- Constants are tightly coupled to a single struct/impl (keep them co-located)

## Structure

```rust
// src/network/consts.rs (or const.rs — match project convention)

/// Maximum number of concurrent connections before backpressure kicks in.
pub(crate) const MAX_CONNECTIONS: usize = 256;

/// Size of the per-connection read buffer in bytes.
pub(crate) const READ_BUFFER_SIZE: usize = 8192;

/// How long to wait for a handshake before dropping the connection.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum message size accepted from a single client.
pub(crate) const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MiB
```

## Visibility Rules

- Default to `pub(crate)` — constants serving a module folder shouldn't leak into the public API
- Use `pub(super)` if the constant only serves the parent module's children
- Use `pub` only if the constant is part of the crate's public API
- Keep function-local constants where they are (don't extract them)

## File Naming

Check the project's existing conventions:
- If the project uses `constants.rs` → use `constants.rs`
- If the project uses `consts.rs` → use `consts.rs`
- If no convention exists → use `consts.rs` (shorter, common in Rust ecosystem)

## Execution Steps

1. **Survey** — For each module directory, count files that define `const` items
2. **Evaluate** — Skip directories with fewer than 3 files defining constants
3. **Group** — Categorize constants by domain (timeouts, sizes, limits, etc.)
4. **Create** — Write `consts.rs` with grouped, documented constants
5. **Register** — Add `mod consts;` to the parent `mod.rs` or `lib.rs`
6. **Migrate** — Replace original constant definitions with imports from `consts`
7. **Deduplicate** — Flag constants with the same value but different names (potential bugs)
8. **Verify** — Run `cargo check` and `cargo test`

## Watch Out For

- **Same value, different meaning** — `const TIMEOUT: u64 = 30` and `const MAX_RETRIES: u64 = 30` happen to share a value but represent different things. Keep them separate.
- **`const` vs `static`** — Only move `const` items. `static` items have different semantics (fixed memory address) and should stay where they are unless there's a specific reason to centralize.
- **Feature-gated constants** — If a constant is behind `#[cfg(feature = "...")]`, preserve the gate in `consts.rs`.
- **Circular imports** — Moving constants to `consts.rs` shouldn't create circular module dependencies. If it would, leave the constant in place.
