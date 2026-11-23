#!/usr/bin/env python3
"""Screenshot showcased components for the static site (website/img/c/<id>.webp + thumbnails).

The images are committed, so CI never needs a browser. Re-run this when demos change.

usage (needs Node + Playwright, Pillow, a built gallery in website/gallery/pkg):
    python3 -m http.server 8787 -d website &          # or ./scripts/serve.sh
    python3 scripts/site-images.py                     # only components without an image
    python3 scripts/site-images.py --all               # redo everything
    python3 scripts/site-images.py ai-chat game-hud    # specific ids
    python3 scripts/site-images.py --convert DIR       # only convert existing shot.mjs PNGs in DIR

Each shot uses the gallery's bare route `#shot/<id>@1` (light theme) at 1200x630.
"""
import json
import os
import subprocess
import sys
import tempfile

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "website", "img", "c")
THUMBS = os.path.join(OUT, "t")
W, H = 1200, 630
TW, TH = 480, 252


def convert(png_dir):
    os.makedirs(THUMBS, exist_ok=True)
    n = 0
    for f in sorted(os.listdir(png_dir)):
        if not (f.startswith("shot_") and f.endswith("_1.png")):
            continue
        cid = f[len("shot_"):-len("_1.png")]
        im = Image.open(os.path.join(png_dir, f)).convert("RGB")
        if im.size != (W, H):
            im = im.resize((W, H), Image.LANCZOS)
        im.save(os.path.join(OUT, cid + ".webp"), "WEBP", quality=80, method=6)
        im.resize((TW, TH), Image.LANCZOS).save(os.path.join(THUMBS, cid + ".webp"), "WEBP", quality=74, method=6)
        n += 1
    print(f"converted {n} screenshots into {os.path.relpath(OUT, ROOT)}")


def main():
    args = sys.argv[1:]
    if args[:1] == ["--convert"]:
        return convert(args[1])
    demos = json.load(open(os.path.join(ROOT, "website", "catalog.json")))["demos"]
    ids = [d["id"] for d in demos if d.get("showcase", True)]
    if args and args[0] != "--all":
        ids = args
    elif not args:
        ids = [i for i in ids if not os.path.exists(os.path.join(OUT, i + ".webp"))]
    if not ids:
        print("all components already have screenshots")
        return
    tmp = tempfile.mkdtemp(prefix="glowkit-shots-")
    env = dict(os.environ, W=str(W), H=str(H), WAIT=os.environ.get("WAIT", "2500"))
    for i in range(0, len(ids), 40):
        batch = ["shot/" + x + "@1" for x in ids[i:i + 40]]
        subprocess.run(["node", os.path.join(ROOT, "scripts", "shot.mjs"), tmp] + batch, env=env, check=True)
    convert(tmp)


if __name__ == "__main__":
    main()
