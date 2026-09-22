// =============================================================================
// post-generation hook
// =============================================================================
// Runs after Liquid expansion, on the copied output, before it's moved to the
// user's destination (a failing hook leaves the destination untouched).
//
// system::command() joins its args into one shell command line rather than
// exec'ing them as a real argv array — an unquoted arg containing spaces gets
// re-split by the shell. Every sed expression below is therefore wrapped in
// its own single quotes so it survives that re-split as one token. Because
// this interpolates RUST_VERSION into a shell command, RUST_VERSION must stay
// regex-validated to digits and dots (see cargo-generate.toml) — never build
// a command this way from an unvalidated free-text placeholder.
// =============================================================================

let rust_version = variable::get("RUST_VERSION");
let version_re = "[0-9]+\\.[0-9]+\\.[0-9]+";

// Three independent places pin the toolchain version, because neither
// Docker's FROM nor the CI action's toolchain step reads rust-toolchain.toml
// at that point — Docker's base-image tag resolves before any repo file is
// copied in, and the CI toolchain-install step runs before the checkout is
// guaranteed usable by rustup's auto-detection. All three are kept as real,
// dogfoodable version strings in this repo (not `{{RUST_VERSION}}` literals,
// which would break `cargo build`/rustup here directly) and substituted here
// instead, once, at generation time.
let substitutions = [
    ["rust-toolchain.toml", "s#channel[[:space:]]*= \"" + version_re + "\"#channel    = \"" + rust_version + "\"#"],
    ["docker/Dockerfile", "s#FROM rust:" + version_re + "-alpine3\\.23#FROM rust:" + rust_version + "-alpine3.23#"],
    [".github/workflows/ci.yml", "s#dtolnay/rust-toolchain@" + version_re + "#dtolnay/rust-toolchain@" + rust_version + "#g"],
];

for entry in substitutions {
    let file = entry[0];
    let expr = "'" + entry[1] + "'";
    system::command("sed", ["-i.bak", "-E", "-e", expr, file]);
    system::command("rm", ["-f", file + ".bak"]);
}

// Cargo.toml can't get the `{% if keep_example %}` treatment README.project.md
// gets: it's real workspace config that must stay parseable Cargo.toml in
// *this* repo too (Liquid tags inside it broke `cargo metadata` here
// directly). So the members glob and the `example` dependency line are
// edited in place instead, only when the example is dropped — mirrors what
// this template's old scaffold.sh did with the same two sed edits.
let keep_example = variable::get("keep_example");
if keep_example == false {
    let member_expr = "'s#members[[:space:]]+= \\[\"crates/\\*\", \"hosts/\\*\", \"services/\\*\"\\]#members  = [\"crates/*\"]#'";
    system::command("sed", ["-i.bak", "-E", "-e", member_expr, "Cargo.toml"]);
    let dep_line_expr = "'/^example[[:space:]]+= \\{ path = \"services\\/example\" \\}$/d'";
    system::command("sed", ["-i.bak", "-E", "-e", dep_line_expr, "Cargo.toml"]);
    system::command("rm", ["-f", "Cargo.toml.bak"]);
}

// README.md was already dropped by cargo-generate.toml's `ignore` list (it
// documents the template itself, not a generated project). Move the real
// generated-project README into its place.
file::rename("README.project.md", "README.md");
