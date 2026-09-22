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

// LICENSE (this repo's own real Apache-2.0 text — dogfoodable, GitHub reads
// it directly, so it's never templated) always gets replaced: license text
// is fetched fresh per generation from GitHub's Licenses API instead of
// bundling every possible SPDX license here, which wouldn't scale and would
// go stale if any text were ever corrected upstream. LICENSE is deleted
// unconditionally first so a dual/multi expression or an unrecognized id
// never leaves this repo's own Apache-2.0 text behind by accident.
//
// LICENSE is regex-validated (see cargo-generate.toml) to only the
// characters a shell command can safely receive unquoted — never build a
// command this way from an unvalidated free-text placeholder (AUTHOR_NAME,
// by contrast, only ever reaches Rhai's native string::replace below, never
// a shell command, specifically so it can safely contain anything,
// including quotes).
file::delete("LICENSE");
let license = variable::get("LICENSE");
let author = variable::get("AUTHOR_NAME");
let year = system::date().year.to_string();

let raw_parts = license.split(" OR ");
let parts = [];
for p in raw_parts {
    let part = p;
    part.trim();
    parts.push(part);
}
let multi = parts.len() > 1;

for part in parts {
    // A single license becomes plain LICENSE; "MIT OR Apache-2.0" becomes
    // LICENSE-MIT + LICENSE-Apache-2.0 (Rust's own dual-license convention).
    let target = if multi { "LICENSE-" + part } else { "LICENSE" };
    let tmp = target + ".fetching";
    // GitHub's Licenses API is case-insensitive on the key and covers the
    // common OSS licenses (MIT, Apache-2.0, the GPL/AGPL/LGPL family,
    // MPL-2.0, BSD-2/3-Clause, Unlicense, ...), not the full SPDX list.
    // mv only runs if the fetch succeeded; otherwise the tmp file (which
    // may hold a JSON error body, not the license text) is discarded and
    // no `target` is created at all.
    system::command("gh", [
        "api", "licenses/" + part, "--jq", ".body",
        ">", tmp, "2>", "/dev/null",
        "&&", "mv", tmp, target,
        "||", "rm", "-f", tmp
    ]);
    if file::exists(target) {
        // GitHub's license bodies use [year]/[fullname] as fill-in markers
        // (MIT/BSD-style licenses only — Apache/GPL-family bodies have no
        // such placeholder, so this is a harmless no-op for those). Rhai's
        // string::replace mutates in place and returns unit, not a new
        // string — call it as a statement, never chained or assigned.
        let body = system::command("cat", [target]);
        body.replace("[year]", year);
        body.replace("[fullname]", author);
        file::write(target, body);
    } else {
        print("note: rust-workspace-template couldn't find a license text for '"
            + part + "' via GitHub's license list — no " + target
            + " written. Add one by hand if you need it.");
    }
}
