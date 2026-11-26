// Screenshots of the Quantum Explainer interface for the report and deck.
// Serves app/crates/ui/dist on localhost, drives headless Edge over the DevTools protocol (no installs),
// and saves PNGs to ../figures. The UI runs in preview mode (no native host), so Gemini is not called.
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import os from "node:os";

const DIST = path.resolve("../../../app/crates/ui/dist");
const OUT = path.resolve("../figures");
const EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";
const W = 1440, H = 900, DPR = 1.5;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".woff2": "font/woff2" };

const server = http.createServer((req, res) => {
  const p = path.join(DIST, decodeURIComponent(req.url.split("?")[0]).replace(/^\/$/, "/index.html"));
  if (!p.startsWith(DIST) || !fs.existsSync(p)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { "Content-Type": MIME[path.extname(p)] || "application/octet-stream" });
  fs.createReadStream(p).pipe(res);
}).listen(0);
const port = server.address().port;

const profile = fs.mkdtempSync(path.join(os.tmpdir(), "qe-edge-"));
const edge = spawn(EDGE, ["--headless=new", "--remote-debugging-port=9333", `--user-data-dir=${profile}`, "--hide-scrollbars", "about:blank"], { stdio: "ignore" });
let targets;
for (let i = 0; i < 40; i++) { try { targets = await (await fetch("http://127.0.0.1:9333/json")).json(); if (targets.length) break; } catch {} await sleep(250); }
const page = targets.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let id = 0; const pending = new Map();
ws.addEventListener("message", (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } });
const cdp = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const js = async (expr) => (await cdp("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true })).result?.result?.value;
async function shot(name) {
  const r = await cdp("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(path.join(OUT, name + ".png"), Buffer.from(r.result.data, "base64"));
  console.log("saved", name);
}
const nav = (label) => js(`(()=>{const b=[...document.querySelectorAll('.nav-link')].find(b=>b.textContent.trim()===${JSON.stringify(label)}); if(b){b.click(); return true} return false})()`);
const scrollStage = (y) => js(`(()=>{const s=document.querySelector('.stage'); s.scrollTo({top:${y}}); return s.scrollHeight})()`);

await cdp("Page.enable"); await cdp("Runtime.enable");
await cdp("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: DPR, mobile: false });
await cdp("Page.navigate", { url: `http://127.0.0.1:${port}/` });
for (let i = 0; i < 60 && !(await js("!!document.querySelector('.titlebar')")); i++) await sleep(250);
await js("document.fonts.ready.then(()=>true)");
await sleep(3500);
await shot("01-home-hero");
const total = await scrollStage(0);
console.log("home scrollHeight", total);
const stops = [0.12, 0.24, 0.38, 0.52, 0.66, 0.8, 0.94];
for (const [i, f] of stops.entries()) { await scrollStage(Math.round(total * f)); await sleep(3200); await shot(`02-home-scroll-${i + 1}`); }
await scrollStage(0);

await nav("Atlas"); await sleep(6000); await shot("03-atlas-map");
const opened = await js(`(()=>{const el=[...document.querySelectorAll('button, a, [role=button]')].find(e=>/entanglement/i.test(e.textContent)); if(el){el.click(); return el.textContent.trim().slice(0,40)} return null})()`);
console.log("topic opened:", opened);
await sleep(4000); await shot("04-topic-top");
const th = await scrollStage(0);
for (const [i, f] of [0.3, 0.55].entries()) { await scrollStage(Math.round(th * f)); await sleep(3500); await shot(`05-topic-scroll-${i + 1}`); }

await nav("Lab"); await sleep(2500); await shot("06-lab-start");
const nextBtn = `(()=>{const b=[...document.querySelectorAll('button')].find(b=>/next gate/i.test(b.textContent)); if(b){b.click(); return true} return false})()`;
await js(nextBtn); await sleep(900); await js(nextBtn); await sleep(1800); await shot("07-lab-oracle");

await nav("Tutor"); await sleep(2500); await shot("08-tutor-empty");
await js(`document.querySelector('.key-btn')?.click()`); await sleep(1500); await shot("09-settings");

ws.close(); edge.kill(); server.close();
setTimeout(() => process.exit(0), 500);
