# Crate Evaluation for Your Codebase

## Prompt

Look at this crate: [crate name/URL]. Analyze whether it would be useful for our codebase, either for production code or testing. Evaluate its API quality, maintenance status, dependency footprint, and how it compares to what we currently use or could build ourselves.

## Usage

This check requires a crate name or URL from the user. Ask for one if not provided.

## Evaluation Dimensions

### 1. Maintenance Status
- **Last release date** — When was the latest version published?
- **Commit frequency** — Is the repo actively maintained?
- **Open issues** — How many? Are they being triaged?
- **Bus factor** — How many active maintainers?
- **Rust edition** — Does it support Rust 2021/2024?

### 2. Dependency Footprint
- **Direct dependencies** — How many?
- **Transitive dependencies** — Total dependency tree size
- **Heavy dependencies** — Does it pull in C build dependencies, proc macros, or large frameworks?
- **Feature flags** — Can the dependency tree be trimmed with features?

### 3. Compatibility with Our Project
- **Runtime compatibility** — If we use `tokio`, does this crate use `async-std` or vice versa?
- **`unsafe` usage** — Does the crate use `unsafe`? Is it audited? Does our project avoid `unsafe`?
- **`no_std` support** — If we need it, does the crate support it?
- **MSRV** — Does its minimum supported Rust version conflict with ours?
- **License** — Is the license compatible with our project's license?

### 4. API Quality
- **Documentation** — Are docs comprehensive with examples?
- **Type safety** — Does it use the type system well (newtypes, enums) or rely on stringly-typed APIs?
- **Error handling** — Does it use proper error types or string errors?
- **Ergonomics** — Is the API intuitive? Does it follow Rust conventions?

### 5. Overlap with Existing Dependencies
- **Already covered** — Does an existing dependency provide this functionality?
- **Feature flag** — Could enabling a feature on an existing dep give us what we need?
- **Build vs buy** — How much effort would it be to implement the needed functionality ourselves?

### 6. Testing Crate Evaluation (if applicable)
- **Ergonomics** — How much boilerplate does it save?
- **Debuggability** — Are test failures easy to understand?
- **Maintenance burden** — Will upgrading this crate break our tests?
- **Coverage** — Does it test things our current approach misses?

## Output Format

```markdown
## Crate Evaluation: `<crate-name>` v<version>

### Summary
[1-2 sentence verdict]

### Recommendation: [USE / CONSIDER / AVOID]

### Fitness for Our Project
- Runtime: [compatible/incompatible] — [details]
- License: [compatible/incompatible] — [our license] vs [crate license]
- Unsafe: [none/minimal/extensive] — [details]
- MSRV: [compatible/incompatible] — [our MSRV] vs [crate MSRV]

### Dependency Impact
- Direct deps: [N]
- Transitive deps: [N] (currently we have [M] total)
- Notable additions: [list any heavy deps]

### Alternatives
1. [existing dep] — covers [X]% of the use case
2. [other crate] — [comparison]
3. Build ourselves — estimated [effort level]

### Trade-offs
- Pro: [specific benefit for our project]
- Pro: [another benefit]
- Con: [specific downside]
- Con: [another downside]
```

## Execution Steps

1. **Read our `Cargo.toml`** — Understand existing dependencies, features, license, MSRV
2. **Fetch crate info** — Use `cargo info <crate>` or crates.io to get metadata
3. **Check dependency tree** — `cargo tree -p <crate>` or analyze on deps.rs
4. **Evaluate compatibility** — Check runtime, license, unsafe, MSRV
5. **Check overlap** — Compare with existing deps and their features
6. **Assess alternatives** — What else could solve the same problem?
7. **Present recommendation** — Structured evaluation with clear verdict
