#!/usr/bin/env python3
"""Rough X length check for the launch drafts.

Splits a Markdown file on headings, drops [media: ...] notes and code fences, counts each link as
23 characters (X's t.co length) and flags posts over 280.

usage: python3 launch/count.py launch/x-thread.md [launch/posts.md ...]
"""
import re
import sys

URL = re.compile(r"https?://\S+")
LIMIT = 280


def posts(text):
    blocks = re.split(r"^#{2,3} .*$", text, flags=re.M)
    heads = re.findall(r"^#{2,3} (.*)$", text, flags=re.M)
    for head, body in zip(heads, blocks[1:]):
        body = re.sub(r"\[media:[^\]]*\]", "", body)
        body = body.split("\n---")[0]
        body = "\n".join(l for l in body.strip().splitlines() if not l.startswith(("Notes:", "> ")))
        body = body.strip()
        if body:
            yield head, body


def x_len(s):
    return len(URL.sub("x" * 23, s))


bad = 0
for path in sys.argv[1:]:
    for head, body in posts(open(path, encoding="utf-8").read()):
        n = x_len(body)
        flag = "  OVER" if n > LIMIT else ""
        bad += n > LIMIT
        print(f"{n:4d}  {path}: {head}{flag}")
        for ch in ("—", "–"):
            if ch in body:
                print(f"      dash found in {head}")
                bad += 1
sys.exit(1 if bad else 0)
