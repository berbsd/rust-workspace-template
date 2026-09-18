# Clone Analysis and Reduction

## Prompt

Analyze `clone()` usage in this code and make a report explaining the best course of action. Identify unnecessary clones, suggest borrowing alternatives, and highlight where clones are actually necessary.

## What to Scan For

### Unnecessary Clones

**Clone before immutable use:**
```rust
// Before: clones just to read
let name = config.name.clone();
println!("{name}");

// After: borrow instead
println!("{}", config.name);
```

**Clone into a function that only needs a reference:**
```rust
// Before: clone to pass owned value
fn process(data: String) { /* only reads data */ }
process(input.clone());

// After: accept a reference
fn process(data: &str) { /* only reads data */ }
process(&input);
```

**Clone inside a loop when a borrow suffices:**
```rust
// Before: clone on every iteration
for item in &items {
    let key = item.key.clone();
    map.contains_key(&key);
}

// After: borrow directly
for item in &items {
    map.contains_key(&item.key);
}
```

**Clone of `Arc`/`Rc` contents instead of cloning the handle:**
```rust
// Before: clone the inner data
let data = (*shared_state.lock().unwrap()).clone();

// After: clone the Arc and access later (if you need ownership of the handle)
let handle = Arc::clone(&shared_state);
```

**Clone to satisfy a moved closure when borrowing works:**
```rust
// Before: clone for closure
let config = config.clone();
let handle = tokio::spawn(async move { use_config(&config) });

// After: borrow if the lifetime allows, or use Arc
let config = Arc::new(config);
let config = Arc::clone(&config);
let handle = tokio::spawn(async move { use_config(&config) });
```

### Clones That Are Necessary

Mark these as **justified** in the report:

- **Crossing thread/task boundaries** — Data sent to `tokio::spawn`, `std::thread::spawn`, or channels needs ownership. `Arc::clone()` is cheap and correct.
- **Interior mutability** — Cloning before mutation to avoid borrow conflicts is sometimes the simplest correct approach.
- **API requirements** — External APIs that take owned values (e.g., `HashMap::insert` keys).
- **Copy-on-write patterns** — `Cow::into_owned()` when mutation is needed.
- **Small types** — Cloning a `u64`, small enum, or short `String` is often cheaper than the indirection of a reference.

### Patterns to Flag

| Pattern | Likely Fix |
|---------|-----------|
| `.clone()` immediately before `&` borrow | Remove clone, use reference |
| `.to_string()` on `&str` passed to `format!` | Use `&str` directly |
| `.clone()` on `Copy` types | Remove (implicit copy) |
| `String::from(x)` where `&str` suffices | Use `&str` / accept `impl AsRef<str>` |
| `.clone()` in hot loop | Hoist or borrow |
| `Vec<T>` cloned to iterate | Borrow `&[T]` |
| `.to_vec()` on a slice only used for reading | Borrow the slice |

## Output Format

```markdown
## Clone Analysis Report

### Summary
- Total `.clone()` calls found: N
- Unnecessary (removable): N
- Optimizable (cheaper alternative): N
- Justified (necessary): N

### Unnecessary Clones

| File:Line | Expression | Suggested Fix | Reason |
|-----------|-----------|---------------|--------|
| `src/handler.rs:42` | `name.clone()` | Borrow `&name` | Only used in `format!` |

### Optimizable Clones

| File:Line | Expression | Suggested Fix | Reason |
|-----------|-----------|---------------|--------|
| `src/service.rs:88` | `data.clone()` in loop | Hoist above loop | Same clone every iteration |

### Justified Clones

| File:Line | Expression | Reason |
|-----------|-----------|--------|
| `src/worker.rs:15` | `Arc::clone(&state)` | Sent to spawned task |
```

## Execution Steps

1. **Scan** — Find all `.clone()`, `.to_owned()`, `.to_string()`, `.to_vec()` calls
2. **Trace usage** — For each clone, determine how the cloned value is used downstream
3. **Categorize** — Unnecessary, optimizable, or justified
4. **Report** — Present findings with the format above
5. **Fix** — Apply changes for unnecessary and optimizable clones (with user approval)
6. **Verify** — `cargo check` and `cargo test` after changes

## Function Signature Improvements

Beyond individual clone sites, look for functions whose signatures force callers to clone:

```rust
// Before: forces callers to own a String
fn register(name: String) { /* only reads name */ }

// After: accepts borrowed or owned
fn register(name: &str) { /* only reads name */ }
// or for flexibility:
fn register(name: impl Into<String>) { /* stores name */ }
```

Flag functions that take `String`, `Vec<T>`, or other owned types when they could accept `&str`, `&[T]`, or `impl AsRef<T>`.
