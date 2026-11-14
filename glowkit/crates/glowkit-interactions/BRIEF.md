# Brief for glowkit-interactions builders

You are adding components to **glowkit**, an open-source pure-Rust animated UI library on egui 0.37
(native + WebAssembly). Repo: /home/user/Validautomations/glowkit. The owner wants ONLY high-quality,
distinctive, polished components. Nothing generic.

1. Read CONTRIBUTING.md fully (spec; note the with_state deadlock rule and use `ui.scope_id()`), skim
   crates/glowkit-core/src/*.rs, and read 3-4 components + demos in crates/glowkit-effects/src/{micro_a,micro_b,scroll}/
   to match style. Reusable from glowkit-effects (already a dependency): `ControlSlider` (parameter rows for demos),
   `PaintedScene` (painted image stand-ins), `CardFace`, `MicroIcon`. egui 0.37: grep ~/.cargo/registry/src/*/egui-0.37.0 when unsure.
2. VISUAL REFERENCE: open each source component's public page with Playwright (chromium preinstalled:
   `const { chromium } = require("playwright")`, run with NODE_PATH=$(npm root -g)); screenshot it, Read the PNG,
   hover/drag/scroll with page.mouse to see the interaction. Recreate the look and behaviour from what you SEE.
   Never copy source code, text, logos or images. Use our own type names and painted visuals.
3. SCOPE: only your module directory under crates/glowkit-interactions/src/<module>/ (mod.rs has an empty `demos()`;
   add files, `pub use` from mod.rs, fill demos). Don't touch other modules, lib.rs, Cargo.toml or other crates.
   Type names must be unique across glowkit-interactions AND glowkit-effects (check with grep); prefix if needed.
   Do NOT git commit.
4. Each demo: `covers` MUST include exactly "<Library>: <Name>" as given in your list. Pick the right
   `glowkit_core::demo::Category` (mostly MicroInteractions; Backgrounds, Motion, Charts, TextAnimations, CardsAndBorders,
   Navigation, ThreeD where it fits). Demo ids must be unique across the whole workspace: check with
   `grep -rn 'id: "<id>"' crates`.
5. Quality: light theme is primary (check it FIRST), and dark must also look right. Physical, springy motion. Autoplay or
   a ghost cursor when idle so screenshots show the effect. 2-4 live ControlSlider parameters in demos where
   meaningful. Docs on all public items. Keep performance reasonable.
6. VERIFY (mandatory):
   cd /home/user/Validautomations/glowkit
   cargo clippy -p glowkit-interactions --features demos -- -D warnings
   CARGO_TARGET_DIR=$PWD/target-wasm-ix-<module> FEATURES=interactions PKG=pkg-ix-<module> PROFILE=dev ./scripts/build-web.sh
   curl -s localhost:8787 >/dev/null || (nohup ./scripts/serve.sh >/dev/null 2>&1 &)
   PKG=pkg-ix-<module> WAIT=4000 NODE_PATH=$(npm root -g) node scripts/shot.mjs /tmp/shots-ix-<module> shot/<id>@1 shot/<id>
   ("@1" is the light theme.) Read every PNG, compare with the reference, and fix anything unpolished. If a compile error is in another
   module (another agent), wait and retry.
7. FINAL REPORT under 200 words: types + demo ids, anything skipped and why.
