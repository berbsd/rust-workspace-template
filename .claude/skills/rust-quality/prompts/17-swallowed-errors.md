# Swallowed Errors

Find places where a `Result` (or `Option` standing in for one) is silently discarded, and fix them. Errors that don't surface in logs *or* propagate to the caller are the leading cause of "everything looked fine until production fell over."

## Workflow

Scan all `*.rs` files in `crates/` and `services/`. Skip `target/` and generated code. For each finding, pick one of the four fixes in the "Fixing" section — never silently delete the warning.

## Patterns to flag

### 1. `let _ = <Result-typed expression>`

```rust
// Bad — error path is invisible
let _ = repo.delete_item(id).await;

// Bad in test code is still bad — use ?, the workspace lints already deny .unwrap()
```

Acceptable only when paired with a comment explaining *why* the failure is fine to ignore *and* with a typed binding to make intent obvious:

```rust
// Best-effort: cache miss is expected when the entry hasn't been seen
// before. Don't propagate, don't log — the next access will repopulate.
let _: Result<(), CacheError> = self.cache.invalidate(key).await;
```

Without the comment + typed binding, flag.

### 2. `.ok()` on `Result` without using the resulting `Option`

```rust
// Bad — converts to Option then drops it
self.audit_log(event).await.ok();

// Bad — same shape, slightly different syntax
let _ = self.audit_log(event).await.ok();
```

Flag any `.ok()` whose result is not bound, matched on, or piped into another expression. The legitimate use of `.ok()` is converting `Result<T, E>` to `Option<T>` when the caller only cares about the success value — it must be consumed.

### 3. `.err()` on `Result` without acting on it

Same pattern as `.ok()`, opposite end. Flag `.err()` whose result is not bound.

### 4. `.map_err(|_| ...)` that throws the error away

```rust
// Bad — original error is gone forever
let user = repo.find(id).await.map_err(|_| ServiceError::NotFound)?;
```

The discarded error often contains the *real* failure (DB unreachable, query timeout, wrong column type) which is now invisible in logs. Two fixes:

- Use `#[from]` or named conversion in the error enum so `?` carries the source.
- If transformation is genuinely needed, log the original first or wrap it: `.map_err(|e| { tracing::warn!(error = %e, "find failed"); ServiceError::Database(e) })?`.

Variants: `.map_err(|_e| ...)`, `.map_err(|err| { /* err unused */ ... })`. Flag any closure that binds the error and never references it.

### 5. `Err(_) => {}` and `Err(_) => continue`

```rust
// Bad — every kind of failure looks the same
match repo.find(id).await {
    Ok(item) => process(item),
    Err(_) => {}
}
```

Same logic as `.map_err(|_| ...)`. Flag empty `Err(_)` arms, `Err(_) => continue`, and `Err(_) => return Default::default()` patterns. The fix is usually `?` propagation; rarely a logged-and-continue with a typed binding.

### 6. `if let Ok(value) = expr { ... }` with no `else`

```rust
// Bad — what if expr was Err?
if let Ok(membership) = self.require_membership(...).await {
    do_something(membership);
}
```

Flag `if let Ok(...)` chains that have no `else` branch *and* whose body has side effects. The bug is usually that the caller meant to bail on `Err` and forgot. Fix with `let value = expr?;` or an explicit `else { return Err(...); }`.

`while let Ok(...)` is the loop variant — same problem when the loop produces side effects.

### 7. `.unwrap_or_default()` / `.unwrap_or(...)` on `Result`

```rust
// Bad — silently substitutes a default for any failure
let count = repo.count_items().await.unwrap_or(0);
```

`.unwrap_or_default()` and `.unwrap_or(value)` on `Result` types convert *any* error into a fallback value. Often the intended behaviour is "fall back only on a specific error variant"; instead the code falls back on connection failures, timeouts, schema errors, and everything else — which silently masks outages. Flag, suggest matching on the specific variant or propagating.

(`.unwrap_or_default()` on `Option` is fine — `None` carries no error info to lose.)

### 8. `tokio::spawn(...)` whose `JoinHandle` is dropped

```rust
// Bad — task panic / error is lost; cancellation on drop may also bite
tokio::spawn(async move { do_work(state).await });
```

Flag `tokio::spawn` calls whose `JoinHandle` is not stored, awaited, or sent into a task tracker. The fix is one of: `let _: JoinHandle<()> = tokio::spawn(...)` *with* a comment explaining that the task is fire-and-forget by design, or store the handle in a `JoinSet` / task tracker so failures surface.

### 9. `tracing::error!` / `warn!` followed by silent continuation

```rust
// Bad — logged, then carry on as if nothing happened
if let Err(e) = self.publish(event).await {
    error!(error = %e, "publish failed");
}
// ... function returns Ok(()) below
```

The error is in logs but not in the response. If publishing is best-effort, that's correct *and* should be commented as such. If it isn't, the function should return the error (or at least mark the operation as partially failed in the response). Flag `error!` / `warn!` calls that aren't followed by `return Err(...)`, `?`, or a clear "best-effort" comment.

**Never-best-effort subclass — an `outbox.enqueue(&mut tx, …)` inside an open transaction that then commits.** Only applies to a workspace with a transactional-outbox pattern; skip if this one has none. This is the highest-severity instance of #9: the swallow doesn't just hide a failure, it commits the state change *without* its event, silently and unrecoverably. The whole point of the transactional outbox is that the row and its event land atomically — a logged-and-committed enqueue breaks cross-service consistency (the consumer's rollup/projection never fires and nothing reconciles it). For an enqueue inside a `tx` that commits, best-effort is **never** correct: always propagate with `?` so a failed enqueue rolls the transaction back. Fix #2 (log-and-return-Ok) does not apply here.

## Fixing

Pick exactly one of:

1. **Propagate.** Replace the swallowing pattern with `?`. Add `.context("...")` (anyhow) or a typed `From` conversion if the error type needs a transform.
2. **Log and return.** If the caller can't propagate, log via `tracing::warn!`/`error!` *and* return a typed error or `()` with a clear semantics in the doc comment.
3. **Bind and ignore intentionally.** Use `let _: Result<T, E> = expr` *with a comment* explaining why ignoring is correct. The typed annotation forces the next reader to acknowledge the discard.
4. **Replace with a typed fallback.** If "default on failure" is the actual intent, match on the specific error variant — never `.unwrap_or_default()` over arbitrary errors.

## Verification

After fixes:

```bash
cargo check --workspace
cargo clippy --workspace --no-deps --all-targets
cargo nextest run --workspace --all-features
```

If a fix changes a function signature (e.g. now returns `Result<T, E>` where it was returning `T`), update callers in the same pass. Don't ship a half-converted function whose error type widens silently.

## Report format

Group by file. For each finding give file:line, the offending snippet (one line), the rule violated (#1–#9), and the chosen fix. End with a per-service count so the user can see where the swallowing tends to cluster.
