// DECK.json -> designed .pptx (pptxgenjs, structured: theme + layouts + sections).
// Usage: node make_pptx.cjs content/DECK.json ../QuantumExplainer_Presentation.pptx
// Visual language follows the app: graphite ground, lime for "verified / +amplitude", coral for "−amplitude / risk",
// condensed uppercase headlines, qubits drawn in circle notation as the recurring motif.
const fs = require("fs");
const path = require("path");
const pptxgen = require("pptxgenjs");
const React = require("react");
const { renderToStaticMarkup } = require("react-dom/server");
const Fi = require("react-icons/fi");
const sharp = require("sharp");
const { applyTheme } = require("./apply_theme.js");

const [, , SRC, OUT] = process.argv;
const deck = JSON.parse(fs.readFileSync(SRC, "utf8"));
const ROOT = path.resolve(__dirname, "..");                    // docs/native-app

const HEX = { bg: "0E1015", surface: "191D25", surface2: "232833", ink: "F2F4F7", muted: "A3ABB9", lime: "D4FF3A", coral: "FF6B5E", paper: "EFEFEA", deep: "5E7A00" };
const THEME = {
  name: "Quantum Explainer",
  headFontFace: "Bahnschrift SemiBold Condensed",
  bodyFontFace: "Segoe UI",
  colors: { dk1: HEX.bg, lt1: HEX.ink, dk2: HEX.surface, lt2: HEX.muted, accent1: HEX.lime, accent2: HEX.coral, accent3: HEX.surface2, accent4: HEX.paper, accent5: "8FD3C1", accent6: HEX.deep, hlink: HEX.lime, folHlink: HEX.muted },
};
const W = 13.333, H = 7.5, M = 0.6;
const pres = new pptxgen();
pres.layout = "LAYOUT_WIDE";
pres.theme = { headFontFace: THEME.headFontFace, bodyFontFace: THEME.bodyFontFace };
pres.title = deck.title; pres.subject = deck.subtitle || ""; pres.author = "Team Chica"; pres.company = "Team Chica";
const C = pres.SchemeColor;
const BG = C.text1, SURF = C.text2, INK = C.background1, MUTED = C.background2, LIME = C.accent1, CORAL = C.accent2, SURF2 = C.accent3, PAPER = C.accent4;

// ── layouts (frames). Footer + slide number live on the layouts, never on slides. ──
const footer = (color) => ({ text: { text: "QUANTUM EXPLAINER  ·  TEAM CHICA  ·  PE-B65-13", options: { x: M, y: H - 0.42, w: 7, h: 0.25, fontSize: 9, color, charSpacing: 2, margin: 0 } } });
pres.defineSlideMaster({ title: "QE Title", background: { color: BG }, objects: [] });
pres.defineSlideMaster({
  title: "QE Content", background: { color: BG },
  objects: [
    footer(MUTED),
    { placeholder: { options: { name: "title", type: "title", x: M, y: 0.62, w: W - 2 * M, h: 0.8, fontSize: 34, bold: false, color: INK, align: "left", valign: "top", margin: 0, fontFace: THEME.headFontFace }, text: "" } },
  ],
  slideNumber: { x: W - M - 0.6, y: H - 0.42, w: 0.6, h: 0.25, fontSize: 9, color: MUTED, align: "right" },
});
pres.defineSlideMaster({ title: "QE Statement", background: { color: LIME }, objects: [footer(BG)], slideNumber: { x: W - M - 0.6, y: H - 0.42, w: 0.6, h: 0.25, fontSize: 9, color: BG, align: "right" } });
pres.defineSlideMaster({ title: "QE Bleed", background: { color: BG }, objects: [], slideNumber: { x: W - M - 0.6, y: H - 0.42, w: 0.6, h: 0.25, fontSize: 9, color: INK, align: "right" } });
pres.defineSlideMaster({ title: "QE Closing", background: { color: PAPER }, objects: [footer(BG)] });

