#!/usr/bin/env bash
# Build agents/starter against this checkout of glowkit (instead of crates.io).
#
# The shipped template depends on `glowkit = "0.1"` from crates.io. This script copies it to a
# temporary directory, swaps that line for a path dependency on the local workspace, and runs
# `cargo build` (plus clippy with -D warnings when available), so the template is known to compile
# with the current sources.
#
# usage: agents/check-starter.sh [--no-clippy]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/glowkit-starter.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

cp -r "$ROOT/agents/starter/." "$TMP/"
# Swap the crates.io dependency for the local workspace.
sed -i.bak "s|^glowkit = \"0.1\"$|glowkit = { path = \"$ROOT/crates/glowkit\" }|" "$TMP/Cargo.toml"
rm -f "$TMP/Cargo.toml.bak"
grep -q "path = \"$ROOT/crates/glowkit\"" "$TMP/Cargo.toml" || { echo "error: could not swap the glowkit dependency" >&2; exit 1; }
# Reuse the workspace's resolved versions and build cache.
cp "$ROOT/Cargo.lock" "$TMP/Cargo.lock"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/starter-check}"

cd "$TMP"
cargo build
if [[ "${1:-}" != "--no-clippy" ]]; then
    cargo clippy -- -D warnings
fi
echo "starter OK"
