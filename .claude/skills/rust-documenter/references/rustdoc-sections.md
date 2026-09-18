# Standard Rustdoc Sections Reference

Detailed guidance on the four standard doc comment sections per RFC 1574 and the Rust API Guidelines.

## Section Order

When multiple sections apply, use this order:

```rust
/// Summary line.
///
/// Extended description (optional).
///
/// # Examples
///
/// # Errors
///
/// # Panics
///
/// # Safety
```

---

## `# Examples`

**When:** All public items that can be meaningfully demonstrated. Every public function, method, struct, enum, trait, and macro should have at least one example.

**Format:**
- Use the plural "Examples" even for a single example.
- Examples are compiled and run as doc tests by `cargo test --doc`.
- Use `?` instead of `unwrap()` for error handling.
- Hide boilerplate with `#` prefix lines (imports, `fn main`, error handling).
- Use `no_run` for examples requiring external resources (network, files, databases).
- Use `should_panic` for examples that demonstrate panic behavior.
- Use `compile_fail` to show what does NOT compile (useful for demonstrating type safety).

**Hiding boilerplate:**

```rust
/// # Examples
///
/// ```
/// # use my_crate::Config;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_file("app.toml")?;
/// assert!(config.is_valid());
/// # Ok(())
/// # }
/// ```
```

**Showing error cases:**

```rust
/// # Examples
///
/// ```
/// # use my_crate::parse_port;
/// assert_eq!(parse_port("8080")?, 8080);
///
/// // Out-of-range values return an error:
/// assert!(parse_port("99999").is_err());
/// # Ok::<(), my_crate::Error>(())
/// ```
```

**No-run examples (external dependencies):**

```rust
/// # Examples
///
/// ```no_run
/// # use my_crate::Client;
/// # async fn run() -> Result<(), my_crate::Error> {
/// let client = Client::connect("https://api.example.com").await?;
/// let data = client.fetch("users").await?;
/// # Ok(())
/// # }
/// ```
```

---

## `# Errors`

**When:** Every function, method, or associated function that returns `Result<T, E>`. Also document trait methods that are expected or allowed to return errors.

**Format:**
- List each error variant that can be returned.
- Explain *when* each error occurs — the condition that triggers it.
- Use intra-doc links to error types: `[`Error::NotFound`]`.
- If the error type is an enum, list the relevant variants.
- If the function wraps another fallible call, document the propagated error.

**Single error type:**

```rust
/// # Errors
///
/// Returns [`Error::InvalidUrl`] if `url` cannot be parsed as a valid URL.
```

**Multiple variants:**

```rust
/// # Errors
///
/// - [`Error::NotFound`] — the resource does not exist.
/// - [`Error::Auth`] — the API key is invalid or expired.
/// - [`Error::RateLimit`] — request rate exceeded; includes retry-after duration.
/// - [`Error::Network`] — connection failed after exhausting retries.
```

**Propagated errors:**

```rust
/// # Errors
///
/// Returns [`std::io::Error`] if the file cannot be read.
/// Returns [`serde_json::Error`] if the file contents are not valid JSON.
```

---

## `# Panics`

**When:** Every function that can panic under any condition. This includes direct `panic!()`, `unwrap()`, `expect()`, `assert!()`, array indexing without bounds checking, and integer overflow in debug mode.

**Format:**
- List each condition that causes a panic.
- Help callers avoid triggering the panic.
- If a panic indicates a bug (not a user error), say so.

**Examples:**

```rust
/// # Panics
///
/// Panics if `index` is out of bounds (`index >= self.len()`).
```

```rust
/// # Panics
///
/// Panics if called from outside a Tokio runtime context.
```

```rust
/// # Panics
///
/// Panics if the lock is poisoned. This indicates a bug — a thread
/// panicked while holding the lock.
```

---

## `# Safety`

**When:** Every `unsafe fn`. This section is mandatory — Clippy's `missing_safety_doc` lint enforces it.

**Format:**
- List every invariant the caller must uphold.
- Be specific about what constitutes undefined behavior if invariants are violated.
- Reference relevant safety documentation from the standard library when wrapping unsafe std functions.

**Example:**

```rust
/// Creates a `Vec<T>` from its raw components.
///
/// # Safety
///
/// - `ptr` must have been allocated by the same allocator that backs `Vec`
///   (typically the global allocator).
/// - `T` must have the same size and alignment as the type used during
///   allocation.
/// - `length` must be less than or equal to `capacity`.
/// - The first `length` values must be properly initialized values of type `T`.
/// - `capacity` must be the capacity the pointer was allocated with.
/// - The caller must ensure the pointer is not used after calling this function.
pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Vec<T>
```

---

## Attributes Related to Documentation

### `#[must_use]`

Include a message explaining why the return value matters:

```rust
#[must_use = "the connection is not established until `.connect()` is called"]
pub fn builder() -> ConnectionBuilder { /* ... */ }

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub fn filter_active(users: &[User]) -> impl Iterator<Item = &User> { /* ... */ }
```

### `#[deprecated]`

Use the full form with `since` and `note`:

```rust
#[deprecated(since = "0.5.0", note = "Use `Client::new_with_config` instead")]
pub fn new(url: &str) -> Self { /* ... */ }
```

Optionally add a `# Deprecated` section in the doc comment with migration details.

### `#[doc(hidden)]`

Use for items that are public for technical reasons but not part of the intended API:

```rust
/// This is public for macro expansion but is not a stable API.
#[doc(hidden)]
pub fn __internal_helper() { /* ... */ }
```

### `#[doc(alias = "...")]`

Add search aliases for items known by multiple names:

```rust
#[doc(alias = "delete")]
#[doc(alias = "erase")]
pub fn remove(&mut self, key: &str) -> Option<Value> { /* ... */ }
```

### Feature-gated items

Mark feature-gated items so docs.rs shows the required feature:

```rust
#[cfg(feature = "compression")]
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
pub fn compress(data: &[u8]) -> Vec<u8> { /* ... */ }
```

Requires crate-level: `#![cfg_attr(docsrs, feature(doc_cfg))]`

And in `Cargo.toml`:
```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

---

## Linking Conventions

Always use intra-doc links instead of plain text or raw URLs:

```rust
/// Returns [`None`] if the key is not present.
/// See [`HashMap::get`] for the non-panicking version.
/// Uses [`std::io::Error`] for I/O failures.
```

Disambiguation when needed:

```rust
/// See [`MyType`](struct@MyType) (the struct, not the macro).
/// See [`my_fn`](fn@my_fn) (the function, not the module).
```
