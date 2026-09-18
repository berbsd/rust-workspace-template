# Async & Unsafe Code: Documentation Decision Guide

When to document async and unsafe patterns in Rust — and when to trust the reader.

## Decision Framework

For each construct, ask:

1. **Is the lifetime surprising?** — Does the task outlive its obvious scope? Is cancellation non-standard?
2. **Is the sizing deliberate?** — Does the buffer, pool, or semaphore bound reflect a specific constraint?
3. **Is the cleanup non-trivial?** — Does resource release have ordering requirements or deadlock potential?
4. **Would a bug here be subtle?** — Could a reader accidentally break a safety invariant?

If **yes** to any → document that specific aspect. If **no to all** → idiomatic, skip it.

---

## Spawned Tasks

**Document when:**
- Task is detached and outlives the scope that created it
- Cancellation requires coordination (shutdown signal, `CancellationToken`, channel)
- Error from the task has non-obvious consequences (silently swallowed, panics the runtime)
- Task holds resources that need explicit cleanup
- Task has ordering dependencies with other tasks

**Skip when:**
- Task is `.await`ed in the same scope
- Standard fire-and-forget with no resource ownership
- Runtime manages the lifecycle (e.g., axum request handlers)

```rust
// Document: surprising lifetime + cleanup
// Detached: survives the request scope. Shut down via `shutdown_rx`.
// Errors are logged but not propagated to the caller.
let handle = tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            item = queue.recv() => process(item).await,
        }
    }
});

// Skip: awaited immediately, obvious scope
let result = tokio::spawn(async { compute(data).await }).await?;
```

---

## Channels (`mpsc`, `oneshot`, `broadcast`, `watch`)

**Document when:**
- Channel type choice has correctness implications (bounded vs unbounded, broadcast replay semantics)
- Bound reflects a specific system constraint (memory budget, backpressure target)
- Multiple producer groups with different priorities
- Close/drop behavior affects correctness
- Receiver is expected to handle lag (broadcast) or missed values (watch)

**Skip when:**
- Standard bounded channel with a reasonable default capacity
- Single producer, single consumer with obvious lifetime
- Using oneshot for simple request-response

```rust
// Document: bound is load-bearing
// Bounded to 1000: matches the upstream API's max batch size.
// Exceeding this causes backpressure on the HTTP handler, which
// returns 503 to the client.
let (tx, rx) = mpsc::channel(1000);

// Document: broadcast semantics matter
// Broadcast: multiple consumers need every config change.
// New subscribers get the current value via `watch` — this channel
// is only for change notifications.
let (tx, _) = broadcast::channel(16);

// Skip: standard oneshot, obvious usage
let (tx, rx) = oneshot::channel();
```

---

## Synchronization (`Mutex`, `RwLock`, `Semaphore`)

**Document when:**
- Lock ordering matters for deadlock prevention
- Using `tokio::sync::Mutex` intentionally (held across `.await`)
- Lock is held across an await point and this is deliberate
- Custom synchronization protocol (barriers, condition variables)
- Semaphore permits reflect a specific system limit

**Skip when:**
- Single `std::sync::Mutex` protecting shared state with obvious scope
- Standard `RwLock` for read-heavy, write-rare patterns
- Lock guard lifetime is clear from the block structure

```rust
// Document: deliberate choice of tokio Mutex
// tokio::sync::Mutex because the guard is held across the
// `client.send().await` call. std::sync::Mutex would deadlock.
let state = tokio::sync::Mutex::new(State::default());

// Document: semaphore bound is load-bearing
// 10 permits: matches the database connection pool size.
// More permits would exceed max_connections and cause errors.
let semaphore = Semaphore::new(10);

// Skip: obvious scope, no await crossing
let counter = std::sync::Mutex::new(0u64);
```

---

## `tokio::select!` and Cancellation

**Document when:**
- Branch cancellation has side effects (partial writes, resource leaks)
- Branch ordering matters (first match wins for simultaneous readiness)
- Biased select is used intentionally
- Cancellation safety of the futures is non-obvious

**Skip when:**
- Standard shutdown pattern (`ctrl_c` + `shutdown_signal`)
- Simple timeout wrapper
- All branches are cancellation-safe

```rust
// Document: cancellation safety concern
// `reader.read_frame()` is cancellation-safe — it uses an internal
// buffer and resumes correctly. If replaced with a non-cancellation-safe
// future, data loss occurs on the timeout branch.
tokio::select! {
    frame = reader.read_frame() => handle(frame),
    _ = tokio::time::sleep(timeout) => return Err(Error::Timeout),
}

// Document: biased is intentional
// Biased: prioritize shutdown over processing to prevent
// draining a full queue before stopping.
tokio::select! {
    biased;
    _ = shutdown.recv() => break,
    item = queue.recv() => process(item).await,
}
```

---

## `unsafe` Blocks

**Always document.** Every `unsafe` block gets a `// SAFETY:` comment. No exceptions.

