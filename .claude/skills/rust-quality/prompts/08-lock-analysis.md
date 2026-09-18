# Lock Analysis and Lock-Free Opportunities

## Prompt

Examine all locks in this codebase. Identify which ones protect critical sections, assess contention risk, and recommend where a lock-free approach using atomics or concurrent data structures would be a net improvement. For each recommendation, explain the trade-off between implementation complexity and expected gain.

## What to Scan For

### Lock Types
- `std::sync::Mutex<T>`
- `std::sync::RwLock<T>`
- `tokio::sync::Mutex<T>`
- `tokio::sync::RwLock<T>`
- `parking_lot::Mutex<T>` / `parking_lot::RwLock<T>`
- `std::sync::Condvar`

### For Each Lock, Evaluate

1. **What it protects** — What data is inside the lock?
2. **Access pattern** — Read-heavy? Write-heavy? Mixed?
3. **Hold duration** — Brief (increment counter) or extended (multi-step transaction)?
4. **Contention risk** — Is this in a hot path? How many threads/tasks access it?
5. **Await crossing** — Is the guard held across `.await` points?

## Common Upgrade Paths

### Simple Counters: `Mutex<u64>` → `AtomicU64`

```rust
// Before: Mutex for a simple counter
let request_count = Arc::new(Mutex::new(0u64));
// In hot path:
*request_count.lock().unwrap() += 1;

// After: Atomic, no lock needed
let request_count = Arc::new(AtomicU64::new(0));
// In hot path:
request_count.fetch_add(1, Ordering::Relaxed);
```

### Boolean Flags: `Mutex<bool>` → `AtomicBool`

```rust
// Before
let is_shutdown = Arc::new(Mutex::new(false));

// After
let is_shutdown = Arc::new(AtomicBool::new(false));
```

### Read-Heavy Maps: `Mutex<HashMap<K, V>>` → `RwLock` or `dashmap`

For maps with many readers and few writers:
- `RwLock<HashMap<K, V>>` — standard library, no extra deps
- `dashmap::DashMap<K, V>` — if `dashmap` is already a dependency or contention is high

### Option Flags: `Mutex<Option<T>>` → `OnceLock<T>` (if write-once)

```rust
// Before: Mutex for a value set once
let config = Arc::new(Mutex::new(None));

// After: OnceLock for write-once semantics
let config = Arc::new(OnceLock::new());
```

## When NOT to Change

- **Multi-step transactions** — Locks protecting operations that modify multiple fields atomically should stay as locks. Making them lock-free usually requires complex CAS loops.
- **Low contention** — A lock accessed once per request with sub-microsecond hold time isn't worth optimizing.
- **Complex data structures** — If the locked data isn't a simple scalar or map, lock-free alternatives add complexity without proportional benefit.

## Deadlock Risk Analysis

Also flag these patterns:

### Nested Lock Acquisition
```rust
// Danger: potential deadlock if another thread acquires in reverse order
let _guard_a = lock_a.lock();
let _guard_b = lock_b.lock();
```

### Lock Held Across `.await`
```rust
// Bug: std::sync::Mutex held across await → blocks the executor thread
let guard = std_mutex.lock().unwrap();
some_async_fn().await; // BUG: guard is still held
drop(guard);
```

**Fix:** Use `tokio::sync::Mutex` if the guard must be held across await, or restructure to release before awaiting.

### Inconsistent Lock Ordering
If locks A and B are acquired in different orders in different code paths, document the required ordering.

## Output Format

For each lock found:

```markdown
### Lock: `Mutex<u64>` in `src/metrics.rs:42`

**Protects:** Request counter
**Access pattern:** Write-only (increment), hot path
**Hold duration:** Single atomic operation
**Contention risk:** High — called on every request
**Await crossing:** No

**Recommendation:** Replace with `AtomicU64::fetch_add(1, Ordering::Relaxed)`
**Complexity:** Low — drop-in replacement
**Expected gain:** Eliminates lock acquisition overhead on hot path
```

## Execution Steps

1. **Scan** — Find all `Mutex`, `RwLock`, and other sync primitives
2. **Analyze** — Evaluate each lock on the dimensions above
3. **Categorize** — Group into: replace with atomic, replace with concurrent structure, keep as-is, fix (deadlock risk)
4. **Apply** — Make recommended changes (skip "keep as-is" items)
5. **Verify** — Run `cargo check` and `cargo test` after each change
6. **Report** — Summarize findings with before/after for each change
