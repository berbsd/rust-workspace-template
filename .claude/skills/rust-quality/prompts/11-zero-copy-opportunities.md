# Zero-Copy Opportunities

## Prompt

Identify zero-copy opportunities in this code. Analyze data movement patterns, suggest ways to eliminate unnecessary copying, and recommend techniques like slicing, borrowing, or memory mapping where appropriate.

## What to Scan For

### Unnecessary Data Movement

**Collecting into a `Vec` just to iterate:**
```rust
// Before: allocates a Vec just to loop
let items: Vec<_> = source.iter().filter(|x| x.is_valid()).collect();
for item in &items {
    process(item);
}

// After: iterate directly, no allocation
for item in source.iter().filter(|x| x.is_valid()) {
    process(item);
}
```

**Converting between string types unnecessarily:**
```rust
// Before: allocating a new String
let owned: String = borrowed_str.to_string();
log::info!("{owned}");

// After: use the &str directly
log::info!("{borrowed_str}");
```

**Copying bytes that could be sliced:**
```rust
// Before: copy a subrange into a new Vec
let header_bytes = data[..16].to_vec();
parse_header(&header_bytes);

// After: pass a slice reference
parse_header(&data[..16]);
```

### Zero-Copy Techniques

#### 1. Slicing (`&[u8]`, `&str`)

Use slices instead of owned copies when the source outlives the consumer:

```rust
// Before
fn extract_name(input: &str) -> String {
    input[5..input.len() - 1].to_string()
}

// After: return a borrow if caller doesn't need ownership
fn extract_name(input: &str) -> &str {
    &input[5..input.len() - 1]
}
```

#### 2. `Cow<'_, T>` (Copy-on-Write)

Use `Cow` when a function usually borrows but sometimes needs to own:

```rust
use std::borrow::Cow;

fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}
```

#### 3. `Bytes` / `BytesMut` (for network I/O)

If the project uses `bytes` crate, prefer `Bytes` over `Vec<u8>` for shared buffer access:

```rust
// Before: clone the buffer for each handler
let data: Vec<u8> = buffer.clone();

// After: cheap reference-counted slice
let data: Bytes = buffer.slice(..);
```

#### 4. Reader/Writer Patterns

Stream data instead of buffering entirely in memory:

```rust
// Before: read entire file into String
let content = std::fs::read_to_string("large.json")?;
let parsed: Value = serde_json::from_str(&content)?;

// After: stream from reader
let file = std::fs::File::open("large.json")?;
let reader = std::io::BufReader::new(file);
let parsed: Value = serde_json::from_reader(reader)?;
```

#### 5. Memory Mapping (for large files)

If the project processes large files, `memmap2` can avoid reading into user-space buffers:

```rust
// Only suggest if memmap2 is already a dependency or files are large enough to justify it
let file = std::fs::File::open("data.bin")?;
let mmap = unsafe { memmap2::Mmap::map(&file)? };
let header = parse_header(&mmap[..64]);
```

#### 6. `impl AsRef<T>` / `impl Into<T>` for API Boundaries

Accept the most general type to let callers avoid conversions:

```rust
// Before: forces String allocation
fn lookup(key: String) -> Option<Value> { ... }

// After: accepts &str, String, Cow, etc.
fn lookup(key: impl AsRef<str>) -> Option<Value> {
    let key = key.as_ref();
    ...
}
```

#### 7. Borrow When Deserializing a `serde_json::Value`

`serde_json::from_value(v)` consumes `v`. When you hold only `&Value`,
`from_value(v.clone())` deep-clones the entire tree just to parse it.
Use borrowing deserialize — serde_json implements `Deserializer` for
`&Value`:

```rust
// Before: clones the whole payload tree, then extracts a few fields
let event: MyEvent = serde_json::from_value(payload.clone())?;

// After: borrows; clones only the fields MyEvent owns
use serde::Deserialize;
let event = MyEvent::deserialize(payload)?;
```

Same error type (`serde_json::Error`), so `?` propagation is unchanged.
Highest-value when the target struct is much smaller than the payload
(e.g. an enum of Copy typed-ids parsed from a rich event body), and in
any dispatch/ingest match arm where the anti-pattern copy-pastes across
every arm.

