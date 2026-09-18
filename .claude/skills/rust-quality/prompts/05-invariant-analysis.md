# Invariant Analysis and Documentation

## Prompt

Analyze this code for spots that would strongly benefit from invariant analysis. For each spot, create a document explaining the invariant, why it matters, what breaks if it's violated, and how the code currently maintains or fails to maintain it.

## What to Look For

### Data Structure Invariants
- Structs with interdependent fields (e.g., `len` must be <= `capacity`)
- Collections that must maintain sorted order
- Indices stored separately from the collection they index
- Reference counts or generation counters

### State Machine Invariants
- Enum-based state machines with implicit transition rules
- States that must never be reached from certain other states
- Fields that are only valid in certain states (`Option<T>` that's `Some` only when active)

### Concurrency Invariants
- Lock ordering requirements (acquire A before B, never reverse)
- Data that must only be accessed from a specific thread/task
- Atomic operation ordering requirements
- Channel protocols (message sequence expectations)

### Resource Invariants
- File handles/connections that must be open during certain operations
- Buffers that must be flushed before close
- Resources with cleanup ordering dependencies

### Numeric Invariants
- Values that must stay within a range
- Quantities that must sum to a fixed total
- Ratios or percentages that must stay normalized

## Output Format

For each invariant found, document:

```markdown
### Invariant: [Short Name]

**Location:** `src/pool.rs` — `ConnectionPool` struct

**Statement:** `active.len() + idle.len() == total_allocated` at all times.

**Why it matters:** The pool pre-allocates connections at startup. If this invariant breaks,
the pool either leaks connections (total grows) or double-frees them (total shrinks).

**What breaks if violated:**
- If `active + idle > total`: memory leak, eventual OOM under sustained load
- If `active + idle < total`: connection reused while still active → data corruption

**How the code maintains it:**
- `acquire()` moves from `idle` to `active` (transfer, not create)
- `release()` moves from `active` to `idle` (transfer, not destroy)
- `drop()` decrements `total` and destroys the underlying resource

**Gaps found:**
- `release()` does not verify the connection was in `active` — a double-release
  would increment `idle` without decrementing `active`, breaking the invariant.
- No `debug_assert!` guards this invariant.

**Recommended guard:**
```rust
debug_assert_eq!(
    self.active.len() + self.idle.len(),
    self.total_allocated,
    "connection pool invariant violated"
);
```
```

## Execution Steps

1. **Scan** — Identify structs, enums, and modules with complex state
2. **Analyze** — For each, determine what invariants must hold
3. **Trace** — Follow every code path that modifies the relevant state
4. **Document** — Write the invariant document with the format above
5. **Recommend** — Suggest `debug_assert!` guards, wrapper methods, or type-system enforcement
6. **Present** — Show the document to the user for review (this check produces documentation, not code changes by default)

## Where to Put the Documentation

- If the project has a `docs/` directory → `docs/invariants.md`
- If no docs directory → present inline and let the user decide placement
- For individual invariants, also add `debug_assert!` comments near the relevant code
- Consider adding invariant checks as `#[cfg(debug_assertions)]` methods on the struct
