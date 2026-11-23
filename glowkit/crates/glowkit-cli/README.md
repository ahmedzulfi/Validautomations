# glowkit-cli

The `glowkit` command: copy [glowkit](https://github.com/ahmedzulfi/glowkit) components into your
project as source you own (the shadcn/ui way), or scaffold a new eframe app. Component sources are
embedded in the binary, so it works offline.

```bash
cargo install glowkit-cli

glowkit list chart                     # search components
glowkit info shimmer-button            # crate, feature, use path and a usage snippet
glowkit add shimmer-button aurora      # copy the source into src/glowkit/
glowkit init my-app --template dashboard
```

Templates for `init`: `blank` (default), `dashboard`, `ai-chat`, `agent-sessions`, `trading`, `game`.

Copied components depend only on `egui` and `glowkit-core`. If you would rather depend on the
library and get updates, use the [`glowkit`](https://crates.io/crates/glowkit) crate instead.

## License

MIT or Apache-2.0, at your option.
