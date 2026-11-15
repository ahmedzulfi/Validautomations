#!/usr/bin/env python3
"""Try `glowkit add <id>` + compile for every component in the CLI registry.

Components are added in batches into a scratch cargo project that depends only on
`glowkit-core` (patched to this workspace) and `egui`; each batch imports every type the CLI says
the component exposes. A failing batch is bisected down to the components that break it.

    python3 scripts/test-cli-add.py [--batch 60] [--check] [--force] [--work DIR] [ids...]

Prints the success rate and writes a report (failures + first compiler error) to DIR/report.txt.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def sh(cmd, cwd=None, env=None):
    return subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--batch", type=int, default=60)
    ap.add_argument("--check", action="store_true", help="cargo check instead of cargo build")
    ap.add_argument("--force", action="store_true", help="pass --force to `glowkit add` (copy deep components too)")
    ap.add_argument("--work", default=os.path.join(ROOT, "target", "cli-add-test"))
    ap.add_argument("ids", nargs="*")
    a = ap.parse_args()

    r = sh(["cargo", "build", "-q", "-p", "glowkit-cli"], cwd=ROOT)
    if r.returncode != 0:
        sys.exit(r.stderr)
    os.makedirs(a.work, exist_ok=True)
    # private copy, so rebuilding the CLI during a sweep doesn't disturb it
    cli = os.path.join(a.work, "glowkit")
    shutil.copy(os.path.join(ROOT, "target", "debug", "glowkit"), cli)
    reg = json.load(open(os.path.join(ROOT, "crates", "glowkit-cli", "registry.json"), encoding="utf-8"))
    ids = a.ids or [c["id"] for c in reg]

    os.makedirs(a.work, exist_ok=True)
    proj = os.path.join(a.work, "sweep")
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(a.work, "target"))

    def attempt(batch):
        """Fresh project, `glowkit add` the batch, compile. Returns (ok, fallback_ids, log)."""
        shutil.rmtree(proj, ignore_errors=True)
        os.makedirs(os.path.join(proj, "src"))
        with open(os.path.join(proj, "Cargo.toml"), "w") as f:
            f.write(
                '[workspace]\n\n[package]\nname = "sweep"\nversion = "0.1.0"\nedition = "2021"\n\n'
                '[dependencies]\nglowkit-core = "0.1"\negui = "0.37"\n\n'
                f'[patch.crates-io]\nglowkit-core = {{ path = "{ROOT}/crates/glowkit-core" }}\n'
            )
        shutil.copy(os.path.join(ROOT, "Cargo.lock"), os.path.join(proj, "Cargo.lock"))
        open(os.path.join(proj, "src", "main.rs"), "w").write("fn main() {}\n")
        r = sh([cli, "add", *batch] + (["--force"] if a.force else []), cwd=proj)
        if r.returncode != 0:
            return False, [], "add failed:\n" + r.stdout + r.stderr
        fallback = re.findall(r"^! (\S+) was not copied", r.stdout, re.M)
        uses = sorted(set(re.findall(r"^\s+(use crate::glowkit::[^;]+;)$", r.stdout, re.M)))
        main = "#![allow(unused_imports)]\nmod glowkit;\n" + "\n".join(uses) + "\nfn main() {}\n"
        open(os.path.join(proj, "src", "main.rs"), "w").write(main)
        r = sh(["cargo", "check" if a.check else "build", "--message-format=short"], cwd=proj, env=env)
        errs = [l for l in r.stderr.splitlines() if "error" in l]
        return r.returncode == 0, fallback, "\n".join(errs[:8])

    ok, failed, fell_back = [], {}, set()

    def run(batch):
        good, fb, log = attempt(batch)
        fell_back.update(fb)
        if good:
            ok.extend(i for i in batch if i not in fb)
            print(f"  ok   {len(batch):3} ({batch[0]} .. {batch[-1]})", flush=True)
        elif len(batch) == 1:
            failed[batch[0]] = log
            print(f"  FAIL {batch[0]}: {log.splitlines()[0] if log else ''}", flush=True)
        else:
            h = len(batch) // 2
            run(batch[:h])
            run(batch[h:])

    for k in range(0, len(ids), a.batch):
        run(ids[k : k + a.batch])

    total = len(ids)
    copied_ok = len(set(ok))
    lines = [
        f"components: {total}",
        f"copied + compiled: {copied_ok} ({100.0 * copied_ok / total:.1f}%)",
        f"fallback (dependency instructions): {len(fell_back)}",
        f"failed: {len(failed)}",
        "",
    ]
    for i, log in failed.items():
        lines.append(f"## {i}\n{log}\n")
    if fell_back:
        lines.append("fallback: " + ", ".join(sorted(fell_back)))
    report = "\n".join(lines)
    open(os.path.join(a.work, "report.txt"), "w").write(report)
    print("\n" + "\n".join(lines[:4]))
    print(f"report: {os.path.join(a.work, 'report.txt')}")


if __name__ == "__main__":
    main()
