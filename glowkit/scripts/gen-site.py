#!/usr/bin/env python3
"""Generate the static, crawlable part of the website from the demo registry.

The gallery is a WebAssembly canvas, so search engines and AI assistants see nothing in it. This
script writes plain HTML for every showcased component, every category, every agent recipe and
the main audiences, plus sitemap.xml, robots.txt, 404.html, llms.txt and llms-full.txt. It also
refreshes the generated blocks of index.html and gallery/index.html (between `gen:` markers).

usage:
    cargo run --release -p glowkit-gallery -- --dump-json > demos.json
    python3 scripts/catalog.py demos.json
    python3 scripts/gen-site.py demos.json      # or with no argument: reuse website/catalog.json

Standard library only. Screenshots in website/img/c/ are committed (made with scripts/shot.mjs,
see scripts/gen-site-images.sh); pages use them when they exist.
"""

import datetime
import html
import json
import os
import re
import subprocess
import sys
from collections import OrderedDict

# The one place the public URL lives. For a custom domain, change this line.
BASE_URL = "https://ahmedzulfi.github.io/glowkit/"
REPO = "https://github.com/ahmedzulfi/glowkit"
REPO_BLOB = REPO + "/blob/main/"
REPO_TREE = REPO + "/tree/main/"

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WEB = os.path.join(ROOT, "website")
SITE = "glowkit"
OG_DEFAULT = "og.png"
TAGLINE = "Premium animated UI for Rust apps"

# ---------------------------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------------------------

def load_json(path, default=None):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


catalog = load_json(os.path.join(WEB, "catalog.json"), {"libraries": [], "demos": []})
demos_all = load_json(sys.argv[1]) if len(sys.argv) > 1 else catalog["demos"]
if not demos_all:
    sys.exit("no demos: pass the output of `glowkit-gallery --dump-json`")
# API details the MCP server embeds (use paths, crate, feature); optional.
mcp = load_json(os.path.join(ROOT, "crates", "glowkit-mcp", "data", "components.json"), {}) or {}
mcp_by_id = {c["id"]: c for c in mcp.get("components", [])}
cli_ids = {c["id"] for c in (load_json(os.path.join(ROOT, "crates", "glowkit-cli", "registry.json"), []) or [])}
lib_urls = {l["name"]: l["url"] for l in catalog.get("libraries", [])}

demos = [d for d in demos_all if d.get("showcase", True)]
by_id = {d["id"]: d for d in demos}
N = len(demos)
N_ROUND = f"{(N // 10) * 10}+"  # "680+"

# Category order as the gallery shows it, with copy written for each.
CATEGORIES = OrderedDict()
for d in demos:
    CATEGORIES.setdefault(d["category"], []).append(d)

# noun used in "<Name>: animated <noun> component for Rust (egui)"
CAT_NOUN = {
    "AI": "AI", "Text Animations": "text", "Backgrounds": "background", "Buttons": "button",
    "Cards & Borders": "card", "Cursor & Interaction": "cursor", "Micro-interactions": "micro-interaction",
    "Motion & Layout": "motion", "Motion Engine": "motion engine", "3D": "3D", "Loaders & Progress": "loader",
    "Inputs & Forms": "input", "Navigation": "navigation", "Overlays & Feedback": "overlay",
    "Data Display": "data display", "Charts": "chart", "Blocks": "section block",
    "Canvas & Editors": "canvas editor", "Dashboards & Shells": "dashboard", "Game UI": "game UI",
    "Templates": "template",
}

CAT_COPY = {
    "AI": ("AI agent interface components for Rust",
           "Everything an AI app needs on screen: streaming Markdown chat, prompt inputs with attachments, reasoning and chain-of-thought panels, tool calls with live arguments, approvals, diff review, traces, agent trees, run monitors, cost and context meters, voice orbs and MCP server managers. Pair them with glowkit-stream to drive them from a real AG-UI, Vercel AI SDK, ACP, rig or OpenAI-compatible stream."),
    "Text Animations": ("Animated text effects for Rust and egui",
           "Per-glyph text effects drawn straight into egui: shimmering and gradient text, blur and split reveals, decrypting and scrambling characters, typewriters, rotating words, counters that roll to a value, glitch, wave and proximity effects. Each one is a single widget you add with ui.add()."),
    "Backgrounds": ("Animated backgrounds for Rust desktop apps",
           "Full-panel animated backgrounds built from vertex-colored meshes, so they stay cheap on the GPU: aurora, silk, plasma, particles, meteors, starfields, beams, grids, waves, liquid metal and dozens more. Paint one behind any egui layout with a single call."),
    "Buttons": ("Animated button components for egui",
           "Buttons that react: shimmer sweeps, rainbow and moving borders, magnetic pull, ripples, particle bursts, 3D push states, copy buttons with feedback and stateful submit buttons. They return a normal egui Response, so clicked() works as usual."),
    "Cards & Borders": ("Animated cards and borders for Rust UIs",
           "Cards with personality: border beams and shine borders, spotlight and magic cards that follow the pointer, tilt and glare, glass surfaces, pixel and decay reveals, and bento grids. Content inside stays fully interactive."),
    "Cursor & Interaction": ("Cursor effects and pointer interactions for egui",
           "Pointer-driven effects: click sparks and ripples, ribbon, pixel and image trails, smooth and target cursors, magnets, confetti and fireworks, lenses, scratch-to-reveal and before/after compare sliders."),
    "Micro-interactions": ("Micro-interactions for Rust desktop apps",
           "Small, physical details that make an app feel finished: liquid toggles, slide-to-confirm, swipe actions, reorderable lists, pull to refresh, emoji reactions, rolling counters, success checks, error shakes and notification badges, each tuned with springs."),
    "Motion & Layout": ("Animated layout components for Rust",
           "Layout pieces that move: a magnifying dock, marquees, animated lists, card stacks and swaps, carousels and coverflow, expandable cards, masonry, parallax heroes, sticky scroll reveals and animated beams between elements."),
    "Motion Engine": ("A Framer-Motion-style animation engine for Rust",
           "The engine under every glowkit animation, exposed for your own UI: springs, keyframes, GSAP-style timelines, variants with stagger, enter and exit presence, FLIP layout animation, shared elements, drag with inertia and snapping, scroll-linked values, motion paths and shape morphing."),
    "3D": ("Real-time 3D components for egui, no GPU setup",
           "A software 3D renderer that draws lit, depth-sorted meshes straight into egui: model viewers with orbit controls, globes, particle clouds that morph between shapes, terrains, voxel text, 3D charts and carousels. No wgpu boilerplate, and it runs in WebAssembly too."),
    "Loaders & Progress": ("Loaders and progress indicators for egui",
           "Spinners, progress rings and multi-step loaders that animate smoothly and stop repainting when they are done."),
    "Inputs & Forms": ("Animated form inputs for Rust apps",
           "Form controls with motion: one-time-code inputs with autofill, ratings, color pickers, calendars, file dropzones, vanishing inputs, day/night switches and availability schedulers."),
    "Navigation": ("Animated navigation components for egui",
           "Menus and navigation that move with intent: bubble, pill, gooey and staggered menus, card navs, floating and resizable navbars, sidebars, command palettes, tables of contents and morphing tab bars."),
    "Overlays & Feedback": ("Animated overlays, toasts and dialogs for Rust",
           "Overlays that open and close with springs: toasts, alert dialogs, morphing modals, dynamic-island notifications, guided tours, banners, undo bars, confirm sheets and feedback widgets."),
    "Data Display": ("Data display components for Rust and egui",
           "Ways to show information: data tables, timelines, stat cards, tree views, syntax-highlighted code blocks and diffs, terminals, globes, avatar groups and empty states."),
    "Charts": ("Animated charts for Rust dashboards",
           "Charts that draw in and respond to the pointer: line, area, bar, pie and donut, radar, radial, gauges, sparklines, scatter, heatmaps and calendar heatmaps, funnels, treemaps, candlesticks, Sankey diagrams and KPI tiles."),
    "Blocks": ("Page section blocks for Rust apps",
           "Ready-made sections you compose into screens: heroes, feature grids and bento layouts, pricing, testimonials, FAQs, teams, changelogs, logo clouds and device mockups."),
    "Canvas & Editors": ("Node editors and infinite canvases for egui",
           "The heavy pieces of creative and developer tools: a React-Flow-style node editor, shader graphs, flowcharts, an infinite canvas and whiteboard, kanban and gantt boards, mind maps, a timeline editor, a data grid, org charts and a signature pad."),
    "Dashboards & Shells": ("Dashboard layouts and app shells for egui",
           "App shells to build dashboards and tools on: sidebar, inset, floating, rail, dual-pane, top-nav, inspector and bottom-nav layouts that collapse and animate."),
    "Game UI": ("Game UI for Rust: menus, HUDs and inventories",
           "Game interface pieces that take plain game state each frame: main and pause menus, HUD bars with damage trails, minimaps, ammo counters, cooldown icons, hotbars, inventories with tooltips, skill trees, dialogue boxes, quest trackers, loot and achievement popups, kill feeds and scoreboards. Works in Bevy through bevy_egui."),
    "Templates": ("Full page and app templates for Rust",
           "Complete screens you can start from: landing, pricing, auth, onboarding and error pages, plus working app templates such as an analytics dashboard, CRM, e-commerce admin, mail client, team chat, AI assistant, agent session manager, local LLM chat, trading terminal and profiler."),
}


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


CAT_SLUG = {c: slug(c) for c in CATEGORIES}
for c, s in CAT_SLUG.items():
    if s in by_id:
        CAT_SLUG[c] = "category-" + s  # never collide with a component id
missing = [c for c in CATEGORIES if c not in CAT_COPY or c not in CAT_NOUN]
if missing:
    sys.exit(f"gen-site.py: write CAT_COPY / CAT_NOUN for new categories: {missing}")

