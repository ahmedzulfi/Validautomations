#!/usr/bin/env bash
# Build the gallery for the web without the dioxus CLI: cargo + wasm-bindgen (0.2.130).
# Output: examples/gallery/web/pkg. Serve examples/gallery/web with any static server.
set -euo pipefail
cd "$(dirname "$0")/../.."
PROFILE="${PROFILE:-dev}"
cargo build -p glowkit-dioxus-gallery --target wasm32-unknown-unknown --profile "$PROFILE"
DIR=$([ "$PROFILE" = dev ] && echo debug || echo "$PROFILE")
wasm-bindgen --target web --no-typescript --out-dir examples/gallery/web/pkg \
  "target/wasm32-unknown-unknown/$DIR/glowkit-dioxus-gallery.wasm"
echo "built examples/gallery/web/pkg"
