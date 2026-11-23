## What this changes

<!-- One or two sentences. Link the issue if there is one: "Closes #123". -->

## Screenshots

<!-- For visual changes: before/after, dark and light theme. `node scripts/shot.mjs` can take them. -->

## Checklist

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-features --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] New components follow [CONTRIBUTING.md](../CONTRIBUTING.md): builder + `egui::Widget`, theme-aware, works in the light theme, stops repainting when idle
- [ ] New components have a gallery demo with `covers` filled in
- [ ] Still compiles for `wasm32-unknown-unknown` (`cargo check -p glowkit-gallery --target wasm32-unknown-unknown`)
- [ ] CHANGELOG.md updated for user-facing changes
