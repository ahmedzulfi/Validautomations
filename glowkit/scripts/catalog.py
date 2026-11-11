#!/usr/bin/env python3
"""Generate CATALOG.md and website/catalog.json.

It compares the component lists of popular web UI libraries (below) with the `covers` field of
every glowkit demo (from `glowkit-gallery --dump-json`), so the parity table is always computed
from the real code, never hand-edited.

usage: cargo run -p glowkit-gallery -- --dump-json > /tmp/demos.json && python3 scripts/catalog.py /tmp/demos.json
"""
import json, re, sys, os
from collections import OrderedDict

# Component lists as published by each library at the time of writing. They grow constantly:
# PRs that update these lists are very welcome. Pro / paid-only components are excluded.
SOURCES = OrderedDict([
    ("React Bits", {"url": "https://reactbits.dev", "items": [
        # text
        "Split Text", "Blur Text", "Circular Text", "Text Type", "Shuffle", "Shiny Text", "Text Pressure",
        "Curved Loop", "Fuzzy Text", "Gradient Text", "Falling Text", "Text Cursor", "Decrypted Text",
        "True Focus", "Scroll Float", "Scroll Reveal", "ASCII Text", "Scrambled Text", "Rotating Text",
        "Glitch Text", "Scroll Velocity", "Variable Proximity", "Count Up",
        # animations
        "Animated Content", "Fade Content", "Electric Border", "Pixel Transition", "Glare Hover", "Logo Loop",
        "Target Cursor", "Magnet Lines", "Click Spark", "Magnet", "Sticker Peel", "Pixel Trail", "Cubes",
        "Metallic Paint", "Noise", "Shape Blur", "Crosshair", "Image Trail", "Ribbons", "Splash Cursor",
        "Meta Balls", "Blob Cursor", "Star Border", "Laser Flow", "Gradual Blur",
        # components
        "Animated List", "Scroll Stack", "Bubble Menu", "Magic Bento", "Circular Gallery", "Card Nav", "Stack",
        "Fluid Glass", "Pill Nav", "Tilted Card", "Masonry", "Glass Surface", "Dome Gallery", "Chroma Grid",
        "Folder", "Staggered Menu", "Model Viewer", "Lanyard", "Profile Card", "Dock", "Gooey Nav",
        "Pixel Card", "Carousel", "Spotlight Card", "Flying Posters", "Card Swap", "Glass Icons",
        "Decay Card", "Flowing Menu", "Elastic Slider", "Counter", "Infinite Menu", "Stepper", "Bounce Cards",
        # backgrounds
        "Liquid Ether", "Prism", "Dark Veil", "Silk", "Faulty Terminal", "Light Rays", "Pixel Blast",
        "Color Bends", "Aurora", "Plasma", "Particles", "Gradient Blinds", "Grid Scan", "Beams", "Lightning",
        "Prismatic Burst", "Galaxy", "Dither", "Ripple Grid", "Dot Grid", "Threads", "Hyperspeed",
        "Iridescence", "Waves", "Grid Distortion", "Ballpit", "Orb", "Letter Glitch", "Grid Motion",
        "Squares", "Liquid Chrome", "Balatro",
    ]}),
    ("Magic UI", {"url": "https://magicui.design", "items": [
        "Marquee", "Terminal", "Hero Video Dialog", "Bento Grid", "Animated List", "Dock", "Globe",
        "Tweet Card", "Orbiting Circles", "Avatar Circles", "Icon Cloud", "Animated Circular Progress Bar",
        "File Tree", "Code Comparison", "Script Copy Button", "Scroll Progress", "Lens", "Pointer",
        "Smooth Cursor", "Progressive Blur", "Dotted Map", "Highlighter", "Magic Card", "Neon Gradient Card",
        "Border Beam", "Shine Border", "Meteors", "Confetti", "Particles", "Cool Mode", "Scratch To Reveal",
        "Blur Fade", "Animated Beam", "Number Ticker", "Animated Shiny Text", "Animated Gradient Text",
        "Text Animate", "Line Shadow Text", "Aurora Text", "Video Text", "Text Reveal", "Hyper Text",
        "Word Rotate", "Typing Animation", "Scroll Based Velocity", "Flip Text", "Box Reveal",
        "Sparkles Text", "Morphing Text", "Spinning Text", "Comic Text", "Animated Subscribe Button",
        "Rainbow Button", "Shimmer Button", "Ripple Button", "Shiny Button", "Pulsating Button",
        "Interactive Hover Button", "Warp Background", "Flickering Grid", "Animated Grid Pattern",
        "Retro Grid", "Ripple", "Dot Pattern", "Grid Pattern", "Interactive Grid Pattern", "Light Rays",
        "Animated Theme Toggler", "Pixel Image",
    ]}),
    ("Aceternity UI", {"url": "https://ui.aceternity.com", "items": [
        "3D Card", "3D Marquee", "3D Pin", "Animated Modal", "Animated Testimonials", "Animated Tooltip",
        "Apple Cards Carousel", "Aurora Background", "Background Beams", "Background Beams With Collision",
        "Background Boxes", "Background Gradient", "Background Gradient Animation", "Background Lines",
        "Bento Grid", "Canvas Reveal Effect", "Card Hover Effect", "Card Spotlight", "Card Stack", "Carousel",
        "Colourful Text", "Compare", "Container Scroll Animation", "Cover", "Codeblock",
        "Direction Aware Hover", "Draggable Card", "Evervault Card", "Expandable Card", "File Upload",
        "Flip Words", "Floating Dock", "Floating Navbar", "Focus Cards", "Following Pointer", "Glare Card",
        "Glowing Effect", "Glowing Stars", "Google Gemini Effect", "Grid and Dot Backgrounds",
        "Hero Highlight", "Hero Parallax", "Hover Border Gradient", "Images Slider",
        "Infinite Moving Cards", "Lamp Effect", "Layout Grid", "Lens", "Link Preview", "Macbook Scroll",
        "Meteors", "Moving Border", "Multi Step Loader", "Navbar Menu", "Parallax Scroll",
        "Placeholders And Vanish Input", "Pointer Highlight", "Resizable Navbar", "Shooting Stars",
        "Sidebar", "Signup Form", "Sparkles", "Spotlight", "Sticky Banner", "Sticky Scroll Reveal",
        "SVG Mask Effect", "Tabs", "Text Generate Effect", "Text Hover Effect", "Text Reveal Card",
        "Timeline", "Tracing Beam", "Typewriter Effect", "Vortex", "Wavy Background", "Wobble Card",
        "World Map", "Stateful Button", "Gradient text",
    ]}),
    ("Motion Primitives", {"url": "https://motion-primitives.com", "items": [
        "Accordion", "Animated Background", "Animated Group", "Animated Number", "Border Trail", "Carousel",
        "Cursor", "Dialog", "Disclosure", "Dock", "Glow Effect", "Image Comparison", "In View",
        "Infinite Slider", "Magnetic", "Morphing Dialog", "Morphing Popover", "Progressive Blur",
        "Scroll Progress", "Sliding Number", "Spinning Text", "Spotlight", "Text Effect", "Text Loop",
        "Text Morph", "Text Roll", "Text Scramble", "Text Shimmer", "Text Shimmer Wave", "Tilt",
        "Toolbar Dynamic", "Toolbar Expandable", "Transition Panel",
    ]}),
    ("shadcn/ui", {"url": "https://ui.shadcn.com", "items": [
        "Accordion", "Alert", "Alert Dialog", "Aspect Ratio", "Avatar", "Badge", "Breadcrumb", "Button",
        "Button Group", "Calendar", "Card", "Carousel", "Chart", "Checkbox", "Collapsible", "Combobox",
        "Command", "Context Menu", "Data Table", "Date Picker", "Dialog", "Drawer", "Dropdown Menu", "Empty",
        "Field", "Form", "Hover Card", "Input", "Input Group", "Input OTP", "Item", "Kbd", "Label", "Menubar",
        "Navigation Menu", "Pagination", "Popover", "Progress", "Radio Group", "Resizable", "Scroll Area",
        "Select", "Separator", "Sheet", "Sidebar", "Skeleton", "Slider", "Sonner", "Spinner", "Switch",
        "Table", "Tabs", "Textarea", "Toast", "Toggle", "Toggle Group", "Tooltip", "Typography",
    ]}),
    ("Vercel AI Elements", {"url": "https://ai-sdk.dev/elements", "items": [
        "Actions", "Artifact", "Branch", "Canvas", "Chain of Thought", "Checkpoint", "Code Block",
        "Confirmation", "Connection", "Context", "Controls", "Conversation", "Edge", "Image",
        "Inline Citation", "Loader", "Message", "Model Selector", "Node", "Open In Chat", "Panel", "Plan",
        "Prompt Input", "Queue", "Reasoning", "Response", "Shimmer", "Sources", "Suggestion", "Task", "Tool",
        "Toolbar", "Web Preview",
    ]}),
    ("prompt-kit", {"url": "https://www.prompt-kit.com", "items": [
        "Prompt Input", "Message", "Markdown", "Code Block", "Chat Container", "Scroll Button", "Loader",
        "Prompt Suggestion", "Response Stream", "Reasoning", "File Upload", "JSX Preview", "Tool", "Source",
        "Steps", "Text Shimmer", "Thinking Bar", "Feedback Bar", "Image",
    ]}),
    ("ElevenLabs UI", {"url": "https://ui.elevenlabs.io", "items": [
        "Orb", "Waveform", "Live Waveform", "Bar Visualizer", "Voice Button", "Voice Picker", "Conversation",
        "Message", "Response", "Shimmering Text", "Audio Player", "Matrix", "Mic Selector", "Speech Input",
        "Scrub Bar", "Transcript Viewer",
    ]}),
    ("Evil Charts", {"url": "https://evilcharts.com", "items": [
        "Area Chart", "Bar Chart", "Line Chart", "Pie Chart", "Radar Chart", "Radial Chart",
    ]}),
    ("MVP Blocks", {"url": "https://blocks.mvp-subha.me", "note": "section types (their catalog is organized by section)", "items": [
        "Hero", "Features", "Bento Grid", "Pricing", "Testimonials", "CTA", "Footer", "Header", "About",
        "Contact", "FAQ", "Team", "Waitlist", "Login", "Signup", "Not Found", "Dashboard", "Stats",
        "Logo Cloud", "Newsletter", "Blog", "Changelog", "Integrations", "Gallery", "Timeline",
    ]}),
    ("React Flow", {"url": "https://reactflow.dev", "items": [
        "Background", "Controls", "MiniMap", "Node Toolbar", "Edge Label", "Custom Nodes", "Custom Edges",
        "Animated Edges", "Selection", "Snap to Grid", "Auto Layout", "Undo Redo",
    ]}),
    ("Tremor", {"url": "https://tremor.so", "items": [
        "Area Chart", "Bar Chart", "Line Chart", "Donut Chart", "Bar List", "Spark Chart", "Tracker",
        "Category Bar", "Progress Bar", "Progress Circle", "Combo Chart", "Scatter Chart",
    ]}),
])