// ── helpers ──
const up = (s) => (s || "").toUpperCase();
let oid = 0; const name = (p) => `${p}-${++oid}`;
const imgPath = (rel) => path.join(ROOT, rel);
async function icon(nameIn, color = "#0E1015") {
  const Comp = Fi[nameIn] || Fi.FiCircle;
  const svg = renderToStaticMarkup(React.createElement(Comp, { size: 256, color, strokeWidth: 2 }));
  const png = await sharp(Buffer.from(svg)).resize(256, 256).png().toBuffer();
  return "image/png;base64," + png.toString("base64");
}
async function imgData(rel, maxW = 2200) {
  const file = imgPath(rel);
  const meta = await sharp(file).metadata();
  const buf = await sharp(file).resize({ width: Math.min(maxW, meta.width) }).jpeg({ quality: 88 }).toBuffer();
  return { data: "image/jpeg;base64," + buf.toString("base64"), ratio: meta.height / meta.width };
}
// Kicker = small lime label above a title (the app's eyebrow style, used sparingly).
function kicker(s, text, y = 0.34, color = LIME) {
  if (!text) return;
  s.addText(up(text), { x: M, y, w: 8, h: 0.26, fontSize: 11, bold: true, color, charSpacing: 3, margin: 0, isTextBox: true, objectName: name("kicker") });
}
function body(s, text, opts) {
  if (!text) return;
  s.addText(text, { fontSize: 16, color: INK, margin: 0, valign: "top", isTextBox: true, paraSpaceAfter: 6, objectName: name("body"), ...opts });
}
function bullets(s, items, opts) {
  if (!items || !items.length) return;
  s.addText(items.map((t, i) => ({ text: t, options: { bullet: { code: "25CF" }, breakLine: i < items.length - 1, paraSpaceAfter: 10 } })),
    { fontSize: 16, color: INK, margin: 0, valign: "top", isTextBox: true, objectName: name("bullets"), ...opts });
}
// A screenshot in a soft "device" frame (surface2 rounded rect), keeps aspect, centred in the box.
async function framed(s, rel, x, y, w, h, caption) {
  const { data, ratio } = await imgData(rel);
  const pad = 0.08;
  let iw = w - 2 * pad, ih = iw * ratio;
  if (ih > h - 2 * pad - (caption ? 0.42 : 0)) { ih = h - 2 * pad - (caption ? 0.42 : 0); iw = ih / ratio; }
  const fx = x + (w - iw - 2 * pad) / 2, fy = y;
  s.addShape(pres.shapes.ROUNDED_RECTANGLE, { x: fx, y: fy, w: iw + 2 * pad, h: ih + 2 * pad, rectRadius: 0.12, fill: { color: SURF2 }, line: { color: SURF2 },
    shadow: { type: "outer", color: "000000", opacity: 0.45, blur: 18, offset: 6, angle: 90 }, objectName: name("frame") });
  s.addImage({ data, x: fx + pad, y: fy + pad, w: iw, h: ih, objectName: name("screenshot"), altText: caption || "Screenshot of Quantum Explainer" });
  if (caption) s.addText(caption, { x: fx, y: fy + ih + 2 * pad + 0.1, w: iw + 2 * pad, h: 0.3, fontSize: 11, color: MUTED, italic: true, margin: 0, isTextBox: true, objectName: name("caption") });
}
// The motif: a qubit in circle notation (radius = |amplitude|, line = phase), as in the app.
function qubit(s, cx, cy, r, amp, phaseDeg, color = LIME) {
  s.addShape(pres.shapes.OVAL, { x: cx - r, y: cy - r, w: 2 * r, h: 2 * r, fill: { color: BG, transparency: 100 }, line: { color: MUTED, width: 1.25, transparency: 55 }, objectName: name("qubit-ring") });
  if (amp > 0) {
    const ri = r * amp;
    s.addShape(pres.shapes.OVAL, { x: cx - ri, y: cy - ri, w: 2 * ri, h: 2 * ri, fill: { color }, line: { color, width: 0 }, objectName: name("qubit-fill") });
    const a = (phaseDeg - 90) * Math.PI / 180, ex = cx + r * Math.cos(a), ey = cy + r * Math.sin(a);
    s.addShape(pres.shapes.LINE, { x: Math.min(cx, ex), y: Math.min(cy, ey), w: Math.abs(ex - cx) || 0.001, h: Math.abs(ey - cy) || 0.001,
      flipH: ex < cx, flipV: ey < cy, line: { color: INK, width: 1.5 }, objectName: name("qubit-phase") });
  }
}
async function iconDisc(s, iconName, x, y, d = 0.62) {
  s.addShape(pres.shapes.OVAL, { x, y, w: d, h: d, fill: { color: LIME }, line: { color: LIME }, objectName: name("icon-disc") });
  s.addImage({ data: await icon(iconName), x: x + d * 0.22, y: y + d * 0.22, w: d * 0.56, h: d * 0.56, objectName: name("icon"), altText: iconName });
}

