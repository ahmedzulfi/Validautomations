/**
 * "Publish" export: one self-contained HTML file with every slide pre-rendered,
 * keyboard/click navigation and no build step. Host it anywhere or email it.
 */
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { SlideView } from "../components/SlideView";
import slideCss from "../slide.css?raw";
import { resolveTheme } from "../themes";
import type { Deck } from "../types";

const esc = (s: string) => s.replace(/[&<>"']/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[ch]!);

const FONTS =
  "https://fonts.googleapis.com/css2?family=DM+Serif+Display&family=Fraunces:opsz,wght@9..144,400;9..144,600;9..144,700&family=IBM+Plex+Mono:wght@400;600&family=Inter:wght@400;500;600;700&family=Manrope:wght@400;600;800&family=Space+Grotesk:wght@400;500;700&display=swap";

export function deckToHtml(deck: Deck): string {
  const theme = resolveTheme(deck);
  const slides = deck.slides
    .map(
      (s, i) =>
        `<section class="frame" data-i="${i}" aria-label="Slide ${i + 1}">${renderToStaticMarkup(
          createElement(SlideView, { slide: s, theme, brand: deck.brand, index: i, total: deck.slides.length }),
        )}</section>`,
    )
    .join("\n");
  const notes = JSON.stringify(deck.slides.map((s) => s.notes)).replace(/</g, "\\u003c");

  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(deck.title)}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="${FONTS}" rel="stylesheet">
<style>
html,body{margin:0;height:100%;background:${theme.dark ? "#000" : "#111"};overflow:hidden}
.frame{position:fixed;inset:0;display:none;align-items:center;justify-content:center}
.frame.on{display:flex}
.frame .slide{transform-origin:center center;flex:none}
.frame.on .slide{animation:${deck.transition === "none" ? "none" : deck.transition === "slide" ? "sl .35s ease" : "fd .35s ease"}}
@keyframes fd{from{opacity:0}to{opacity:1}}
@keyframes sl{from{opacity:0;translate:40px 0}to{opacity:1;translate:0 0}}
#bar{position:fixed;left:0;bottom:0;height:4px;background:${theme.primary};transition:width .25s}
#notes{position:fixed;left:0;right:0;bottom:0;max-height:30vh;overflow:auto;background:rgba(0,0,0,.85);color:#fff;font:16px/1.5 system-ui,sans-serif;padding:16px 24px;display:none}
@media print{html,body{height:auto;overflow:visible;background:none}.frame{position:static;display:block;page-break-after:always}.frame .slide{transform:none!important;animation:none!important}#bar,#notes{display:none!important}@page{size:1280px 720px;margin:0}}
${slideCss}
</style>
</head>
<body>
${slides}
<div id="bar"></div><div id="notes"></div>
<script>
(function(){
var f=[].slice.call(document.querySelectorAll('.frame')),n=f.length,i=0,notes=${notes},nb=document.getElementById('notes'),showN=false;
var h=parseInt((location.hash||'').slice(1),10);if(h>0&&h<=n)i=h-1;
function fit(){var s=Math.min(innerWidth/1280,innerHeight/720);f.forEach(function(x){x.firstChild.style.transform='scale('+s+')'})}
function show(){f.forEach(function(x,j){x.classList.toggle('on',j===i)});document.getElementById('bar').style.width=((i+1)/n*100)+'%';nb.textContent=notes[i]||'No notes.';history.replaceState(null,'','#'+(i+1))}
function go(d){i=Math.max(0,Math.min(n-1,i+d));show()}
addEventListener('keydown',function(e){if(['ArrowRight','ArrowDown','PageDown',' ','Enter'].indexOf(e.key)>-1){e.preventDefault();go(1)}else if(['ArrowLeft','ArrowUp','PageUp','Backspace'].indexOf(e.key)>-1){e.preventDefault();go(-1)}else if(e.key==='Home'){i=0;show()}else if(e.key==='End'){i=n-1;show()}else if(e.key==='n'||e.key==='N'){showN=!showN;nb.style.display=showN?'block':'none'}else if(e.key==='f'||e.key==='F'){document.documentElement.requestFullscreen&&document.documentElement.requestFullscreen()}});
addEventListener('click',function(e){go(e.clientX>innerWidth/3?1:-1)});
var tx=null;addEventListener('touchstart',function(e){tx=e.touches[0].clientX},{passive:true});addEventListener('touchend',function(e){if(tx===null)return;var d=e.changedTouches[0].clientX-tx;if(Math.abs(d)>40)go(d<0?1:-1);tx=null});
addEventListener('resize',fit);fit();show();
})();
</script>
</body>
</html>
`;
}
