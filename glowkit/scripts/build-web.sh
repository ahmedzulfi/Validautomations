#!/usr/bin/env bash
# Build the gallery to WebAssembly.
#   ./scripts/build-web.sh                         # release build of everything -> website/gallery/pkg
#   PROFILE=dev ./scripts/build-web.sh             # faster debug build
#   FEATURES=text PKG=pkg-text PROFILE=dev ./scripts/build-web.sh   # only one crate, separate output
# Requires: rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE="${PROFILE:-web}"
PKG="${PKG:-pkg}"
FLAG="--profile $PROFILE"; DIR="$PROFILE"
if [ "$PROFILE" = "dev" ]; then FLAG=""; DIR="debug"; fi
FEAT_ARGS=()
if [ -n "${FEATURES:-}" ]; then FEAT_ARGS=(--no-default-features --features "$FEATURES"); fi
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build $FLAG -p glowkit-gallery --target wasm32-unknown-unknown "${FEAT_ARGS[@]}"
wasm-bindgen --target web --no-typescript --out-dir "website/gallery/$PKG" \
  "$TARGET_DIR/wasm32-unknown-unknown/$DIR/glowkit-gallery.wasm"
if command -v wasm-opt >/dev/null && [ "$PROFILE" != "dev" ]; then
  wasm-opt -Oz -o "website/gallery/$PKG/glowkit-gallery_bg.wasm" "website/gallery/$PKG/glowkit-gallery_bg.wasm"
fi
echo "Built website/gallery/$PKG ($(du -h "website/gallery/$PKG/glowkit-gallery_bg.wasm" | cut -f1))"