// ── slide renderers ──
const R = {
  async title(sl, s) {
    s.addText(up(deck.title), { x: M, y: 1.55, w: 7.2, h: 2.4, fontFace: THEME.headFontFace, fontSize: 66, color: INK, margin: 0, valign: "bottom", lineSpacingMultiple: 0.85, isTextBox: true, objectName: "title" });
    kicker(s, sl.kicker || "Native Windows app", 1.2);
    s.addText(sl.body || deck.subtitle || "", { x: M, y: 4.15, w: 6.6, h: 1.0, fontSize: 18, color: MUTED, margin: 0, valign: "top", isTextBox: true, objectName: "subtitle" });
    if (sl.caption) s.addText(sl.caption, { x: M, y: 6.35, w: 6.8, h: 0.5, fontSize: 11, color: MUTED, margin: 0, isTextBox: true, objectName: "meta" });
    // Grover after diffusion: all amplitude on the marked item |01⟩ (the app's own hero state).
    const g = [[0, 0], [1, 0], [0, 0], [0, 0]];
    const cx0 = 9.45, cy0 = 2.35, r = 0.95, gap = 2.3;
    [[0, 0], [1, 0], [0, 1], [1, 1]].forEach(([cx, cy], i) => qubit(s, cx0 + cx * gap, cy0 + cy * gap, r, i === 1 ? 1 : 0, 0));  // marked item |01⟩
    ["|00⟩", "|01⟩", "|10⟩", "|11⟩"].forEach((lab, i) => {
      const cx = cx0 + (i % 2) * gap, cy = cy0 + Math.floor(i / 2) * gap;
      s.addText(lab, { x: cx - 0.6, y: cy + r + 0.08, w: 1.2, h: 0.3, fontSize: 12, color: MUTED, align: "center", margin: 0, fontFace: "Cambria Math", isTextBox: true, objectName: name("ket") });
    });
  },
  async statement(sl, s) {
    kicker(s, sl.kicker, 0.6, BG);
    s.addText(up(sl.title), { x: M, y: 1.3, w: W - 2 * M, h: 3.6, fontFace: THEME.headFontFace, fontSize: 80, color: BG, margin: 0, valign: "bottom", lineSpacingMultiple: 0.85, isTextBox: true, objectName: "title" });
    if (sl.body) s.addText(sl.body, { x: M, y: 5.2, w: 7.6, h: 1.2, fontSize: 18, color: BG, margin: 0, valign: "top", isTextBox: true, objectName: "body" });
  },
  async quote(sl, s) { return R.statement({ ...sl, title: sl.quote || sl.title }, s); },
  async "image-right"(sl, s, left = false) {
    const colW = 4.3, imgW = W - 2 * M - colW - 0.5;
    const tx = left ? M + imgW + 0.5 : M, ix = left ? M : M + colW + 0.5;
    // Body paragraph and bullets share one text box, so the bullets always sit right under the body.
    const parts = [];
    if (sl.body) parts.push({ text: sl.body, options: { fontSize: 17, breakLine: true, paraSpaceAfter: 14 } });
    (sl.bullets || []).forEach((t, i, a) => parts.push({ text: t, options: { bullet: { code: "25CF" }, fontSize: 16, breakLine: i < a.length - 1, paraSpaceAfter: 10 } }));
    if (parts.length) s.addText(parts, { x: tx, y: 1.75, w: colW, h: 4.9, color: INK, margin: 0, valign: "top", isTextBox: true, objectName: name("body") });
    if (sl.image) await framed(s, sl.image, ix, 1.7, imgW, 5.1, sl.caption);
  },
  async "image-left"(sl, s) { return R["image-right"](sl, s, true); },
  async "image-full"(sl, s) {
    const { data } = await imgData(sl.image);
    s.addImage({ data, x: 0, y: 0, w: W, h: W * 900 / 1440, sizing: { type: "cover", w: W, h: H }, objectName: "bleed-image", altText: sl.caption || sl.title });
    s.addShape(pres.shapes.RECTANGLE, { x: 0, y: H - 2.0, w: W, h: 2.0, fill: { color: BG }, line: { color: BG }, objectName: "title-band" });
    kicker(s, sl.kicker, H - 1.78);
    s.addText(up(sl.title), { x: M, y: H - 1.5, w: 8.4, h: 0.9, fontFace: THEME.headFontFace, fontSize: 36, color: INK, margin: 0, valign: "top", isTextBox: true, objectName: "title" });
    if (sl.body || sl.caption) s.addText(sl.body || sl.caption, { x: 9.3, y: H - 1.5, w: W - 9.3 - M, h: 1.0, fontSize: 13, color: MUTED, margin: 0, valign: "top", isTextBox: true, objectName: "body" });
  },
  async pipeline(sl, s) {
    body(s, sl.body, { x: M, y: 1.65, w: 9.5, h: 0.9 });
    const steps = sl.steps || [], n = steps.length || 1;
    const y = 3.05, d = 1.0, span = W - 2 * M, cell = span / n;
    const icons = ["FiMessageSquare", "FiBookOpen", "FiCheckCircle", "FiEdit3", "FiRepeat"];
    for (let i = 0; i < n; i++) {
      const cx = M + cell * i + cell / 2;
      if (i < n - 1) s.addShape(pres.shapes.LINE, { x: cx + d / 2 + 0.1, y: y + d / 2, w: cell - d - 0.2, h: 0, line: { color: SURF2, width: 2, endArrowType: "triangle" }, objectName: name("connector") });
      await iconDisc(s, (sl.icons && sl.icons[i]) || icons[i % icons.length], cx - d / 2, y, d);
      s.addText(up(steps[i].title), { x: cx - cell / 2 + 0.1, y: y + d + 0.25, w: cell - 0.2, h: 0.45, fontFace: THEME.headFontFace, fontSize: 22, color: INK, align: "center", margin: 0, isTextBox: true, objectName: name("step-title") });
      s.addText(steps[i].text, { x: cx - cell / 2 + 0.15, y: y + d + 0.75, w: cell - 0.3, h: 1.2, fontSize: 13, color: MUTED, align: "center", margin: 0, valign: "top", isTextBox: true, objectName: name("step-text") });
    }
    if (sl.caption) s.addText(sl.caption, { x: M, y: 6.35, w: W - 2 * M, h: 0.4, fontSize: 12, color: LIME, margin: 0, isTextBox: true, objectName: "note" });
  },
  async stats(sl, s) {
    body(s, sl.body, { x: M, y: 1.65, w: 9.8, h: 0.9 });
    const st = sl.stats || [], n = st.length || 1, gap = 0.3, cw = (W - 2 * M - gap * (n - 1)) / n;
    st.forEach((x, i) => {
      const cx = M + i * (cw + gap);
      s.addShape(pres.shapes.ROUNDED_RECTANGLE, { x: cx, y: 2.75, w: cw, h: 2.9, rectRadius: 0.14, fill: { color: SURF }, line: { color: SURF }, objectName: name("stat-card") });
      s.addText(x.value, { x: cx + 0.3, y: 3.0, w: cw - 0.6, h: 1.5, fontFace: THEME.headFontFace, fontSize: x.value.length > 6 ? 44 : 64, color: LIME, margin: 0, valign: "bottom", isTextBox: true, objectName: name("stat-value") });
      s.addText(x.label, { x: cx + 0.3, y: 4.65, w: cw - 0.6, h: 0.8, fontSize: 14, color: INK, margin: 0, valign: "top", isTextBox: true, objectName: name("stat-label") });
    });
    if (sl.caption) s.addText(sl.caption, { x: M, y: 6.0, w: W - 2 * M, h: 0.4, fontSize: 12, color: MUTED, italic: true, margin: 0, isTextBox: true, objectName: "caption" });
  },
  async "two-column"(sl, s) {
    body(s, sl.body, { x: M, y: 1.65, w: 10, h: 0.8 });
    const cols = sl.columns || [], gap = 0.9, cw = (W - 2 * M - gap) / 2, top = sl.body ? 2.6 : 1.8;
    cols.slice(0, 2).forEach((c, i) => {
      const x = M + i * (cw + gap);
      s.addShape(pres.shapes.ROUNDED_RECTANGLE, { x, y: top, w: cw, h: 6.55 - top, rectRadius: 0.14, fill: { color: i === 1 ? SURF2 : SURF }, line: { color: i === 1 ? LIME : SURF, width: i === 1 ? 1.25 : 0 }, objectName: name("column") });
      s.addText(up(c.title), { x: x + 0.35, y: top + 0.3, w: cw - 0.7, h: 0.5, fontFace: THEME.headFontFace, fontSize: 24, color: i === 1 ? LIME : INK, margin: 0, isTextBox: true, objectName: name("column-title") });
      bullets(s, c.bullets, { x: x + 0.35, y: top + 0.95, w: cw - 0.7, h: 6.55 - top - 1.2, fontSize: 15 });
    });
    if (cols.length === 2) s.addText("→", { x: M + cw, y: top + 1.6, w: gap, h: 0.7, fontSize: 32, color: LIME, align: "center", margin: 0, isTextBox: true, objectName: "arrow" });
  },
  async architecture(sl, s) {
    body(s, sl.body, { x: M, y: 1.65, w: 4.0, h: 4.5, fontSize: 15 });
    const L = sl.layers || [], x = 5.0, w = W - M - x, top = 1.7, gap = 0.16, lh = Math.min(0.95, (5.0 - gap * (L.length - 1)) / (L.length || 1));
    L.forEach((l, i) => {
      const y = top + i * (lh + gap), hi = /qcore|simulator/i.test(l.name);
      s.addShape(pres.shapes.ROUNDED_RECTANGLE, { x, y, w, h: lh, rectRadius: 0.1, fill: { color: hi ? LIME : SURF }, line: { color: hi ? LIME : SURF2, width: 1 }, objectName: name("layer") });
      s.addText(up(l.name), { x: x + 0.3, y, w: 2.7, h: lh, fontFace: THEME.headFontFace, fontSize: 17, color: hi ? BG : INK, valign: "middle", margin: 0, isTextBox: true, objectName: name("layer-name") });
      s.addText(l.detail, { x: x + 3.05, y, w: w - 3.3, h: lh, fontSize: 12, color: hi ? BG : MUTED, valign: "middle", margin: 0, isTextBox: true, objectName: name("layer-detail") });
    });
  },
  async cards(sl, s) {
    body(s, sl.body, { x: M, y: 1.65, w: 10, h: 0.8 });
    const cs = sl.cards || [], n = cs.length, cols = n <= 4 ? 2 : 3, rows = Math.ceil(n / cols), gap = 0.3;
    const top = sl.body ? 2.55 : 1.8, cw = (W - 2 * M - gap * (cols - 1)) / cols, ch = (6.6 - top - gap * (rows - 1)) / rows;
    for (let i = 0; i < n; i++) {
      const c = cs[i], x = M + (i % cols) * (cw + gap), y = top + Math.floor(i / cols) * (ch + gap);
      s.addShape(pres.shapes.ROUNDED_RECTANGLE, { x, y, w: cw, h: ch, rectRadius: 0.14, fill: { color: SURF }, line: { color: SURF }, objectName: name("card") });
      await iconDisc(s, c.icon, x + 0.3, y + 0.3, 0.6);
      s.addText(up(c.title), { x: x + 1.1, y: y + 0.3, w: cw - 1.35, h: 0.6, fontFace: THEME.headFontFace, fontSize: 19, color: INK, valign: "middle", margin: 0, isTextBox: true, objectName: name("card-title") });
      s.addText(c.text, { x: x + 0.3, y: y + 1.05, w: cw - 0.6, h: ch - 1.2, fontSize: 13, color: MUTED, valign: "top", margin: 0, isTextBox: true, objectName: name("card-text") });
    }
  },
  async closing(sl, s) {
    kicker(s, sl.kicker, 0.6, C.accent6);
    s.addText(up(sl.title), { x: M, y: 1.1, w: W - 2 * M, h: 2.6, fontFace: THEME.headFontFace, fontSize: 84, color: BG, margin: 0, valign: "bottom", lineSpacingMultiple: 0.85, isTextBox: true, objectName: "title" });
    if (sl.body) s.addText(sl.body, { x: M, y: 4.0, w: 7.5, h: 1.0, fontSize: 18, color: BG, margin: 0, valign: "top", isTextBox: true, objectName: "body" });
    bullets(s, sl.bullets, { x: M, y: 5.0, w: 7.5, h: 1.5, color: BG, fontSize: 15 });
    if (sl.caption) s.addText(sl.caption, { x: 8.4, y: 5.0, w: W - 8.4 - M, h: 1.5, fontSize: 13, color: BG, margin: 0, valign: "top", isTextBox: true, objectName: "meta" });
  },
};
const MASTER = { title: "QE Title", statement: "QE Statement", quote: "QE Statement", "image-full": "QE Bleed", closing: "QE Closing" };
const OWN_TITLE = new Set(["title", "statement", "quote", "image-full", "closing"]);

(async () => {
  let section = null;
  for (const sl of deck.slides) {
    const sec = sl.section || (sl.layout === "title" ? "Opening" : sl.layout === "closing" ? "Close" : section || "Story");
    if (sec !== section) { pres.addSection({ title: sec }); section = sec; }
    const s = pres.addSlide({ masterName: MASTER[sl.layout] || "QE Content", sectionTitle: section });
    if (!OWN_TITLE.has(sl.layout)) {
      kicker(s, sl.kicker);
      s.addText(up(sl.title), { placeholder: "title" });
    }
    const fn = R[sl.layout] || R["image-right"];
    await fn(sl, s);
    if (sl.notes) s.addNotes(sl.notes);
  }
  await pres.writeFile({ fileName: OUT });
  await applyTheme(OUT, THEME);
  console.log("wrote", OUT, deck.slides.length, "slides");
})().catch((e) => { console.error(e); process.exit(1); });
