// Render the social images and the PNG favicon set with headless Chromium (Playwright).
//   node scripts/render-og.mjs
// Writes website/og.png (1200x630), website/social-preview.png (1280x640, for the GitHub repo
// settings), website/favicon-32.png, apple-touch-icon.png (180), icon-192.png and icon-512.png.
// These files are committed; CI does not run this.
import { createRequire } from "module";
import path from "path";
import { fileURLToPath } from "url";
import fs from "fs";
const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const web = path.join(root, "website");
const browser = await chromium.launch();

async function shot(url, w, h, out, scale = 1) {
  const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: scale });
  await page.goto(url, { waitUntil: "load" });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(200);
  await page.screenshot({ path: out });
  await page.close();
  console.log("wrote", path.relative(root, out));
}

const og = "file://" + path.join(root, "scripts", "og", "og.html");
await shot(og, 1200, 630, path.join(web, "og.png"));
await shot(og + "?w=1280&h=640", 1280, 640, path.join(web, "social-preview.png"));

// Favicons from favicon.svg: transparent for small sizes, an opaque tile for touch/app icons.
const svg = fs.readFileSync(path.join(web, "favicon.svg"), "utf8");
const icon = (size, tile) => `data:text/html,${encodeURIComponent(`<!doctype html><html><body style="margin:0;background:transparent">
  <div style="width:${size}px;height:${size}px;display:grid;place-items:center;${tile ? "background:#111113;" : ""}">
  <div style="width:${tile ? Math.round(size * 0.56) : size}px;height:${tile ? Math.round(size * 0.56) : size}px">${svg.replace("<svg ", '<svg width="100%" height="100%" ')}</div></div></body></html>`)}`;
for (const [size, name, tile] of [[32, "favicon-32.png", false], [180, "apple-touch-icon.png", true], [192, "icon-192.png", true], [512, "icon-512.png", true]]) {
  const page = await browser.newPage({ viewport: { width: 600, height: 600 } });
  await page.goto(icon(size, tile));
  for (let tries = 0; ; tries++) {  // capture can fail transiently on a busy machine
    try { await page.screenshot({ path: path.join(web, name), omitBackground: !tile, clip: { x: 0, y: 0, width: size, height: size } }); break; }
    catch (e) { if (tries >= 3) throw e; await page.waitForTimeout(500); }
  }
  await page.close();
  console.log("wrote", "website/" + name);
}
await browser.close();