# Component names that appear more than once get their category appended in titles.
_name_count = {}
for d in demos:
    _name_count[d["name"]] = _name_count.get(d["name"], 0) + 1


def display_name(d):
    return d["name"] if _name_count[d["name"]] == 1 else f"{d['name']} ({d['category']})"


def lastmod():
    try:
        out = subprocess.run(["git", "log", "-1", "--format=%cs"], cwd=ROOT, capture_output=True, text=True, timeout=10)
        if out.returncode == 0 and re.match(r"\d{4}-\d\d-\d\d$", out.stdout.strip()):
            return out.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        pass
    return datetime.date.today().isoformat()


LASTMOD = lastmod()


def workspace_version():
    try:
        m = re.search(r'^version\s*=\s*"([^"]+)"', open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8").read(), re.M)
        return m.group(1) if m else "0.1.0"
    except OSError:
        return "0.1.0"


VERSION = workspace_version()

# ---------------------------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------------------------

def esc(s):
    return html.escape(str(s), quote=True)


def img_path(cid):
    return f"img/c/{cid}.webp"


def has_img(cid):
    return os.path.exists(os.path.join(WEB, img_path(cid)))


def thumb_path(cid):
    return f"img/c/t/{cid}.webp"


def has_thumb(cid):
    return os.path.exists(os.path.join(WEB, thumb_path(cid)))


def parse_cover(c):
    if ":" in c:
        lib, item = [x.strip() for x in c.split(":", 1)]
        return lib, item
    return None, c.strip()


def covers_text(d, limit=3):
    """'React Bits Shiny Text, Magic UI Animated Shiny Text' (first few)."""
    out = []
    for c in d["covers"][:limit]:
        lib, item = parse_cover(c)
        out.append(f"{lib} {item}" if lib else item)
    return ", ".join(out)


SENT = re.compile(r"(?<=[.!?])\s+(?=[A-Z0-9])")


def fit_description(core, tails, lo=150, hi=160):
    """Build a meta description between lo and hi characters from `core` plus optional tails.
    Cuts only at sentence or clause boundaries, never inside a phrase."""
    core = re.sub(r"\s+", " ", core).strip()
    sents = SENT.split(core)
    bases = []
    for k in range(len(sents), 0, -1):
        bases.append(" ".join(sents[:k]).strip())
    first = sents[0].rstrip(".!?")
    clauses = re.split(r"(?<=[,;:])\s+|\s+(?=\()", first)
    for k in range(len(clauses) - 1, 0, -1):
        bases.append(" ".join(clauses[:k]).rstrip(",;: "))
    seen, cands = set(), []
    for b in bases:
        b = b.rstrip()
        if not b.endswith((".", "!", "?")):
            b += "."
        if b not in seen:
            seen.add(b)
            cands.append(b)
    combos = [()] + [(t,) for t in tails] + [(a, b) for a in tails for b in tails if a is not b]
    combos += [(a, b, c) for a in tails[:6] for b in tails[:6] for c in tails[:6] if len({a, b, c}) == 3]
    for b in cands:
        for combo in combos:
            out = " ".join((b,) + combo)
            if lo <= len(out) <= hi:
                return out
    # Nothing fits exactly: the closest candidate under the limit.
    best = max((" ".join((b,) + c) for b in cands for c in combos if len(" ".join((b,) + c)) <= hi), key=len, default=cands[-1][:hi])
    return best


def component_title(name, noun):
    """Topic first, ~50-60 characters where the name allows it, always unique (names are)."""
    for t in (f"{name}: animated Rust {noun} component (egui) | {SITE}",
              f"{name}: animated {noun} component for Rust | {SITE}",
              f"{name}: Rust {noun} component for egui | {SITE}",
              f"{name}: Rust egui component | {SITE}"):
        if len(t) <= 62:
            return t
    return f"{name} for Rust and egui | {SITE}"


def component_description(d):
    name = d["name"]
    noun = CAT_NOUN[d["category"]]
    cov = [parse_cover(c) for c in d["covers"]]
    libs = []
    for lib, _ in cov:
        if lib and lib not in libs:
            libs.append(lib)
    tails = [
        f"Rust and egui, native and WebAssembly.",
        f"Native Rust UI for egui apps.",
        f"Built in Rust on egui.",
        f"Free {noun} component for Rust (egui).",
        f"A glowkit component for Rust and egui.",
        f"For native Rust apps and the web.",
        f"Open source, for Rust desktop apps.",
        f"Windows, macOS, Linux and the web.",
        f"In Rust, no webview.",
        f"Rust (egui).",
    ]
    if libs:
        tails.insert(0, f"Like {libs[0]}, in Rust.")
    return fit_description(d["description"], tails)


def category_description(cat, ds):
    noun = CAT_NOUN[cat]
    head = f"{len(ds)} animated {noun} components for Rust and egui, including "
    names = [d["name"] for d in ds]
    best = None
    for k in range(1, len(names) + 1):
        s = head + ", ".join(names[:k - 1]) + (" and " if k > 1 else "") + names[k - 1] + "."
        if len(s) > 160:
            break
        best = s
        if len(s) >= 150:
            return s
    tails = ["Native and WebAssembly.", "Free and open source.", "No webview.", "In Rust."]
    out = best or head.rstrip(", including ") + "."
    for t in tails:
        if len(out) >= 150:
            break
        if len(out + " " + t) <= 160:
            out += " " + t
    return out


def page_description(core, extra=()):
    tails = list(extra) + [
        "Native Rust UI on egui, no webview.",
        "For Rust desktop apps and WebAssembly.",
        "Open source, MIT or Apache-2.0.",
        "Built in Rust on egui 0.37.",
        "Free and open source.",
        "Rust and egui.",
    ]
    return fit_description(core, tails)


# --- tiny Rust highlighter (keywords, strings, comments, numbers, types, calls) ---------------
RUST_KW = set("as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while".split())
TOKEN = re.compile(r"""
    (?P<c>//[^\n]*)
  | (?P<s>r\#*"(?:.|\n)*?"\#*|"(?:\\.|[^"\\])*"|b?'(?:\\.|[^'\\])')
  | (?P<n>\b\d[\d_]*(?:\.\d[\d_]*)?(?:_?[a-z]\d*)?\b)
  | (?P<w>[A-Za-z_][A-Za-z0-9_]*!?)
  | (?P<o>.|\n)
""", re.X)


def highlight_rust(code):
    out = []
    pos_after = lambda m: code[m.end():m.end() + 1]
    for m in TOKEN.finditer(code):
        kind, text = m.lastgroup, m.group()
        t = esc(text)
        if kind == "c":
            out.append(f'<span class="c">{t}</span>')
        elif kind == "s":
            out.append(f'<span class="s">{t}</span>')
        elif kind == "n":
            out.append(f'<span class="n">{t}</span>')
        elif kind == "w":
            if text in RUST_KW:
                out.append(f'<span class="k">{t}</span>')
            elif text.endswith("!"):
                out.append(f'<span class="f">{t}</span>')
            elif text[0].isupper():
                out.append(f'<span class="t">{t}</span>')
            elif pos_after(m) == "(":
                out.append(f'<span class="f">{t}</span>')
            else:
                out.append(t)
        else:
            out.append(t)
    return "".join(out)


def code_block(code, lang="rust"):
    body = highlight_rust(code) if lang == "rust" else esc(code)
    return f'<pre><code class="language-{lang}">{body}</code></pre>'


# --- page shell ------------------------------------------------------------------------------
LOGO = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 0c.6 6.2 5.8 11.4 12 12-6.2.6-11.4 5.8-12 12C11.4 17.8 6.2 12.6 0 12 6.2 11.4 11.4 6.2 12 0z"/></svg>'
HL_CSS = ".k{color:#6d28d9}.s{color:#166534}.t{color:#9a3f07}.f{color:#1d4ed8}.c{color:#6b6b74}.n{color:#a8350a}@media (prefers-color-scheme:dark){.k{color:#c792ea}.s{color:#c3e88d}.t{color:#ffcb6b}.f{color:#82aaff}.c{color:#8a90a6}.n{color:#f78c6c}}"

NAV = [("components/", "Components"), ("agents/", "AI agents"), ("dashboards/", "Dashboards"),
       ("games/", "Games"), ("motion/", "Motion"), ("docs/recipes/", "Recipes"), ("benchmarks.html", "Benchmarks")]

PAGES = []  # (relpath, url, title, description, image) for sitemap / llms / checks


def head_tags(rel, root, title, desc, image=None, image_alt=None, og_type="website", jsonld=(), noindex=False, minimal=False):
    url = BASE_URL + rel_url(rel)
    img = image or OG_DEFAULT
    img_url = BASE_URL + img
    alt = image_alt or f"glowkit: {TAGLINE}"
    w, h = (1200, 630)
    lines = [] if minimal else ['<meta charset="utf-8">', '<meta name="viewport" content="width=device-width, initial-scale=1">']
    lines += [
        f"<title>{esc(title)}</title>",
        f'<meta name="description" content="{esc(desc)}">',
        f'<link rel="canonical" href="{esc(url)}">',
        '<meta name="robots" content="noindex">' if noindex else '<meta name="robots" content="index, follow, max-image-preview:large">',
        '<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">',
        '<meta name="theme-color" content="#0b0b0d" media="(prefers-color-scheme: dark)">',
        f'<meta property="og:type" content="{og_type}">',
        f'<meta property="og:site_name" content="{SITE}">',
        f'<meta property="og:title" content="{esc(title)}">',
        f'<meta property="og:description" content="{esc(desc)}">',
        f'<meta property="og:url" content="{esc(url)}">',
        f'<meta property="og:image" content="{esc(img_url)}">',
        f'<meta property="og:image:width" content="{w}">',
        f'<meta property="og:image:height" content="{h}">',
        f'<meta property="og:image:alt" content="{esc(alt)}">',
        '<meta name="twitter:card" content="summary_large_image">',
        f'<meta name="twitter:title" content="{esc(title)}">',
        f'<meta name="twitter:description" content="{esc(desc)}">',
        f'<meta name="twitter:image" content="{esc(img_url)}">',
        f'<meta name="twitter:image:alt" content="{esc(alt)}">',
        f'<link rel="icon" href="{root}favicon.svg" type="image/svg+xml">',
        f'<link rel="icon" href="{root}favicon-32.png" sizes="32x32" type="image/png">',
        f'<link rel="apple-touch-icon" href="{root}apple-touch-icon.png">',
        f'<link rel="manifest" href="{root}site.webmanifest">',
    ]
    if not minimal:
        lines += [
            '<link rel="preconnect" href="https://fonts.googleapis.com">',
            '<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>',
            '<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Plus+Jakarta+Sans:wght@700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">',
            f'<link rel="stylesheet" href="{root}assets/site.css">',
            f"<style>{HL_CSS}</style>",
        ]
    for obj in jsonld:
        data = json.dumps(obj, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
        lines.append(f'<script type="application/ld+json">{data}</script>')
    return "\n".join(lines)


def rel_url(rel):
    """website-relative file path -> URL path (directories end with '/')."""
    if rel.endswith("index.html"):
        return rel[: -len("index.html")]
    return rel


def nav_html(root, current=None):
    links = "".join(
        '<a href="{}{}"{}>{}</a>'.format(root, href, ' aria-current="page"' if href == current else "", esc(label))
        for href, label in NAV)
    return f"""<header class="nav"><div class="wrap">
<a class="brand" href="{root}"><i>{LOGO}</i>glowkit</a>
<nav class="links" aria-label="Main">{links}</nav>
<div class="right"><a class="gh" href="{REPO}">GitHub</a><a class="btn primary sm" href="{root}gallery/">Live gallery</a></div>
</div></header>"""


def footer_html(root):
    cats = list(CATEGORIES)
    col_cats = "".join(f'<li><a href="{root}components/{CAT_SLUG[c]}/">{esc(c)}</a></li>' for c in cats[:11])
    col_cats2 = "".join(f'<li><a href="{root}components/{CAT_SLUG[c]}/">{esc(c)}</a></li>' for c in cats[11:])
    recipes = "".join(f'<li><a href="{root}docs/recipes/{r["slug"]}/">{esc(r["short"])}</a></li>' for r in RECIPES)
    return f"""<footer><div class="wrap">
<div class="cols4">
<div><h2>glowkit</h2><ul>
<li><a href="{root}">Home</a></li><li><a href="{root}gallery/">Live gallery</a></li><li><a href="{root}components/">All components</a></li>
<li><a href="{root}benchmarks.html">Benchmarks</a></li><li><a href="https://docs.rs/glowkit">API docs (docs.rs)</a></li>
<li><a href="https://crates.io/crates/glowkit">crates.io</a></li><li><a href="{REPO}">GitHub</a></li></ul></div>
<div><h2>Build</h2><ul>
<li><a href="{root}agents/">AI agent UIs</a></li><li><a href="{root}dashboards/">Dashboards</a></li>
<li><a href="{root}games/">Game menus and HUDs</a></li><li><a href="{root}motion/">Motion engine</a></li>
<li><a href="{root}docs/recipes/">Agent recipes</a></li>{recipes}</ul></div>
<div><h2>Categories</h2><ul>{col_cats}</ul></div>
<div><h2>More categories</h2><ul>{col_cats2}</ul></div>
</div>
<p class="legal">glowkit is dual-licensed under MIT or Apache-2.0. Component names from web libraries are used only to describe equivalence; all glowkit code is original Rust.</p>
</div></footer>"""


def crumbs_html(root, items):
    """items: [(label, href or None)]; last one is the current page."""
    lis = []
    for i, (label, href) in enumerate(items):
        if i == len(items) - 1:
            lis.append(f'<li><span aria-current="page">{esc(label)}</span></li>')
        else:
            lis.append(f'<li><a href="{root}{href}">{esc(label)}</a></li>')
    return f'<nav class="crumbs" aria-label="Breadcrumb"><ol>{"".join(lis)}</ol></nav>'


def crumbs_ld(items):
    out = []
    for i, (label, href) in enumerate(items, 1):
        out.append({"@type": "ListItem", "position": i, "name": label, "item": BASE_URL + (href or "")})
    return {"@context": "https://schema.org", "@type": "BreadcrumbList", "itemListElement": out}


def write_page(rel, title, desc, body, image=None, image_alt=None, og_type="website", jsonld=(), current=None,
               noindex=False, sitemap=True, priority="0.6"):
    depth = rel.count("/")
    root = "../" * depth
    doc = f"""<!doctype html>
<html lang="en">
<head>
{head_tags(rel, root, title, desc, image, image_alt, og_type, jsonld, noindex)}
</head>
<body>
{nav_html(root, current)}
<main>
{body.replace("{root}", root)}
</main>
{footer_html(root)}
</body>
</html>
"""
    path = os.path.join(WEB, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(doc)
    if sitemap:
        PAGES.append({"rel": rel, "url": BASE_URL + rel_url(rel), "title": title, "desc": desc, "priority": priority})


def card(d, root="{root}"):
    cid = d["id"]
    img = ""
    if has_thumb(cid):
        img = f'<img src="{root}{thumb_path(cid)}" width="480" height="252" loading="lazy" decoding="async" alt="{esc(d["name"])} preview">'
    return (f'<a class="card" href="{root}components/{cid}/">{img}<div class="body"><h3>{esc(d["name"])}</h3>'
            f'<small>{esc(d["category"])}</small><p>{esc(d["description"])}</p></div></a>')


def use_lines(d):
    m = mcp_by_id.get(d["id"], {})
    uses = m.get("use") or []
    if not uses:
        return "use glowkit::prelude::*;"
    groups = OrderedDict()
    for u in uses:
        mod, _, name = u.rpartition("::")
        groups.setdefault(mod, []).append(name)
    lines = ["use glowkit::prelude::*;"]
    for mod, names in groups.items():
        if len(names) == 1:
            lines.append(f"use {mod}::{names[0]};")
        else:
            one = f"use {mod}::{{{', '.join(names)}}};"
            if len(one) <= 96:
                lines.append(one)
            else:
                lines.append(f"use {mod}::{{\n    " + ",\n    ".join(names) + ",\n};")
    return "\n".join(lines)


# ---------------------------------------------------------------------------------------------
# Recipes (docs/recipes/*.md)
# ---------------------------------------------------------------------------------------------

RECIPE_META = {
    "ollama-chat": {
        "short": "Ollama chat",
        "title": "Ollama Rust GUI: local LLM chat in egui | glowkit",
        "desc": "Build a native Rust GUI for Ollama, vLLM or LM Studio: streaming chat, reasoning and tool calls in about 50 lines with glowkit-stream, egui and async-openai.",
        "talks": "Ollama, vLLM, LM Studio, OpenRouter",
    },
    "rig-agent": {
        "short": "rig agent UI",
        "title": "rig agent UI in Rust: a native desktop app | glowkit",
        "desc": "Give a rig agent a native window: streamed answers, tool calls with live arguments and results, and reasoning, drawn by glowkit's AgentThread in Rust.",
        "talks": "a rig agent in your process",
    },
    "ag-ui-desktop": {
        "short": "AG-UI desktop client",
        "title": "Rust AG-UI client: desktop front-end for agents | glowkit",
        "desc": "A native Rust AG-UI client for LangGraph, CrewAI, Mastra, Pydantic AI or ADK agents: streamed text, tool calls and human-in-the-loop approvals, built with egui.",
        "talks": "LangGraph, CrewAI, Mastra, Pydantic AI, ADK",
    },
    "vercel-ai-sdk-client": {
        "short": "Vercel AI SDK client",
        "title": "Vercel AI SDK in Rust: a native desktop client | glowkit",
        "desc": "Point a native Rust desktop app at the Vercel AI SDK route you already have: UI message streams, tool calls and reasoning drawn by glowkit. No webview.",
        "talks": "any toUIMessageStreamResponse() route",
    },
    "acp-viewer": {
        "short": "ACP agent viewer",
        "title": "ACP agent viewer in Rust: Agent Client Protocol | glowkit",
        "desc": "Watch an Agent Client Protocol coding agent in a native Rust window: plans, tool calls, file diffs and permission requests over stdio, drawn with glowkit.",
        "talks": "Agent Client Protocol agents",
    },
}
RECIPE_ORDER = ["ollama-chat", "rig-agent", "ag-ui-desktop", "vercel-ai-sdk-client", "acp-viewer"]


def inline_md(s, link_fix):
    """Inline Markdown: code spans, links, bold, italics. Escapes everything else."""
    parts = re.split(r"(`[^`]+`)", s)
    out = []
    for p in parts:
        if p.startswith("`") and p.endswith("`") and len(p) > 1:
            out.append(f"<code>{esc(p[1:-1])}</code>")
            continue
        t = esc(p)
        t = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", lambda m: f'<a href="{esc(link_fix(html.unescape(m.group(2))))}">{m.group(1)}</a>', t)
        t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", t)
        t = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", t)
        t = re.sub(r"(?<![\w_])_(?!\s)(.+?)(?<!\s)_(?![\w_])", r"<em>\1</em>", t)
        out.append(t)
    return "".join(out)


def md_to_html(md, link_fix, skip_h1=True):
    """A small Markdown converter: headings, paragraphs, fenced code, lists, tables, quotes,
    links, bold/italics and inline code. Enough for the recipes, not a general implementation."""
    lines = md.split("\n")
    out, i, n = [], 0, len(lines)
    h1 = None
    while i < n:
        line = lines[i]
        if line.startswith("```"):
            lang = line[3:].strip() or "text"
            j = i + 1
            buf = []
            while j < n and not lines[j].startswith("```"):
                buf.append(lines[j])
                j += 1
            out.append(code_block("\n".join(buf), "rust" if lang == "rust" else lang))
            i = j + 1
            continue
        m = re.match(r"(#{1,6})\s+(.*)", line)
        if m:
            level, text = len(m.group(1)), m.group(2).strip()
            if level == 1 and h1 is None:
                h1 = text
                if skip_h1:
                    i += 1
                    continue
            hid = slug(re.sub(r"`", "", text))
            out.append(f'<h{level} id="{hid}">{inline_md(text, link_fix)}</h{level}>')
            i += 1
            continue
        if re.match(r"\s*([-*+]|\d+\.)\s+", line):
            ordered = bool(re.match(r"\s*\d+\.", line))
            items = []
            while i < n and (re.match(r"\s*([-*+]|\d+\.)\s+", lines[i]) or (lines[i].startswith("  ") and lines[i].strip() and items)):
                if re.match(r"\s*([-*+]|\d+\.)\s+", lines[i]):
                    items.append(re.sub(r"^\s*([-*+]|\d+\.)\s+", "", lines[i]))
                else:
                    items[-1] += " " + lines[i].strip()
                i += 1
            tag = "ol" if ordered else "ul"
            out.append(f"<{tag}>" + "".join(f"<li>{inline_md(it, link_fix)}</li>" for it in items) + f"</{tag}>")
            continue
        if line.startswith("|") and i + 1 < n and re.match(r"^\|?\s*:?-{3,}", lines[i + 1]):
            cells = lambda l: [c.strip() for c in l.strip().strip("|").split("|")]
            head = cells(line)
            i += 2
            rows = []
            while i < n and lines[i].startswith("|"):
                rows.append(cells(lines[i]))
                i += 1
            th = "".join(f"<th>{inline_md(c, link_fix)}</th>" for c in head)
            trs = "".join("<tr>" + "".join(f"<td>{inline_md(c, link_fix)}</td>" for c in r) + "</tr>" for r in rows)
            out.append(f"<table><thead><tr>{th}</tr></thead><tbody>{trs}</tbody></table>")
            continue
        if line.startswith(">"):
            buf = []
            while i < n and lines[i].startswith(">"):
                buf.append(lines[i].lstrip(">").strip())
                i += 1
            out.append(f"<blockquote><p>{inline_md(' '.join(buf), link_fix)}</p></blockquote>")
            continue
        if not line.strip():
            i += 1
            continue
        buf = []
        while i < n and lines[i].strip() and not lines[i].startswith(("```", "#", "|", ">")) and not re.match(r"\s*([-*+]|\d+\.)\s+", lines[i]):
            buf.append(lines[i].strip())
            i += 1
        out.append(f"<p>{inline_md(' '.join(buf), link_fix)}</p>")
    return h1, "\n".join(out)


def recipe_link_fixer(src_dir_rel):
    """Rewrite links inside a recipe: other recipes -> their page, repo paths -> GitHub."""
    def fix(href):
        if re.match(r"[a-z]+:", href) or href.startswith("#"):
            return href
        path, _, frag = href.partition("#")
        name = os.path.basename(path)
        if path.endswith(".md") and os.path.dirname(path) in ("", "."):
            stem = name[:-3]
            if stem == "README":
                return "{root}docs/recipes/"
            return "{root}docs/recipes/" + stem + "/" + ("#" + frag if frag else "")
        target = os.path.normpath(os.path.join(src_dir_rel, path)).replace(os.sep, "/")
        kind = REPO_BLOB if os.path.isfile(os.path.join(ROOT, target)) else REPO_TREE
        return kind + target + ("#" + frag if frag else "")
    return fix


RECIPES = []
for stem in RECIPE_ORDER + sorted(f[:-3] for f in os.listdir(os.path.join(ROOT, "docs", "recipes"))
                                  if f.endswith(".md") and f != "README.md" and f[:-3] not in RECIPE_ORDER):
    p = os.path.join(ROOT, "docs", "recipes", stem + ".md")
    if not os.path.exists(p):
        continue
    meta = RECIPE_META.get(stem, {})
    md = open(p, encoding="utf-8").read()
    h1 = (re.search(r"^#\s+(.+)$", md, re.M) or [None, stem])[1]
    RECIPES.append({"slug": stem, "md": md, "h1": h1, "short": meta.get("short", h1),
                    "title": meta.get("title", f"{h1} in Rust | glowkit"),
                    "desc": meta.get("desc") or page_description(h1 + "."), "talks": meta.get("talks", "")})


def build_recipes():
    for r in RECIPES:
        rel = f"docs/recipes/{r['slug']}/index.html"
        h1, body = md_to_html(r["md"], recipe_link_fixer("docs/recipes"))
        others = "".join(f'<li><a href="{{root}}docs/recipes/{o["slug"]}/">{esc(o["short"])}</a></li>' for o in RECIPES if o is not r)
        items = [("Home", ""), ("Docs", "docs/recipes/"), ("Recipes", "docs/recipes/"), (r["short"], rel_url(rel))]
        items = [items[0], ("Recipes", "docs/recipes/"), items[-1]]
        page = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="cols">
<article class="prose">
<div class="hero"><div class="eyebrow">Recipe · glowkit-stream</div>
<h1>{inline_md(h1, lambda h: h)}</h1>
<div class="pills"><span class="pill">Talks to <b>{esc(r["talks"])}</b></span><span class="pill">egui 0.37</span><span class="pill">Native, no webview</span></div></div>
{body}
</article>
<aside>
<div class="box"><h2>Other recipes</h2><ul>{others}</ul></div>
<div class="box"><h2>Components used</h2><ul>
<li><a href="{{root}}components/ai-chat/">AI Chat</a></li><li><a href="{{root}}components/ai-tool-call/">Tool Call</a></li>
<li><a href="{{root}}components/ai-reasoning/">Reasoning</a></li><li><a href="{{root}}components/ai-prompt-input/">Prompt Input</a></li>
<li><a href="{{root}}components/ai-confirmation/">Confirmation</a></li><li><a href="{{root}}components/category-ai/">All AI components</a></li></ul></div>
<div class="box"><h2>Source</h2><ul><li><a href="{REPO_BLOB}docs/recipes/{r["slug"]}.md">This recipe on GitHub</a></li>
<li><a href="{REPO_TREE}crates/glowkit-stream">glowkit-stream crate</a></li><li><a href="{{root}}agents/">Building AI agent UIs</a></li></ul></div>
</aside>
</div>
</div>"""
        page = page.replace("{root}components/category-ai/", "{root}components/" + CAT_SLUG.get("AI", "ai") + "/")
        ld = [{"@context": "https://schema.org", "@type": "TechArticle", "headline": h1, "description": r["desc"],
               "url": BASE_URL + rel_url(rel), "inLanguage": "en", "dateModified": LASTMOD,
               "proficiencyLevel": "Intermediate", "programmingLanguage": "Rust",
               "isPartOf": {"@type": "WebSite", "name": SITE, "url": BASE_URL},
               "about": {"@type": "SoftwareSourceCode", "name": "glowkit-stream", "codeRepository": REPO}},
              crumbs_ld(items)]
        write_page(rel, r["title"], r["desc"], page, og_type="article", jsonld=ld, current="docs/recipes/", priority="0.7")

    # index
    rel = "docs/recipes/index.html"
    rows = "".join(
        f'<a class="feature" href="{{root}}docs/recipes/{r["slug"]}/"><h3>{esc(r["h1"])}</h3><p>{esc(r["desc"])}</p></a>'
        for r in RECIPES)
    items = [("Home", ""), ("Recipes", "docs/recipes/")]
    body = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="hero"><div class="eyebrow">Docs</div>
<h1>Connect real agents to a native Rust UI</h1>
<p class="lead">Each recipe is a complete app: what you get, the Cargo.toml, about 50 lines of code and how to run it. They all use glowkit-stream: an adapter turns the agent's stream into events, a <code>Session</code> folds them into a thread, and <code>AgentThread</code> draws it with glowkit's AI components.</p></div>
<h2>Recipes</h2>
<div class="features">{rows}</div>
<h2>The three parts</h2>
<div class="features">
<div class="feature"><h3>Adapters</h3><p>AG-UI, the Vercel AI SDK UI message stream, the Agent Client Protocol, rig and async-openai (Ollama, vLLM, LM Studio, OpenRouter) all map onto one event model.</p></div>
<div class="feature"><h3>Session</h3><p>Folds events into a thread: messages, reasoning, tool calls with partially streamed JSON arguments, approvals, plans, sources and errors. Plain data you can inspect and save.</p></div>
<div class="feature"><h3>AgentThread</h3><p>Renders the session with streaming Markdown, tool-call cards and approval prompts, and returns the user's decisions so you can send them back to the agent.</p></div>
</div>
{code_block('rx.drain_into(&mut session);                        // events from any thread or runtime' + chr(10) + 'let out = AgentThread::new(&mut session).show(ui);  // the whole thread' + chr(10) + 'for decision in out.decisions { /* send the approval back */ }')}
</div>"""
    desc = "Recipes for connecting real AI agents to a native Rust UI: Ollama, rig, AG-UI, the Vercel AI SDK and ACP, each a complete egui app in about 50 lines of code."
    write_page(rel, "Agent recipes: Ollama, rig, AG-UI, Vercel AI SDK, ACP | glowkit",
               desc, body, jsonld=[crumbs_ld(items)], current="docs/recipes/", priority="0.7")


# ---------------------------------------------------------------------------------------------
# Component pages
# ---------------------------------------------------------------------------------------------

def related_for(d, k=6):
    same = CATEGORIES[d["category"]]
    out = []
    for rid in mcp_by_id.get(d["id"], {}).get("related", []):
        r = by_id.get(rid)
        if r and r["category"] == d["category"] and r is not d and r not in out:
            out.append(r)
    i = same.index(d)
    ring = same[i + 1:] + same[:i]
    for r in ring:
        if len(out) >= k:
            break
        if r not in out:
            out.append(r)
    return out[:k]


def build_components():
    for d in demos:
        cid, name, cat = d["id"], display_name(d), d["category"]
        noun = CAT_NOUN[cat]
        rel = f"components/{cid}/index.html"
        title = component_title(name, noun)
        desc = component_description(d)
        m = mcp_by_id.get(cid, {})
        crate = m.get("crate", "glowkit")
        feature = m.get("feature")
        items = [("Home", ""), ("Components", "components/"), (cat, f"components/{CAT_SLUG[cat]}/"), (d["name"], rel_url(rel))]
        # equivalents
        eq = []
        for c in d["covers"]:
            lib, item = parse_cover(c)
            if lib:
                url = lib_urls.get(lib)
                libh = f'<a class="link" href="{esc(url)}" rel="nofollow noopener">{esc(lib)}</a>' if url else esc(lib)
                eq.append(f"<li><b>{esc(item)}</b><span>{libh}</span></li>")
            else:
                eq.append(f"<li><b>{esc(item)}</b></li>")
        libs = []
        for c in d["covers"]:
            lib, _ = parse_cover(c)
            if lib and lib not in libs:
                libs.append(lib)
        if eq:
            lib_phrase = ", ".join(libs[:-1]) + (" and " if len(libs) > 1 else "") + libs[-1] if libs else "web component libraries"
            eq_html = (f"<h2>Equivalent to</h2><p>If you know this pattern from {esc(lib_phrase)}, {esc(d['name'])} is the Rust version: "
                       f"an original egui implementation, not a port, that runs natively and in WebAssembly.</p>"
                       f'<ul class="equiv">{"".join(eq)}</ul>')
        else:
            eq_html = ""
        img_html = ""
        image = None
        alt = f"{d['name']}, a glowkit {noun} component, rendered in the light theme"
        if has_img(cid):
            image = img_path(cid)
            img_html = (f'<figure class="shot"><img src="{{root}}{image}" width="1200" height="630" alt="{esc(alt)}" fetchpriority="high">'
                        f'<figcaption>Screenshot of the live demo. <a class="link" href="{{root}}gallery/#{esc(cid)}">Open it in the gallery</a> to interact with it and change the theme.</figcaption></figure>')
        usage = use_lines(d) + "\n\n" + d["code"]
        rel_items = related_for(d)
        related_html = "".join(card(r) for r in rel_items)
        cli = ""
        if cid in cli_ids:
            cli = (f"<p>Prefer to own the code? The glowkit CLI copies this component's source into your project, shadcn-style:</p>"
                   + code_block(f"cargo install glowkit-cli\nglowkit add {cid}", "bash"))
        pills = [f'<span class="pill">Category <b>{esc(cat)}</b></span>', f'<span class="pill">Crate <b>{esc(crate)}</b></span>']
        if feature:
            pills.append(f'<span class="pill">Feature <b>{esc(feature)}</b></span>')
        pills.append('<span class="pill">egui 0.37</span>')
        cat_copy = CAT_COPY[cat][1]
        feat_note = (f" It lives in the <code>{esc(crate)}</code> crate, enabled by the <code>{esc(feature)}</code> feature of <code>glowkit</code> (on by default)."
                     if feature else "")
        body = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="cols">
<article>
<div class="hero"><div class="eyebrow">{esc(cat)}</div>
<h1>{esc(d["name"])}</h1>
<p class="lead">{esc(d["description"])}</p>
<div class="pills">{"".join(pills)}</div>
<div class="actions"><a class="btn primary" href="{{root}}gallery/#{esc(cid)}">Try it live</a><a class="btn" href="{{root}}components/{CAT_SLUG[cat]}/">More {esc(cat)}</a></div>
</div>
{img_html}
<h2>Usage</h2>
<p>Add glowkit next to eframe:</p>
{code_block("cargo add glowkit eframe", "bash")}
<p>Then draw it inside any egui <code>Ui</code>:</p>
{code_block(usage)}
{cli}
{eq_html}
<h2>About {esc(d["name"])}</h2>
<p>{esc(d["name"])} is one of {len(CATEGORIES[cat])} components in glowkit's {esc(cat)} category.{feat_note} It reads its colors from the active glowkit theme, so it matches the rest of your app in light, dark or a custom brand theme, and every color, size and timing has a setter to override it.</p>
<p class="muted">{esc(cat_copy)}</p>
</article>
<aside>
<div class="box"><h2>Install</h2>{code_block("cargo add glowkit", "bash")}<ul><li><a href="https://docs.rs/glowkit">API docs on docs.rs</a></li><li><a href="{REPO_TREE}crates/{esc(crate)}">{esc(crate)} source</a></li></ul></div>
<div class="box"><h2>{esc(cat)}</h2><ul>{"".join(f'<li><a href="{{root}}components/{r["id"]}/">{esc(r["name"])}</a></li>' for r in rel_items)}<li><a href="{{root}}components/{CAT_SLUG[cat]}/">All {len(CATEGORIES[cat])} {esc(cat)} components</a></li></ul></div>
</aside>
</div>
<h2>More in {esc(cat)}</h2>
<div class="grid">{related_html}</div>
</div>"""
        ssc = {"@context": "https://schema.org", "@type": "SoftwareSourceCode", "name": f"{d['name']} (glowkit)",
               "description": d["description"], "url": BASE_URL + rel_url(rel), "codeRepository": REPO,
               "programmingLanguage": {"@type": "ComputerLanguage", "name": "Rust"},
               "runtimePlatform": "egui 0.37 (Windows, macOS, Linux, WebAssembly)",
               "codeSampleType": "code snippet", "text": usage, "license": "https://spdx.org/licenses/MIT.html",
               "keywords": ", ".join([d["name"], cat, "Rust", "egui", "animated UI"] + [parse_cover(c)[1] for c in d["covers"][:4]]),
               "isPartOf": {"@type": "SoftwareSourceCode", "name": "glowkit", "codeRepository": REPO},
               "dateModified": LASTMOD}
        if image:
            ssc["image"] = BASE_URL + image
        write_page(rel, title, desc, body, image=image, image_alt=alt if image else None, og_type="article",
                   jsonld=[ssc, crumbs_ld(items)], current="components/", priority="0.6")


def build_categories():
    for cat, ds in CATEGORIES.items():
        rel = f"components/{CAT_SLUG[cat]}/index.html"
        h1, copy = CAT_COPY[cat]
        title = f"{cat} components for Rust and egui | {SITE}"
        desc = category_description(cat, ds)
        items = [("Home", ""), ("Components", "components/"), (cat, rel_url(rel))]
        other = "".join(f'<a class="chip" href="{{root}}components/{CAT_SLUG[c]}/">{esc(c)}<b>{len(v)}</b></a>' for c, v in CATEGORIES.items() if c != cat)
        image = next((img_path(d["id"]) for d in ds if has_img(d["id"])), None)
        body = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="hero"><div class="eyebrow">{len(ds)} components · {esc(cat)}</div>
<h1>{esc(h1)}</h1>
<p class="lead">{esc(copy)}</p>
<div class="actions"><a class="btn primary" href="{{root}}gallery/#{esc(ds[0]["id"])}">Open in the live gallery</a><a class="btn" href="{{root}}components/">All categories</a></div></div>
<h2>All {esc(cat)} components</h2>
<div class="grid">{"".join(card(d) for d in ds)}</div>
<h2>Other categories</h2>
<div class="chips">{other}</div>
</div>"""
        ld = [{"@context": "https://schema.org", "@type": "CollectionPage", "name": h1, "description": desc,
               "url": BASE_URL + rel_url(rel), "isPartOf": {"@type": "WebSite", "name": SITE, "url": BASE_URL},
               "mainEntity": {"@type": "ItemList", "numberOfItems": len(ds), "itemListElement": [
                   {"@type": "ListItem", "position": i, "url": BASE_URL + f"components/{d['id']}/", "name": d["name"]}
                   for i, d in enumerate(ds, 1)]}},
              crumbs_ld(items)]
        write_page(rel, title, desc, body, image=image, image_alt=f"A glowkit {CAT_NOUN[cat]} component" if image else None,
                   jsonld=ld, current="components/", priority="0.8")


def build_components_index():
    rel = "components/index.html"
    items = [("Home", ""), ("Components", "components/")]
    chips = "".join(f'<a class="chip" href="#{CAT_SLUG[c]}">{esc(c)}<b>{len(v)}</b></a>' for c, v in CATEGORIES.items())
    sections = []
    for c, ds in CATEGORIES.items():
        lis = "".join(f'<li><a href="{{root}}components/{d["id"]}/">{esc(d["name"])}</a></li>' for d in ds)
        sections.append(f'<h2 id="{CAT_SLUG[c]}"><a href="{{root}}components/{CAT_SLUG[c]}/">{esc(c)}</a> <span class="muted">({len(ds)})</span></h2>'
                        f'<p class="muted">{esc(CAT_COPY[c][1])}</p><ul class="list">{lis}</ul>')
    featured = ["app-agent-sessions", "app-trading-terminal", "game-hud", "ai-chat", "node-editor", "globe-3d", "aurora", "line-chart"]
    feat = "".join(card(by_id[i]) for i in featured if i in by_id)
    desc = page_description(f"Browse all {N} glowkit components for Rust and egui: AI agent UIs, charts, dashboards, game menus and HUDs, 3D, text effects, backgrounds and templates.")
    body = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="hero"><div class="eyebrow">Components</div>
<h1>{N} animated components <span>for Rust and egui</span></h1>
<p class="lead">Every component here runs natively on Windows, macOS and Linux and in the browser through WebAssembly. Each page has the usage code, the web components it matches and a link to try it live.</p>
<div class="chips">{chips}</div></div>
<h2>Highlights</h2>
<div class="grid">{feat}</div>
{"".join(sections)}
</div>"""
    ld = [{"@context": "https://schema.org", "@type": "CollectionPage", "name": f"glowkit components ({N})", "description": desc,
           "url": BASE_URL + "components/", "isPartOf": {"@type": "WebSite", "name": SITE, "url": BASE_URL}}, crumbs_ld(items)]
    write_page(rel, f"All {N} animated UI components for Rust (egui) | {SITE}", desc, body, jsonld=ld,
               current="components/", priority="0.9")


# ---------------------------------------------------------------------------------------------
# Audience landing pages
# ---------------------------------------------------------------------------------------------

AGENT_CODE = """use glowkit::prelude::*;
use glowkit_stream::{AgentThread, Session};

// Your agent runs anywhere: a tokio task, a thread, another process.
// Its adapter (AG-UI, Vercel AI SDK, ACP, rig, async-openai) sends events.
rx.drain_into(&mut session);

// One call renders the whole thread: streaming Markdown, reasoning,
// tool calls with live arguments, plans, sources and errors.
let out = AgentThread::new(&mut session).show(ui);
for decision in out.decisions {
    // the user approved or denied a tool call: send it back to the agent
}"""

LANDINGS = [
    {
        "slug": "agents", "current": "agents/",
        "title": "Rust AI agent UI: chat, tool calls and approvals | glowkit",
        "desc": "Build native AI agent UIs in Rust: streaming chat, tool calls, approvals, diffs, traces and agent fleets in egui, wired to AG-UI, the Vercel AI SDK, ACP or rig.",
        "eyebrow": "For AI agent apps",
        "h1": "AI agent interfaces <span>in Rust, native and fast</span>",
        "lead": "Session managers, local LLM chat, agent fleets and coding-agent supervisors are moving off Electron. glowkit gives them the interface layer: streaming Markdown chat, reasoning, tool calls, approvals, diff review, traces and run monitors, drawn natively with egui.",
        "features": [
            ("Streams from real agents", "glowkit-stream turns AG-UI, Vercel AI SDK, ACP, rig and async-openai streams into one event model. Tool-call arguments render while the JSON is still arriving."),
            ("Human in the loop", "Approval cards, confirmation prompts, approval queues and error-recovery cards return the user's decision to your code, ready to send back to the agent."),
            ("Supervise many agents", "Agent trees, fleet grids, run monitors and timelines, trace waterfalls, cost meters and context-window views for apps that watch more than one run."),
            ("Local models", "A complete local LLM chat template with tokens per second, a model switcher and RAM, VRAM and GPU meters, plus an Ollama recipe in about 50 lines."),
            ("MCP and skills", "MCP server lists with health, a skills browser, memory viewer and schedule list for agent management apps."),
            ("Small and native", "One Rust binary for Windows, macOS and Linux, or a WebAssembly build. No Electron, no webview, no JavaScript runtime."),
        ],
        "ids": ["app-agent-sessions", "app-local-llm", "ai-chat", "ai-tool-call", "ai-reasoning", "approval-queue",
                "diff-review", "trace-waterfall", "agent-tree", "run-monitor", "context-window-viz", "mcp-server-list"],
        "code_title": "Render a live agent thread",
        "code_intro": "Add <code>glowkit</code> and <code>glowkit-stream</code>, connect an adapter, and let <code>AgentThread</code> draw the conversation. The recipes below are complete apps.",
        "code": AGENT_CODE,
        "links_title": "Recipes: connect your agent",
        "links": "recipes",
        "cats": ["AI", "Templates", "Data Display"],
    },
    {
        "slug": "dashboards", "current": "dashboards/",
        "title": "egui dashboards: animated charts and KPIs in Rust | glowkit",
        "desc": "Build egui dashboards in Rust: animated line, area and candlestick charts, KPI tiles, Sankey, heatmaps, data grids and app shells, native or in WebAssembly.",
        "eyebrow": "For dashboards",
        "h1": "Dashboards in egui <span>that feel alive</span>",
        "lead": "Trading terminals, crypto and infrastructure monitors, analytics and internal tools: glowkit adds charts that draw in and track the pointer, numbers that count to their value, and app shells that hold it all together, in a native Rust binary.",
        "features": [
            ("Charts that respond", "Line, area, bar, pie, radar, scatter, heatmap, treemap, funnel, candlestick and Sankey charts animate in, snap a crosshair to the nearest point and show tooltips."),
            ("Live numbers", "KPI tiles with counting values, delta badges and sparklines; tracker and uptime bars; gauges and activity rings that ease between updates instead of jumping."),
            ("Real templates", "A trading terminal with order book and depth, an analytics dashboard, a finance dashboard, a CRM pipeline and a developer console you can start from."),
            ("App shells", "Sidebar, inset, floating, rail, dual-pane, top-nav and inspector layouts that collapse with springs, plus a command palette and data grid."),
            ("Idle stays idle", "One-shot animations stop requesting repaints when they finish, so a dashboard that is not changing does not burn CPU."),
            ("Themeable", "Chart palettes come from semantic theme tokens. Switch light and dark, or set your brand colors once for every chart and egui widget."),
        ],
        "ids": ["app-trading-terminal", "app-analytics", "app-finance", "candlestick-chart", "line-chart", "area-chart",
                "stat-tile", "board-sankey-card", "heatmap", "data-grid", "shell-sidebar", "gauge"],
        "code_title": "A KPI row and a chart",
        "code_intro": "Components take your data as slices and draw into any egui layout. Hover and animation state is kept for you.",
        "code_ids": ["stat-tile", "line-chart"],
        "links_title": "Keep exploring",
        "links": "cats",
        "cats": ["Charts", "Dashboards & Shells", "Data Display", "Templates", "Canvas & Editors"],
    },
    {
        "slug": "games", "current": "games/",
        "title": "egui game UI: menus, HUDs and inventories in Rust | glowkit",
        "desc": "Game UI for Rust and egui: animated main and pause menus, HUD bars, minimaps, inventories, skill trees, dialogue and loot popups. Works in Bevy via bevy_egui.",
        "eyebrow": "For games",
        "h1": "Game menus and HUDs <span>for Rust games</span>",
        "lead": "glowkit's game UI blocks take plain game state every frame and handle the motion: damage trails on health bars, cooldown sweeps, letters dropping into a title screen, loot that pops and achievements that slide in. They draw into any egui Ui, so they work in Bevy through bevy_egui or in your own engine.",
        "features": [
            ("Menus that move", "Main menus over a parallax scene, pause menus, settings panels and character select screens, with staggered entries, hover slides and a sound-cue hook for every hover and confirm."),
            ("HUDs", "Health, mana and stamina bars with damage trails, boss bars, ammo counters with reload states, cooldown icons, hotbars, dynamic crosshairs, kill feeds and damage numbers."),
            ("Inventory and progression", "Inventories with item tooltips, skill trees, quest trackers and logs, level-up banners, loot popups and achievement toasts."),
            ("Stateless by design", "Pass the numbers from your game loop each frame. glowkit keeps only animation state, so it never fights your ECS or save system."),
            ("Bevy and beyond", "Use it with bevy_egui (pick a release built on egui 0.37), with eframe, or with any egui integration in your own renderer."),
            ("Styles", "Sci-fi and fantasy styles, plus theme tokens for your own palette, radius and glow."),
        ],
        "ids": ["game-hud", "game-main-menu", "game-pause-menu", "game-inventory", "game-skill-tree", "game-dialogue-box",
                "game-minimap", "game-loot-popup", "game-achievement-toast", "game-settings-panel", "game-character-select", "game-scoreboard"],
        "code_title": "A HUD from plain game state",
        "code_intro": "Every block takes your values each frame. Here is the HUD from the demo above, and a main menu that reports sound cues.",
        "code_ids": ["game-hud", "game-main-menu"],
        "links_title": "Keep exploring",
        "links": "cats",
        "cats": ["Game UI", "3D", "Text Animations", "Backgrounds", "Overlays & Feedback"],
    },
    {
        "slug": "motion", "current": "motion/",
        "title": "Framer Motion for Rust: springs and layout in egui | glowkit",
        "desc": "A Framer-Motion-style animation engine for Rust and egui: springs, keyframes, timelines, staggered variants, exit presence, layout animation and drag.",
        "eyebrow": "Motion engine",
        "h1": "Framer Motion ideas, <span>in Rust</span>",
        "lead": "glowkit-motion brings the animation model web developers know from Framer Motion and GSAP to egui: animate any value with a spring, orchestrate children with variants, animate things out before they disappear, and let layouts glide instead of jump.",
        "features": [
            ("animate() and springs", "Animate f32, Vec2, Pos2, Rect, Color32 or your own type toward a target with a spring (stiffness, damping, mass) or a tween with one of 31 easing curves."),
            ("MotionBox", "The motion.div idea: initial props, hover and tap states, and spring transitions on a container whose children stay fully interactive."),
            ("Variants and stagger", "Switch one variant name and every child follows, staggered from the first or from the center outward, with delays for children."),
            ("Presence", "The AnimatePresence idea: exit animations for content that is about to disappear, and keyed lists where items enter, exit and the rest slide into place."),
            ("Layout and shared elements", "FLIP-style layout animation keyed by identity, and shared elements that fly between two places, like layoutId."),
            ("Timelines, drag and scroll", "GSAP-style timelines with labels and overlaps, drag with momentum, elastic edges and snapping, scroll-linked values, motion paths and shape morphing."),
        ],
        "ids": ["spring-playground", "motion-box", "stagger-variants", "presence", "layout-animation", "shared-element",
                "motion-timeline", "drag", "reorder", "scroll-linked", "motion-path", "shape-morph"],
        "code_title": "Springs, hover states and presence",
        "code_intro": "If you have written <code>&lt;motion.div whileHover&gt;</code> or <code>&lt;AnimatePresence&gt;</code>, this will feel familiar.",
        "code_ids": ["motion-box", "presence"],
        "table": [("motion.div with animate / whileHover / whileTap", "MotionBox", "motion-box"),
                  ("useSpring, animate()", "animate(ui, id, target, Transition::spring(..))", "spring-playground"),
                  ("variants, staggerChildren", "MotionGroup + Variants", "stagger-variants"),
                  ("AnimatePresence", "Presence, PresenceList", "presence"),
                  ("layout, layoutId", "LayoutAnim", "layout-animation"),
                  ("drag, dragConstraints, dragElastic", "Draggable", "drag"),
                  ("Reorder.Group", "Reorder", "reorder"),
                  ("useScroll, useTransform", "scroll_offset, transform, InView", "scroll-linked"),
                  ("keyframes, repeat, repeatType", "Keyframes", "keyframes"),
                  ("GSAP timeline", "Timeline + Tween", "motion-timeline")],
        "links_title": "Keep exploring",
        "links": "cats",
        "cats": ["Motion Engine", "Motion & Layout", "Micro-interactions", "Text Animations", "Cursor & Interaction"],
    },
]


def build_landings():
    for L in LANDINGS:
        rel = f"{L['slug']}/index.html"
        crumb = {"agents": "AI agent UIs", "dashboards": "Dashboards", "games": "Game UI", "motion": "Motion engine"}.get(L["slug"], L["slug"])
        items = [("Home", ""), (crumb, rel_url(rel))]
        feats = "".join(f'<div class="feature"><h3>{esc(a)}</h3><p>{esc(b)}</p></div>' for a, b in L["features"])
        comps = [by_id[i] for i in L["ids"] if i in by_id]
        grid = "".join(card(d) for d in comps)
        if "code" in L:
            code = L["code"]
        else:
            code = "\n\n".join(use_lines(by_id[i]).replace("use glowkit::prelude::*;\n", "") + "\n" + by_id[i]["code"] for i in L["code_ids"])
            code = "use glowkit::prelude::*;\n" + code
        table = ""
        if L.get("table"):
            rows = "".join(f'<tr><td><code>{esc(a)}</code></td><td><code>{esc(b)}</code></td><td><a class="link" href="{{root}}components/{c}/">{esc(by_id[c]["name"])}</a></td></tr>'
                           for a, b, c in L["table"] if c in by_id)
            table = (f'<h2>From Framer Motion to glowkit</h2><p class="muted">The same concepts, mapped to their glowkit names. Each links to a page with a live demo.</p>'
                     f'<div class="prose"><table><thead><tr><th>Framer Motion / GSAP</th><th>glowkit</th><th>Demo</th></tr></thead><tbody>{rows}</tbody></table></div>')
        if L["links"] == "recipes":
            links = "".join(f'<a class="feature" href="{{root}}docs/recipes/{r["slug"]}/"><h3>{esc(r["h1"])}</h3><p>Talks to {esc(r["talks"])}.</p></a>' for r in RECIPES)
            links = f'<div class="features">{links}</div>'
        else:
            links = ""
        cats = "".join(f'<a class="chip" href="{{root}}components/{CAT_SLUG[c]}/">{esc(c)}<b>{len(CATEGORIES[c])}</b></a>' for c in L["cats"] if c in CATEGORIES)
        hero_img = ""
        image = img_path(L["ids"][0]) if has_img(L["ids"][0]) else None
        if image:
            hero_img = (f'<figure class="shot"><img src="{{root}}{image}" width="1200" height="630" alt="{esc(by_id[L["ids"][0]]["name"])}, built with glowkit" fetchpriority="high">'
                        f'<figcaption>{esc(by_id[L["ids"][0]]["name"])}: {esc(by_id[L["ids"][0]]["description"])}</figcaption></figure>')
        body = f"""<div class="wrap">
{crumbs_html("{root}", items)}
<div class="hero"><div class="eyebrow">{esc(L["eyebrow"])}</div>
<h1>{L["h1"]}</h1>
<p class="lead">{esc(L["lead"])}</p>
<div class="actions"><a class="btn primary" href="{{root}}gallery/#{esc(L["ids"][0])}">See it live</a><a class="btn" href="#components">Best components</a></div></div>
{hero_img}
<h2>Why glowkit</h2>
<div class="features">{feats}</div>
<h2 id="components">Best components</h2>
<div class="grid">{grid}</div>
<h2>{esc(L["code_title"])}</h2>
<p class="muted">{L["code_intro"]}</p>
{code_block("cargo add glowkit eframe" + (chr(10) + 'cargo add glowkit-stream --features "egui http"' if L["slug"] == "agents" else ""), "bash")}
{code_block(code)}
{table}
<h2>{esc(L["links_title"])}</h2>
{links}
<div class="chips">{cats}<a class="chip" href="{{root}}components/">All {N} components</a></div>
<div class="cta"><h2>Make your Rust app glow</h2><p class="muted">Open source, MIT or Apache-2.0. One <code>cargo add</code> away.</p>
<div class="actions"><a class="btn primary" href="{{root}}gallery/">Open the live gallery</a><a class="btn" href="{REPO}">Star on GitHub</a></div></div>
</div>"""
        ld = [{"@context": "https://schema.org", "@type": "WebPage", "name": L["title"], "description": L["desc"],
               "url": BASE_URL + rel_url(rel), "isPartOf": {"@type": "WebSite", "name": SITE, "url": BASE_URL},
               "about": {"@type": "SoftwareSourceCode", "name": "glowkit", "codeRepository": REPO, "programmingLanguage": "Rust"}},
              crumbs_ld(items)]
        write_page(rel, L["title"], L["desc"], body, image=image,
                   image_alt=f"{by_id[L['ids'][0]]['name']} built with glowkit" if image else None,
                   jsonld=ld, current=L["current"], priority="0.9")


# ---------------------------------------------------------------------------------------------
# 404, sitemap, robots, llms
# ---------------------------------------------------------------------------------------------

def build_404():
    # GitHub Pages serves this file for any missing path, at any depth: absolute links only.
    body = f"""<div class="wrap narrow">
<div class="hero" style="padding-top:64px"><div class="eyebrow">404</div>
<h1>This page is not here.</h1>
<p class="lead">The link may be old, or the page may have moved. Everything glowkit has is a click away:</p>
<div class="actions"><a class="btn primary" href="{BASE_URL}">Home</a><a class="btn" href="{BASE_URL}components/">All components</a><a class="btn" href="{BASE_URL}gallery/">Live gallery</a></div></div>
<h2>Popular pages</h2>
<div class="chips"><a class="chip" href="{BASE_URL}agents/">AI agent UIs</a><a class="chip" href="{BASE_URL}dashboards/">Dashboards</a><a class="chip" href="{BASE_URL}games/">Game UI</a><a class="chip" href="{BASE_URL}motion/">Motion engine</a><a class="chip" href="{BASE_URL}docs/recipes/">Agent recipes</a><a class="chip" href="{BASE_URL}benchmarks.html">Benchmarks</a></div>
</div>"""
    doc_root = BASE_URL
    path = "404.html"
    html_doc = f"""<!doctype html>
<html lang="en">
<head>
{head_tags(path, doc_root, f"Page not found | {SITE}", "This page does not exist. Browse glowkit's animated Rust UI components, the live WebAssembly gallery, the agent recipes and the benchmarks page instead.", noindex=True)}
</head>
<body>
{nav_html(doc_root)}
<main>
{body}
</main>
{footer_html(doc_root)}
</body>
</html>
"""
    open(os.path.join(WEB, path), "w", encoding="utf-8").write(html_doc)


STATIC_PAGES = [  # hand-written pages: their <head> SEO block and footer are filled in by build_blocks()
    {"rel": "index.html",
     "title": "glowkit: animated UI components for Rust and egui",
     "desc": f"{TAGLINE}: {N_ROUND} egui components for AI agent UIs, dashboards, games and dev tools. Native on Windows, macOS, Linux and WebAssembly."},
    {"rel": "gallery/index.html",
     "title": "Live gallery: Rust egui components in WebAssembly | glowkit",
     "desc": f"Every glowkit component live in your browser: {N_ROUND} animated Rust UI components compiled from egui to WebAssembly. AI agents, charts, game UI, 3D and more."},
    {"rel": "benchmarks.html",
     "title": "Benchmarks: frame time and memory per component | glowkit",
     "desc": "Frame time, tessellation, vertex counts and memory for every glowkit component, measured headlessly in release builds, plus idle-repaint checks for each one."},
]


def site_jsonld():
    desc = STATIC_PAGES[0]["desc"]
    return {"@context": "https://schema.org", "@graph": [
        {"@type": "WebSite", "@id": BASE_URL + "#website", "name": SITE, "url": BASE_URL, "description": desc, "inLanguage": "en"},
        {"@type": "SoftwareApplication", "@id": BASE_URL + "#app", "name": SITE, "url": BASE_URL,
         "description": f"An animated UI component library for Rust apps built on egui: {N_ROUND} components for AI agent interfaces, dashboards, game UI, 3D and motion.",
         "applicationCategory": "DeveloperApplication", "applicationSubCategory": "UI component library",
         "operatingSystem": "Windows, macOS, Linux, Web (WebAssembly)", "softwareVersion": VERSION,
         "image": BASE_URL + OG_DEFAULT, "license": "https://opensource.org/licenses/MIT",
         "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
         "downloadUrl": "https://crates.io/crates/glowkit", "softwareHelp": "https://docs.rs/glowkit",
         "sameAs": [REPO, "https://crates.io/crates/glowkit"]},
        {"@type": "SoftwareSourceCode", "@id": BASE_URL + "#code", "name": SITE, "codeRepository": REPO,
         "programmingLanguage": {"@type": "ComputerLanguage", "name": "Rust"}, "runtimePlatform": "egui 0.37",
         "license": ["https://opensource.org/licenses/MIT", "https://opensource.org/licenses/Apache-2.0"],
         "targetProduct": {"@id": BASE_URL + "#app"}},
    ]}


def build_manifest():
    data = {"name": "glowkit", "short_name": "glowkit", "description": TAGLINE + ".",
            "start_url": "./", "scope": "./", "display": "standalone",
            "background_color": "#ffffff", "theme_color": "#111113",
            "icons": [{"src": "icon-192.png", "sizes": "192x192", "type": "image/png"},
                      {"src": "icon-512.png", "sizes": "512x512", "type": "image/png"},
                      {"src": "favicon.svg", "sizes": "any", "type": "image/svg+xml"}]}
    with open(os.path.join(WEB, "site.webmanifest"), "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")


def build_sitemap():
    entries = []
    for sp in STATIC_PAGES:
        entries.append((BASE_URL + rel_url(sp["rel"]), None))
    for p in PAGES:
        entries.append((p["url"], None))
    xml = ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">']
    for url, pr in entries:
        xml.append(f"  <url><loc>{esc(url)}</loc><lastmod>{LASTMOD}</lastmod></url>")
    xml.append("</urlset>")
    open(os.path.join(WEB, "sitemap.xml"), "w", encoding="utf-8").write("\n".join(xml) + "\n")
    open(os.path.join(WEB, "robots.txt"), "w", encoding="utf-8").write(
        f"User-agent: *\nAllow: /\n\nSitemap: {BASE_URL}sitemap.xml\n")
    return len(entries)


def build_llms():
    recipes = "\n".join(f"- [{r['h1']}]({BASE_URL}docs/recipes/{r['slug']}/): talks to {r['talks']}." for r in RECIPES)
    txt = f"""# glowkit

> {TAGLINE}. glowkit is an open-source Rust UI library of {N_ROUND} animated components built on egui 0.37: AI agent interfaces, dashboards and charts, game menus and HUDs, node editors, 3D, text effects, backgrounds, micro-interactions and full app templates. It runs natively on Windows, macOS and Linux and in the browser through WebAssembly. 100% Rust; no Electron, no webview. Dual-licensed MIT or Apache-2.0.

Install with `cargo add glowkit` (all component crates are default features of the `glowkit` umbrella crate). Components are egui widgets: `ui.add(ShinyText::new("Hi"))`. `use glowkit::prelude::*;` brings in Theme, Easing, Spring and Trigger. Install a theme once with `Theme::dark().install(ctx)` (presets: dark, light, ocean, ember, matrix).

## Key pages

- [Home]({BASE_URL}): overview and quick start.
- [All components]({BASE_URL}components/): every component, grouped by category, one page each with usage code.
- [Live gallery]({BASE_URL}gallery/): the full component gallery compiled to WebAssembly (a canvas; use the component pages for text).
- [AI agent UIs]({BASE_URL}agents/), [Dashboards]({BASE_URL}dashboards/), [Game UI]({BASE_URL}games/), [Motion engine]({BASE_URL}motion/).
- [Benchmarks]({BASE_URL}benchmarks.html): frame time, tessellation and memory for every component.
- [llms-full.txt]({BASE_URL}llms-full.txt): every component with its description, `use` path and URL.

## Categories

{len(CATEGORIES)} categories: {", ".join(CATEGORIES)}. Each has a page under {BASE_URL}components/.

## Agent recipes (glowkit-stream)

{recipes}

## Crates

- `glowkit`: umbrella crate; re-exports everything below behind cargo features.
- `glowkit-core`: theme tokens, easings, springs, clocks, gradients, glows, noise, per-glyph text.
- `glowkit-text` (feature `text`), `glowkit-backgrounds` (`backgrounds`), `glowkit-effects` (`effects`), `glowkit-components` (`components`), `glowkit-charts` (`charts`), `glowkit-ai` (`ai`), `glowkit-blocks` (`blocks`), `glowkit-motion` (`motion`), `glowkit-3d` (`three-d`, a software 3D renderer), `glowkit-canvas` (`canvas`), `glowkit-templates` (`templates`), `glowkit-interactions` (`interactions`), `glowkit-agents` (`agents`).
- `glowkit-stream` (`stream`): drives the AI components from real agent streams: AG-UI, Vercel AI SDK, ACP, rig and async-openai adapters; `Session` and `AgentThread`.
- `glowkit-mcp`: an MCP server for coding agents with component search, exact `use` paths, constructors and setters, templates, theme tokens and egui 0.37 notes. `cargo install glowkit-mcp`, then `claude mcp add glowkit -- glowkit-mcp`.
- `glowkit-cli`: the `glowkit` command: `glowkit list`, `glowkit info <id>`, `glowkit add <id>` (copy a component's source into your project), `glowkit init <name>`.

## Links

- Source: {REPO}
- API docs: https://docs.rs/glowkit
- crates.io: https://crates.io/crates/glowkit
"""
    open(os.path.join(WEB, "llms.txt"), "w", encoding="utf-8").write(txt)

    full = [f"# glowkit: full component list", "",
            f"> {N} showcased components of glowkit, an animated UI library for Rust on egui 0.37 (native Windows/macOS/Linux and WebAssembly). "
            f"Each entry: name, id, `use` path(s), crate and feature, description, web equivalents and its page. "
            f"Install with `cargo add glowkit`; every component crate is a default feature of `glowkit`. Usage snippets are on each page.", ""]
    for c, ds in CATEGORIES.items():
        full += [f"## {c} ({len(ds)})", ""]
        for d in ds:
            m = mcp_by_id.get(d["id"], {})
            uses = ", ".join(f"`{u}`" for u in (m.get("use") or [])) or "`glowkit::prelude::*`"
            crate = m.get("crate", "glowkit")
            feat = f", feature `{m['feature']}`" if m.get("feature") else ""
            eq = f" Equivalent to: {covers_text(d, 6)}." if d["covers"] else ""
            full.append(f"- **{d['name']}** (`{d['id']}`): {uses}; crate `{crate}`{feat}. {d['description']}{eq} {BASE_URL}components/{d['id']}/")
        full.append("")
    open(os.path.join(WEB, "llms-full.txt"), "w", encoding="utf-8").write("\n".join(full))


# ---------------------------------------------------------------------------------------------
# Generated blocks inside hand-written pages
# ---------------------------------------------------------------------------------------------

def replace_block(path, name, content):
    p = os.path.join(WEB, path)
    if not os.path.exists(p):
        return
    s = open(p, encoding="utf-8").read()
    pat = re.compile(rf"(<!-- gen:{name} -->)(.*?)(<!-- /gen:{name} -->)", re.S)
    if not pat.search(s):
        print(f"  note: no gen:{name} block in {path}")
        return
    s = pat.sub(lambda m: m.group(1) + "\n" + content + "\n" + m.group(3), s)
    open(p, "w", encoding="utf-8").write(s)


def build_blocks():
    cats = "".join(f'<a class="cat" href="components/{CAT_SLUG[c]}/">{esc(c)}<b>{len(v)}</b></a>' for c, v in CATEGORIES.items())
    replace_block("index.html", "categories", f'<div class="cats" id="cats">{cats}</div>')
    replace_block("index.html", "count", f"{N} components across {len(CATEGORIES)} categories")
    recipes = "".join(f'<a class="feature" href="docs/recipes/{r["slug"]}/"><h3>{esc(r["h1"])}</h3><p>Talks to {esc(r["talks"])}.</p></a>' for r in RECIPES)
    replace_block("index.html", "recipes", recipes)
    # gallery static content
    cats_g = "".join(f'<li><a href="../components/{CAT_SLUG[c]}/">{esc(c)}</a> <span>{len(v)}</span></li>' for c, v in CATEGORIES.items())
    replace_block("gallery/index.html", "categories", f"<ul>{cats_g}</ul>")
    replace_block("gallery/index.html", "count", str(N))
    for sp in STATIC_PAGES:
        rel = sp["rel"]
        root = "../" * rel.count("/")
        ld = [site_jsonld()] if rel == "index.html" else [
            {"@context": "https://schema.org", "@type": "WebPage", "name": sp["title"], "description": sp["desc"],
             "url": BASE_URL + rel_url(rel), "isPartOf": {"@id": BASE_URL + "#website"}},
            crumbs_ld([("Home", ""), (sp["title"].split(":")[0], rel_url(rel))])]
        replace_block(rel, "head", head_tags(rel, root, sp["title"], sp["desc"], jsonld=ld, minimal=True))
        if rel != "gallery/index.html":  # the gallery is a full-screen canvas
            replace_block(rel, "footer", footer_html(root))


def main():
    build_recipes()
    build_components()
    build_categories()
    build_components_index()
    build_landings()
    build_404()
    build_manifest()
    build_llms()
    build_blocks()
    total = build_sitemap()
    imgs = sum(1 for d in demos if has_img(d["id"]))
    print(f"gen-site: {len(PAGES)} generated pages ({N} components, {len(CATEGORIES)} categories, {len(RECIPES)} recipes, "
          f"{len(LANDINGS)} landing pages), {total} URLs in sitemap.xml, {imgs} component screenshots")


if __name__ == "__main__":
    main()