def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())

ALIASES = {  # library name variants used in `covers`
    "aceternity": "Aceternity UI", "aceternityui": "Aceternity UI", "magicui": "Magic UI",
    "reactbits": "React Bits", "motionprimitives": "Motion Primitives", "shadcn": "shadcn/ui",
    "shadcnui": "shadcn/ui", "aielements": "Vercel AI Elements", "vercelaielements": "Vercel AI Elements",
    "promptkit": "prompt-kit", "elevenlabsui": "ElevenLabs UI", "elevenlabs": "ElevenLabs UI",
    "evilcharts": "Evil Charts", "tremor": "Tremor", "mvpblocks": "MVP Blocks", "reactflow": "React Flow",
    "xyflow": "React Flow",
}

def main():
    demos = json.load(open(sys.argv[1]))
    covered = {}  # (lib, normalized item) -> [demo ids]
    other = {}    # libraries not in SOURCES -> set of items
    for d in demos:
        for c in d["covers"]:
            if ":" not in c:
                continue
            lib, item = [x.strip() for x in c.split(":", 1)]
            item = re.sub(r"\s*\(.*?\)\s*", " ", item).strip()
            canon = ALIASES.get(norm(lib), lib)
            for part in re.split(r"\s*/\s*|\s*&\s*|,\s*", item):
                if part:
                    covered.setdefault((canon, norm(part)), []).append(d["id"])
            if canon not in SOURCES:
                other.setdefault(canon, set()).add(item)

    out = ["# glowkit catalog", "",
           "Which components from popular web UI libraries have a glowkit equivalent.",
           "**Generated from the code** by `scripts/catalog.py` (every demo declares what it `covers`).", "",
           "glowkit components are original Rust implementations of these UI patterns, not ports of their code.",
           "Some items are hard or impossible in an immediate-mode GPU painter without images/3D/DOM",
           "(e.g. video text, 3D model viewers, real backdrop blur); those are marked as open.", ""]
    summary = []
    data = {"libraries": [], "demos": demos}
    total_have = total_all = 0
    sections = []
    for lib, info in SOURCES.items():
        rows = []
        have = 0
        for item in info["items"]:
            ids = covered.get((lib, norm(item)))
            if ids:
                have += 1
            rows.append((item, ids))
        total_have += have
        total_all += len(info["items"])
        pct = round(100 * have / max(1, len(info["items"])))
        summary.append(f"| [{lib}]({info['url']}) | {have} / {len(info['items'])} | {pct}% |")
        data["libraries"].append({"name": lib, "url": info["url"], "have": have, "total": len(info["items"]),
                                   "items": [{"name": i, "glowkit": ids or []} for i, ids in rows]})
        note = f" ({info['note']})" if info.get("note") else ""
        sec = [f"## {lib}", "", f"{have} of {len(info['items'])} covered{note}.", "", "| Component | glowkit |", "|---|---|"]
        for item, ids in rows:
            cell = ", ".join(f"`{i}`" for i in sorted(set(ids))) if ids else "open"
            sec.append(f"| {item} | {'✅ ' + cell if ids else '⬜ ' + cell} |")
        sections.append("\n".join(sec))
    out += ["## Summary", "", "| Library | Covered | % |", "|---|---|---|"] + summary
    out += [f"| **Total** | **{total_have} / {total_all}** | **{round(100 * total_have / max(1, total_all))}%** |", ""]
    out += [f"glowkit itself ships **{len(demos)}** gallery entries.", ""]
    out += sections
    if other:
        out += ["", "## Also covers", ""]
        for lib in sorted(other):
            out.append(f"* **{lib}**: " + ", ".join(sorted(other[lib])))
    root = os.path.join(os.path.dirname(__file__), "..")
    open(os.path.join(root, "CATALOG.md"), "w").write("\n".join(out) + "\n")
    data["total_have"], data["total_all"] = total_have, total_all
    json.dump(data, open(os.path.join(root, "website", "catalog.json"), "w"), indent=1)
    print(f"{len(demos)} demos; parity {total_have}/{total_all}")

if __name__ == "__main__":
    main()
