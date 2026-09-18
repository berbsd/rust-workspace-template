# Settings Documentation

## Prompt

Find all configurable settings in this codebase, whether they come from environment variables, config files, CLI arguments, or hardcoded defaults. Create a comprehensive settings document that lists each setting with its name, source, type, default value, valid range, and a description of what it controls.

## What to Scan For

### Environment Variables
- `std::env::var("...")` and `std::env::var_os("...")`
- `env!()` and `option_env!()` macros
- `dotenvy` / `dotenv` loading
- Environment variable parsing in config structs

### Config File Parsing
- `serde` deserialization targets (TOML, YAML, JSON config structs)
- `config` crate usage
- Custom config file parsers
- `.env`, `.env.example`, `config.toml`, `settings.yaml`, etc.

### CLI Arguments
- `clap` / `structopt` definitions
- Custom argument parsing
- Subcommand-specific flags

### Hardcoded Defaults
- `unwrap_or()`, `unwrap_or_else()`, `unwrap_or_default()` on config values
- Default trait implementations for config structs
- Fallback values in config loading chains

### Feature Flags
- `#[cfg(feature = "...")]` that change behavior
- Compile-time feature toggles that affect runtime behavior

## Output Format

Create a document organized by component/service:

```markdown
# Configuration Reference

## Service: `api-gateway`

| Setting | Source | Type | Default | Valid Range | Description |
|---------|--------|------|---------|-------------|-------------|
| `PORT` | env | `u16` | `3000` | 1-65535 | HTTP listen port |
| `DATABASE_URL` | env | `String` | *required* | valid URL | PostgreSQL connection string |
| `LOG_LEVEL` | env | `String` | `"info"` | trace/debug/info/warn/error | Tracing filter level |
| `MAX_CONNECTIONS` | env | `u32` | `100` | 1-10000 | Database connection pool size |
| `--config` | CLI | `PathBuf` | `./config.toml` | valid path | Config file location |
| `server.timeout_secs` | TOML | `u64` | `30` | 1-3600 | Request timeout in seconds |
```

## Additional Analysis

For each setting, also note:

- **Validation:** Is the value validated? What happens with invalid input?
- **Sensitivity:** Does it contain secrets (passwords, tokens, keys)?
- **Runtime vs startup:** Can it be changed without restart?
- **Cross-references:** Do multiple settings interact? (e.g., pool size vs max connections)

## Flag These Issues

- Settings with no validation (any string accepted where only specific values work)
- Settings with defaults that differ between environments (dev vs prod)
- Sensitive settings with no redaction in logs
- Settings documented in code but not in `.env.example` or config templates
- Settings in `.env.example` that don't match what the code actually reads
- Duplicate settings (same concept configured in two different ways)

## Execution Steps

1. **Scan all config entry points** — env vars, CLI args, config files, feature flags
2. **Trace default values** — Follow the fallback chain for each setting
3. **Check validation** — Note which settings validate their input
4. **Check `.env.example`** — Compare against actual env var usage
5. **Generate document** — Create the settings reference
6. **Flag issues** — Note missing validation, undocumented settings, inconsistencies
7. **Present** — Show the document to the user for review

## Where to Put the Documentation

- If `.env.example` exists → update it to match reality
- Settings reference → `docs/configuration.md` or project README section
- For complex projects → consider a `docs/settings/` directory per service