#### 8. Serialize Once, at the Wire/Storage Boundary

`serde_json::to_value(x)` allocates a full `Value` tree. Running it on
something that is already a `Value`, or serializing an intermediate
`Value` only to re-serialize it for the database or wire, does the work
2–3×. Carry the typed value generically to the boundary and serialize
exactly once:

```rust
// Before: event -> Value -> envelope Value -> to_value (identity deep
// clone) -> sqlx re-encodes to jsonb. Three trees for one row.
let payload = serde_json::to_value(event)?;
let envelope_value = serde_json::to_value(Envelope::new(payload))?;
enqueue(&serde_json::to_value(&envelope_value)?).await?;

// After: generic to the bind; one serialization, no trees
let envelope = Envelope::new(&event); // Envelope<&E>, E: Serialize
query.bind(sqlx::types::Json(&envelope)).execute(tx).await?;
```

Flag any path where a payload is `to_value`'d more than once before
leaving the process. Serialize in the repository (not inside the sqlx
bind) when the caller's error taxonomy distinguishes serialization
failures from database failures. Note: byte-stability of the *stored*
form only holds on `jsonb` columns (Postgres normalizes key order);
plain `json` columns preserve the textual difference between a
`Value`-tree path (alphabetical keys) and direct struct serialization
(declaration order).

### Patterns to Flag

| Pattern | Zero-Copy Alternative |
|---------|----------------------|
| `.to_vec()` on a `&[u8]` only used for reading | Pass the slice |
| `.to_string()` on `&str` only used for display | Use `&str` directly |
| `collect::<Vec<_>>()` followed by single iteration | Chain iterators |
| `read_to_string` + `from_str` | `from_reader` |
| `Vec<u8>` clone for shared access | `Bytes::from` / `Arc<[u8]>` |
| Repeated `format!` building the same string | Cache or use `Cow` |
| `String` concatenation in a loop | `String::with_capacity` + `push_str` |
| Returning `String` from a function that always borrows input | Return `&str` with lifetime |
| `serde_json::from_value(v.clone())` on a `&Value` | `T::deserialize(&v)` — borrowing deserialize |
| `to_value` on an already-`Value`, or a payload `to_value`'d more than once | Serialize once at the bind/wire boundary; keep the type generic |

## Output Format

```markdown
## Zero-Copy Analysis Report

### Summary
- Unnecessary copies found: N
- Estimated allocations eliminable: N
- Techniques applicable: [slicing, Cow, Bytes, streaming, ...]

### Findings

#### High Impact (hot paths / large data)

| File:Line | Pattern | Suggested Fix | Impact |
|-----------|---------|---------------|--------|
| `src/parser.rs:120` | `to_vec()` on 4KB slice | Pass `&[u8]` | Eliminates 4KB alloc per parse |

#### Medium Impact (frequent but small)

| File:Line | Pattern | Suggested Fix | Impact |
|-----------|---------|---------------|--------|
| `src/handler.rs:55` | `to_string()` for logging | Use `&str` in format | Eliminates small alloc per request |

#### Low Impact / Style Improvements

| File:Line | Pattern | Suggested Fix |
|-----------|---------|---------------|
| `src/config.rs:12` | `collect` then iterate | Chain iterators |
```

## Execution Steps

1. **Scan** — Find `.to_vec()`, `.to_string()`, `.to_owned()`, `.collect()`, `clone()` on buffers, `read_to_string`, string concatenation in loops
2. **Trace data flow** — Determine if the owned value is necessary or if a borrow/slice suffices
3. **Prioritize** — Rank by: hot path frequency, data size, allocation count
4. **Report** — Present findings grouped by impact
5. **Fix** — Apply changes (high and medium impact first, with user approval)
6. **Verify** — `cargo check` and `cargo test` after changes

## When NOT to Optimize

- **Small, infrequent copies** — Cloning a 20-byte struct once at startup isn't worth a lifetime parameter
- **Readability trade-off** — If zero-copy requires complex lifetime annotations that obscure the logic, the copy may be the better choice
- **Unsafe memory mapping** — Only suggest `mmap` for genuinely large files where the `unsafe` is justified
- **API stability** — Changing a public function from `-> String` to `-> &str` is a breaking change in libraries
