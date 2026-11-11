// Screenshot gallery routes headlessly.
// usage: [PKG=pkg-text] [WAIT=ms] node scripts/shot.mjs <outdir> <route> [route...]   (route like "home", "shiny-text", "shot/shiny-text")
// Needs a static server on :8787 serving ./website (scripts/serve.sh).
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const [outdir, ...routes] = process.argv.slice(2);
const fs = await import("fs");
fs.mkdirSync(outdir, { recursive: true });
const browser = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1360, height: 860 }, deviceScaleFactor: 1 });
const errors = [];
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
page.on("pageerror", (e) => errors.push(String(e)));
for (const r of routes) {
  const hash = r === "home" ? "" : "#" + r;
  await page.goto("about:blank");
  const q = process.env.PKG ? `?pkg=${process.env.PKG}` : "";
  await page.goto(`http://localhost:8787/gallery/index.html${q}${hash}`);
  await page.waitForFunction(() => !document.getElementById("loading"), null, { timeout: 60000 }).catch(() => errors.push("timeout loading " + r));
  await page.waitForTimeout(Number(process.env.WAIT || 1800));
  const file = `${outdir}/${r.replace(/[\/@]/g, "_")}.png`;
  await page.screenshot({ path: file });
  console.log("saved", file);
}
if (errors.length) console.log("ERRORS:\n" + [...new Set(errors)].slice(0, 20).join("\n"));
await browser.close();