The comment must explain why the safety invariants are upheld *at this specific call site*, not just restate what the unsafe function requires.

```rust
// Good: explains why invariants hold HERE
// SAFETY: `buf` was allocated by `Vec::with_capacity(n)` on the line
// above with the same allocator. `len` is 0 (no initialized elements
// to drop), and `n` is the capacity we just allocated.
let vec = unsafe { Vec::from_raw_parts(buf, 0, n) };

// Bad: restates the function's Safety docs
// SAFETY: ptr must be valid and properly aligned.
let val = unsafe { ptr.read() };

// Good: specific to this call site
// SAFETY: `self.data` is a valid, aligned pointer to `Header` because
// it was obtained from `Box::into_raw` in `new()`, and the struct
// guarantees exclusive ownership (no aliasing). The pointer has not
// been freed because `drop()` is the only place that calls `Box::from_raw`.
let header = unsafe { &*self.data };
```

---

## `unsafe` Traits and Implementations

**Document on the trait declaration:**
- What invariants implementors must uphold
- What undefined behavior results from incorrect implementation

**Document on `unsafe impl`:**
- Why this specific type satisfies the trait's invariants

```rust
/// Marker for types that can be safely zero-initialized.
///
/// # Safety
///
/// Implementors must ensure that an all-zeroes bit pattern is a valid
/// value for the type. Implementing this for types with validity
/// invariants (e.g., `NonZeroU32`, `bool`, references) is undefined
/// behavior.
pub unsafe trait ZeroInit {}

// SAFETY: `u32` has no validity invariants — all bit patterns,
// including all-zeroes, are valid.
unsafe impl ZeroInit for u32 {}
```

---

## FFI Boundaries

**Always document thoroughly.** FFI is where Rust's safety guarantees end.

```rust
/// Reads up to `len` bytes from the device into `buf`.
///
/// # Safety
///
/// - `buf` must point to a valid, writable buffer of at least `len` bytes.
/// - `buf` must remain valid for the duration of the call (no concurrent
///   mutation or deallocation).
/// - The caller must not pass a `len` larger than the actual buffer size.
/// - `device_fd` must be a valid, open file descriptor obtained from
///   [`open_device`].
///
/// # Errors
///
/// Returns a negative errno value on failure. Common errors:
/// - `-EINVAL` — invalid file descriptor or zero-length buffer.
/// - `-EIO` — device communication error.
pub unsafe fn device_read(device_fd: i32, buf: *mut u8, len: usize) -> i32 {
    // ...
}
```

---

## `Drop` Implementations

**Document when:**
- Drop does async work or spawns cleanup tasks
- Drop has ordering requirements relative to other resources
- Dropping in certain contexts causes issues (inside `spawn_blocking` → deadlock)
- Drop has a timeout or fallback behavior
- Drop does non-trivial work (flushes buffers, sends shutdown signals)

**Skip when:**
- Drop just releases memory (standard RAII)
- Drop calls `.close()` or `.cancel()` on one obvious resource

```rust
/// Flushes pending writes and closes the underlying connection.
///
/// If the flush fails, pending data is silently dropped. Use
/// [`Connection::shutdown`] for graceful shutdown with error reporting.
///
/// # Warning
///
/// Dropping inside `spawn_blocking` deadlocks because the flush
/// spawns a blocking task internally.
impl Drop for Connection {
    fn drop(&mut self) {
        // ...
    }
}
```

---

## Lifetimes and Borrowing

**Document when:**
- Lifetime relationship between parameters and return value is non-obvious
- Struct stores a reference with implications for the owner's lifetime
- Self-referential patterns or `Pin` requirements
- Lifetime bounds on trait objects affect usability

**Skip when:**
- Lifetime is elided and obvious from context
- Standard `&self` → `&T` borrow pass-through
- Single-lifetime structs where the borrow is obvious

```rust
/// Returns a reference to the value associated with `key`.
///
/// The returned reference borrows from `self` — the map must not be
/// modified while the reference is live. For an owned value, use
/// [`get_owned`](Self::get_owned).
pub fn get(&self, key: &str) -> Option<&V> { /* ... */ }

/// An iterator that borrows from the source `[u8]` slice.
///
/// The source slice must outlive this iterator. Consuming the
/// iterator into a `Vec<&[u8]>` extends the borrow.
pub struct ChunkIter<'a> {
    source: &'a [u8],
    chunk_size: usize,
}
```

---

## Pin and Futures

**Document when:**
- Type must be `Unpin` or explicitly `!Unpin`
- `Pin` is required for correctness (self-referential futures)
- Manual `Future` or `Stream` implementation with pin projection

**Skip when:**
- Using `async fn` (compiler handles pinning)
- Standard `Box::pin` usage
- `pin_mut!` macro with obvious scope

```rust
/// A future that resolves when all inner futures complete.
///
/// This future is `!Unpin` because it stores the inner futures inline.
/// Use `Box::pin(join_all(futures))` if you need to move it after creation.
pub struct JoinAll<F: Future> {
    // ...
}
```
