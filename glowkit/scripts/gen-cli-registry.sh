#!/usr/bin/env bash
# Regenerate the component registry embedded in the glowkit CLI (crates/glowkit-cli/registry.json).
#
#   ./scripts/gen-cli-registry.sh            # registry only
#   BUNDLE=1 ./scripts/gen-cli-registry.sh   # also write crates/glowkit-cli/sources.bundle
#                                            # (needed before `cargo publish -p glowkit-cli`)
#
# The registry comes from the gallery (`--dump-json`); each demo is tagged with the crate whose
# demos.rs defines it.
set -euo pipefail
cd "$(dirname "$0")/.."

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
cargo run -q -p glowkit-gallery -- --dump-json > "$tmp"

python3 - "$tmp" crates/glowkit-cli/registry.json <<'PY'
import json, os, re, sys

src, dst = sys.argv[1], sys.argv[2]
demos = json.load(open(src, encoding="utf-8"))

# demo id -> crate, by scanning every demos*.rs for `id: "<id>"`
owner = {}
sources = []
for root, _, files in os.walk("crates"):
    if "/src" not in root.replace(os.sep, "/") + "/":
        continue
    for f in files:
        if not f.endswith(".rs"):
            continue
        p = os.path.join(root, f)
        crate = p.replace(os.sep, "/").split("/")[1]
        if not crate.startswith("glowkit-") or crate in ("glowkit-cli", "glowkit-mcp"):
            continue
        text = open(p, encoding="utf-8").read()
        for m in re.finditer(r'\bid:\s*"([a-z0-9-]+)"', text):
            owner.setdefault(m.group(1), crate)
        if re.match(r"demos.*\.rs$", f):
            sources.append((crate, text))

# demos built by helper functions: fall back to any string literal equal to the id in a demos file
for d in demos:
    if d["id"] not in owner:
        for crate, text in sources:
            if f'"{d["id"]}"' in text:
                owner[d["id"]] = crate
                break

out = []
missing = []
for d in demos:
    crate = owner.get(d["id"])
    if crate is None:
        missing.append(d["id"])
        continue
    out.append({
        "id": d["id"],
        "name": d["name"],
        "category": d["category"],
        "crate": crate,
        "description": d["description"],
        "code": d["code"],
    })
if missing:
    print("warning: no crate found for:", ", ".join(missing), file=sys.stderr)
with open(dst, "w", encoding="utf-8") as fh:
    json.dump(out, fh, ensure_ascii=False, indent=0)
    fh.write("\n")
print(f"wrote {len(out)} components to {dst}")
PY

if [ "${BUNDLE:-0}" = "1" ]; then
  GLOWKIT_CLI_WRITE_BUNDLE=1 cargo build -q -p glowkit-cli
  echo "wrote crates/glowkit-cli/sources.bundle"
fi
