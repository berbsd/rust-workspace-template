# Flaky Test Detection and Remediation

## Prompt

Analyze the test suite for potentially flaky tests and fix them in a reliable way. Look for timing dependencies, shared mutable state, filesystem operations, non-deterministic ordering, and any test that could intermittently fail depending on execution environment or timing.

## What to Scan For

### Timing Dependencies
- Tests using `Instant::now()`, `SystemTime::now()`, or `sleep()` for synchronization
- Assertions on elapsed time (fragile on slow CI machines)
- Hardcoded `Duration` values in timeout assertions

**Fix:** Replace timing assertions with event-based synchronization (channels, callbacks, condition variables).

```rust
// Flaky: depends on timing
#[test]
fn test_timeout_handler() {
    let start = Instant::now();
    trigger_timeout(Duration::from_millis(100));
    assert!(start.elapsed() < Duration::from_millis(150)); // races on slow CI
}

// Fixed: event-based synchronization
#[test]
fn test_timeout_handler() {
    let (tx, rx) = channel();
    trigger_timeout_with_callback(Duration::from_millis(100), move || {
        tx.send(()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(5)).expect("timeout callback not fired");
}
```

### Shared Mutable State
- `static mut` or `lazy_static!` / `OnceLock` shared across tests
- Tests that modify global state without isolation
- Tests that depend on execution order

**Fix:** Use per-test state, `#[serial_test::serial]` for tests that genuinely need shared state, or restructure to avoid sharing.

### Filesystem Operations
- Tests creating files at hardcoded paths (race with parallel tests)
- Tests that don't clean up temp files
- Tests assuming specific working directory

**Fix:** Use `tempfile::tempdir()` for all filesystem operations. Paths should be unique per test invocation.

### Network/Port Dependencies
- Tests binding to hardcoded ports (e.g., `127.0.0.1:8080`)
- Tests assuming network availability
- Tests that don't wait for server readiness

**Fix:** Use port `0` for OS-assigned ports. Use readiness channels or health check loops instead of `sleep`.

### Environment Variables
- Tests reading `std::env::var()` without isolation
- Tests setting env vars without restoring them (affects parallel tests)

**Fix:** Use `temp_env::with_vars()` or similar isolation. Never set env vars globally in tests.

### Non-deterministic Ordering
- Tests iterating `HashMap`/`HashSet` and asserting order
- Tests depending on thread scheduling order
- Tests using `rand` without fixed seeds

**Fix:** Sort before comparing, use `BTreeMap`/`BTreeSet`, seed RNGs deterministically.

## Execution Steps

1. **Scan** — Search for the patterns above across all `#[test]` and `#[tokio::test]` functions
2. **Categorize** — Group findings by flakiness type (timing, state, filesystem, network, env, ordering)
3. **Fix** — Apply the appropriate fix for each finding
4. **Verify** — Run `cargo test` to ensure all tests still pass
5. **Report** — List each fix with before/after and explanation of what made it flaky
