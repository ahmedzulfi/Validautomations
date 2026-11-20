# Research: Chronicle (chroniclehq.com)

How Slidesmith's feature set was chosen. Nothing here copies Chronicle's code, copy or visual design; Slidesmith is an independent implementation of the same *kind* of product.

## Method

1. **Read the public site**: home, pricing, AI presentation maker, integrations and API pages.
2. **Clean-room capture with [Craft Anything](https://github.com/storytold/craft-anything)** (v0.6.1, built from source). Its `web_site_open` → `web_site_capture` tools were run in **clean-room mode**, which records only what a visitor sees (screenshots at 1440 px and 390 px, visible text, measured layout, colors, fonts) and refuses to read page source, scripts or network traffic.
   - `https://chroniclehq.com/`: the marketing page (6154 px tall at desktop). Sections: hero, "Start from anywhere / Generate impressive slides / Edit with AI / Share in any format", "Built for serious business", templates, testimonials.
   - `https://app.chroniclehq.com/`: redirects a signed-out visitor to `login.chroniclehq.com` ("Log in to your Chronicle account"). Only the sign-in screen is visible without an account, so the editor itself was not observed.

Reproduce:

```sh
git clone https://github.com/storytold/craft-anything && cd craft-anything
cargo build --release -p craft-anything-cli
# web tools need one browser session, so drive them through the MCP server:
#   craft-anything-cli --clean-room --allow-run mcp
#   tools/call web_site_open {"url": "https://chroniclehq.com/"}
#   tools/call web_site_capture {"output_directory": "/abs/path/capture", "widths": [1440, 390]}
```

## What Chronicle offers (public pages)

- **Core loop:** start from notes, outlines, docs or old decks (attach files to a prompt) → AI structures the story into slides (hook → problem → solution → proof) → edit by hand or by instructing the AI → share or present.
- **Editing:** freeform canvas, every slide editable, "edit with AI" (an editing agent on higher plans).
- **Content:** charts, metrics, tables, embeds (YouTube, Loom, Notion, Google Sheets, forms, calendars).
- **Brand:** brand kit with logo, fonts and colors; preset and custom themes.
- **Share:** publish to web, PDF, PowerPoint (higher plan), social images; link view tracking.
- **Team:** real-time collaboration, comments, workspaces, saved templates.
- **Developer:** API and MCP server to create decks from other AI tools.
- **Pricing:** free tier without AI; paid tiers from about $12/user/month with a token allowance for AI actions.

## How Slidesmith maps to it

| Chronicle | Slidesmith |
|---|---|
| Start from anywhere (prompt + attachments) | Prompt box with PDF/Markdown/text attachments, or a plain Markdown outline (no AI) |
| Generate impressive slides | Claude with a structured deck schema and 11 purpose-built layouts |
| Edit with AI | Per-slide and whole-deck AI revision, each one an undo step |
| Brand kit | Themes + color, font, logo and footer overrides |
| Share in any format | Native PPTX, PDF, self-contained HTML, Markdown, JSON (all free) |
| Paid AI tokens | Bring your own Anthropic key; pay only API usage |
| Collaboration, analytics, MCP API | Not yet: needs a backend (see README "Not yet") |
