// Markdown subset -> styled Word document (docx-js).
// Usage: node make_docx.mjs <content.md> <out.docx> <config.json>
// Subset: # / ## / ### headings, paragraphs, **bold** *italic* `code`, "- " bullets, "1. " numbered lists,
// pipe tables, ![caption](figures/x.png), ```text code blocks, "> " callouts, <<<PAGEBREAK>>>.
import fs from "node:fs";
import path from "node:path";
import sharp from "sharp";
import {
  AlignmentType, BorderStyle, Document, Footer, Header, HeadingLevel, ImageRun, LevelFormat, Packer, PageBreak,
  PageNumber, Paragraph, ShadingType, Table, TableCell, TableOfContents, TableRow, TextRun, WidthType, VerticalAlign,
  TabStopType, TabStopPosition,
} from "docx";

const [, , SRC, OUT, CFG] = process.argv;
const cfg = JSON.parse(fs.readFileSync(CFG, "utf8"));
const md = fs.readFileSync(SRC, "utf8").replace(/\r\n/g, "\n");
const ROOT = path.resolve(path.dirname(SRC), "..", "..");          // k:\Prompt\docs\native-app

// ── design tokens (match the app: graphite ink, lime accent, coral for warnings) ──
const T = {
  ink: "14161C", muted: "5B6270", accent: "6E8F00", lime: "D4FF3A", coral: "C2453A",
  headFont: "Bahnschrift", bodyFont: "Calibri", monoFont: "Consolas",
  tableHead: "1B1F27", tableAlt: "F4F6F8", rule: "D9DDE3", calloutFill: "F2F8DF",
};
const PAGE_W = 11906, MARGIN = 1440, TEXT_W = PAGE_W - 2 * MARGIN; // A4, 1" margins → 9026 DXA
const PX_PER_DXA = 96 / 1440;
const TEXT_PX = Math.round(TEXT_W * PX_PER_DXA);                    // ≈ 602 px

// ── inline markup → runs ──
function runs(text, base = {}) {
  const out = [];
  const re = /(\*\*[^*]+\*\*|\*[^*\s][^*]*\*|`[^`]+`)/g;
  let last = 0, m;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(new TextRun({ text: text.slice(last, m.index), ...base }));
    const t = m[0];
    if (t.startsWith("**")) out.push(new TextRun({ text: t.slice(2, -2), bold: true, ...base }));
    else if (t.startsWith("`")) out.push(new TextRun({ text: t.slice(1, -1), font: T.monoFont, size: 19, color: "3A4150", ...base }));
    else out.push(new TextRun({ text: t.slice(1, -1), italics: true, ...base }));
    last = re.lastIndex;
  }
  if (last < text.length) out.push(new TextRun({ text: text.slice(last), ...base }));
  return out;
}
const plain = (s) => s.replace(/\*\*|`/g, "").replace(/(^|\s)\*(\S)/g, "$1$2").replace(/(\S)\*(\s|$)/g, "$1$2");

const UNNUMBERED = /^(abstract|references|appendix|glossary|declaration|acknowledg)/i;
let figNo = 0, tableNo = 0, listInstance = 0;

async function figure(caption, rel) {
  const file = path.join(ROOT, rel);
  if (!fs.existsSync(file)) { console.warn("missing figure", rel); return [new Paragraph({ children: [new TextRun({ text: `[missing figure ${rel}]`, color: T.coral })] })]; }
  const meta = await sharp(file).metadata();
  // Downscale large screenshots to keep the .docx small; 1600 px wide is plenty for print.
  const buf = await sharp(file).resize({ width: Math.min(1600, meta.width) }).png({ compressionLevel: 9 }).toBuffer();
  const w = TEXT_PX, h = Math.round((meta.height / meta.width) * w);
  figNo++;
  return [
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 160, after: 60 }, keepNext: true,
      children: [new ImageRun({ type: "png", data: buf, transformation: { width: w, height: h },
        altText: { title: `Figure ${figNo}`, description: plain(caption), name: `figure${figNo}` } })] }),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 220 },
      children: [new TextRun({ text: `Figure ${figNo}. `, bold: true, size: 19, color: T.muted }), ...runs(caption, { size: 19, color: T.muted, italics: true })] }),
  ];
}

function table(lines) {
  const rows = lines.filter((l, i) => i !== 1).map((l) => l.trim().replace(/^\||\|$/g, "").split("|").map((c) => c.trim()));
  const n = Math.max(...rows.map((r) => r.length));
  rows.forEach((r) => { while (r.length < n) r.push(""); });
  // Column widths ∝ longest cell (clamped), summing exactly to the text width.
  // Each column first gets room for its longest unbreakable word (IDs like FR-WIN-1 stay on one line),
  // capped at 40% of the width; the rest is shared in proportion to how much text the column holds.
  const words = (c) => rows.map((r) => plain(r[c]).split(/\s+/)).flat();
  const minW = [...Array(n)].map((_, c) => Math.min(Math.max(...words(c).map((w) => w.length)) * 105 + 240, TEXT_W * 0.4));
  const want = [...Array(n)].map((_, c) => Math.min(60, Math.max(4, ...rows.map((r) => plain(r[c]).length))));
  let base = minW.reduce((a, b) => a + b, 0);
  const scale = base > TEXT_W ? TEXT_W / base : 1;
  const mins = minW.map((w) => w * scale);
  const spare = TEXT_W - mins.reduce((a, b) => a + b, 0);
  const sumWant = want.reduce((a, b) => a + b, 0);
  const widths = mins.map((w, c) => Math.floor(w + (spare * want[c]) / sumWant));
  widths[n - 1] += TEXT_W - widths.reduce((a, b) => a + b, 0);
  const border = { style: BorderStyle.SINGLE, size: 4, color: T.rule };
  const borders = { top: border, bottom: border, left: border, right: border };
  tableNo++;
  return new Table({
    width: { size: TEXT_W, type: WidthType.DXA }, columnWidths: widths,
    rows: rows.map((r, ri) => new TableRow({
      tableHeader: ri === 0, cantSplit: true,
      children: r.map((cell, ci) => new TableCell({
        width: { size: widths[ci], type: WidthType.DXA }, borders, verticalAlign: VerticalAlign.CENTER,
        margins: { top: 60, bottom: 60, left: 100, right: 100 },
        shading: ri === 0 ? { type: ShadingType.CLEAR, fill: T.tableHead, color: "auto" } : ri % 2 === 0 ? { type: ShadingType.CLEAR, fill: T.tableAlt, color: "auto" } : undefined,
        children: [new Paragraph({ spacing: { before: 0, after: 0, line: 260 },
          children: runs(cell.replace(/([A-Z]{1,5}(?:-[A-Z0-9]{1,6})+)/g, (m) => m.replace(/-/g, "‑")), ri === 0 ? { bold: true, color: "FFFFFF", size: 19 } : { size: 19 }) })],
      })),
    })),
  });
}

async function body() {
  const out = [];
  const L = md.split("\n");
  for (let i = 0; i < L.length; i++) {
    const line = L[i];
    if (!line.trim()) continue;
    if (line.trim() === "<<<PAGEBREAK>>>") { out.push(new Paragraph({ children: [new PageBreak()] })); continue; }
    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    if (h) {
      const level = h[1].length, text = h[2].trim();
      const numbered = !UNNUMBERED.test(text);
      if (level === 1 && out.length) out.push(new Paragraph({ children: [new PageBreak()] }));
      out.push(new Paragraph({
        heading: [HeadingLevel.HEADING_1, HeadingLevel.HEADING_2, HeadingLevel.HEADING_3][level - 1],
        numbering: numbered ? { reference: "heads", level: level - 1 } : undefined,
        children: [new TextRun(level === 1 ? plain(text) : plain(text))],
      }));
      continue;
    }
    if (line.startsWith("```")) {
      const code = [];
      for (i++; i < L.length && !L[i].startsWith("```"); i++) code.push(L[i]);
      out.push(...code.map((c, k) => new Paragraph({
        shading: { type: ShadingType.CLEAR, fill: "F3F4F6", color: "auto" }, keepLines: true, keepNext: k < code.length - 1,
        spacing: { before: k === 0 ? 120 : 0, after: k === code.length - 1 ? 160 : 0, line: 240 },
        indent: { left: 120, right: 120 },
        children: [new TextRun({ text: c || " ", font: T.monoFont, size: 17, color: "1F2430" })],
      })));
      continue;
    }
    const fig = /^!\[(.*)\]\((.+)\)\s*$/.exec(line.trim());
    if (fig) { out.push(...(await figure(fig[1], fig[2]))); continue; }
    if (line.trim().startsWith("|")) {
      const tl = [];
      for (; i < L.length && L[i].trim().startsWith("|"); i++) tl.push(L[i]);
      i--;
      out.push(table(tl), new Paragraph({ spacing: { after: 120 }, children: [] }));
      continue;
    }
    if (/^- /.test(line)) {
      for (; i < L.length && /^- /.test(L[i]); i++)
        out.push(new Paragraph({ numbering: { reference: "bullets", level: 0 }, spacing: { after: 60 }, children: runs(L[i].slice(2)) }));
      i--; continue;
    }
    if (/^\d+\. /.test(line)) {
      const inst = ++listInstance;
      for (; i < L.length && /^\d+\. /.test(L[i]); i++)
        out.push(new Paragraph({ numbering: { reference: "numbers", level: 0, instance: inst }, spacing: { after: 60 }, children: runs(L[i].replace(/^\d+\. /, "")) }));
      i--; continue;
    }
    if (line.startsWith("> ")) {
      const q = [];
      for (; i < L.length && L[i].startsWith("> "); i++) q.push(L[i].slice(2));
      i--;
      out.push(new Paragraph({
        shading: { type: ShadingType.CLEAR, fill: T.calloutFill, color: "auto" },
        border: { left: { style: BorderStyle.SINGLE, size: 18, color: T.accent, space: 8 } },
        indent: { left: 200, right: 120 }, spacing: { before: 120, after: 200 },
        children: runs(q.join(" ")),
      }));
      continue;
    }
    // paragraph: join consecutive non-empty, non-special lines
    const p = [line];
    while (i + 1 < L.length && L[i + 1].trim() && !/^(#|- |\d+\. |\||```|> |!\[|<<<)/.test(L[i + 1])) p.push(L[++i]);
    out.push(new Paragraph({ spacing: { after: 140 }, alignment: AlignmentType.JUSTIFIED, children: runs(p.join(" ")) }));
  }
  return out;
}

async function cover() {
  const items = [];
  items.push(new Paragraph({ spacing: { before: 600, after: 0 }, children: [new TextRun({ text: cfg.kicker.toUpperCase(), font: T.headFont, size: 22, bold: true, color: T.accent, characterSpacing: 40 })] }));
  items.push(new Paragraph({ spacing: { before: 120, after: 80 }, children: [new TextRun({ text: cfg.title, font: T.headFont, size: 64, bold: true, color: T.ink })] }));
  items.push(new Paragraph({ spacing: { after: 300 }, children: [new TextRun({ text: cfg.subtitle, font: T.bodyFont, size: 28, color: T.muted })] }));
  if (cfg.coverImage) {
    const file = path.join(ROOT, cfg.coverImage);
    const meta = await sharp(file).metadata();
    const buf = await sharp(file).resize({ width: 1600 }).png({ compressionLevel: 9 }).toBuffer();
    const w = TEXT_PX, hgt = Math.round((meta.height / meta.width) * w);
    items.push(new Paragraph({ spacing: { after: 300 }, children: [new ImageRun({ type: "png", data: buf, transformation: { width: w, height: hgt },
      altText: { title: "Cover", description: cfg.coverAlt || "Screenshot of Quantum Explainer", name: "cover" } })] }));
  }
  const border = { style: BorderStyle.SINGLE, size: 4, color: T.rule };
  const none = { style: BorderStyle.NONE, size: 0, color: "FFFFFF" };
  const w1 = 2400, w2 = TEXT_W - w1;
  items.push(new Table({
    width: { size: TEXT_W, type: WidthType.DXA }, columnWidths: [w1, w2],
    rows: cfg.fields.map(([k, v]) => new TableRow({ children: [
      new TableCell({ width: { size: w1, type: WidthType.DXA }, borders: { top: none, left: none, right: none, bottom: border }, margins: { top: 50, bottom: 50, left: 0, right: 100 },
        children: [new Paragraph({ children: [new TextRun({ text: k, bold: true, size: 19, color: T.muted })] })] }),
      new TableCell({ width: { size: w2, type: WidthType.DXA }, borders: { top: none, left: none, right: none, bottom: border }, margins: { top: 50, bottom: 50, left: 100, right: 0 },
        children: v.split("\n").map((line) => new Paragraph({ children: [new TextRun({ text: line, size: 19 })] })) }),
    ] })),
  }));
  items.push(new Paragraph({ children: [new PageBreak()] }));
  items.push(new Paragraph({ spacing: { after: 200 }, children: [new TextRun({ text: "Contents", font: T.headFont, size: 36, bold: true, color: T.ink })] }));
  items.push(new TableOfContents("Contents", { hyperlink: true, headingStyleRange: "1-2" }));
  items.push(new Paragraph({ children: [new PageBreak()] }));
  return items;
}

const doc = new Document({
  creator: cfg.author, title: cfg.title, subject: cfg.subtitle, description: cfg.kicker,
  features: { updateFields: true },
  styles: {
    default: { document: { run: { font: T.bodyFont, size: 22, color: T.ink }, paragraph: { spacing: { line: 288 } } } },
    paragraphStyles: [
      { id: "Heading1", name: "Heading 1", basedOn: "Normal", next: "Normal", quickFormat: true,
        run: { font: T.headFont, size: 40, bold: true, color: T.ink }, paragraph: { spacing: { before: 0, after: 240 }, keepNext: true, outlineLevel: 0 } },
      { id: "Heading2", name: "Heading 2", basedOn: "Normal", next: "Normal", quickFormat: true,
        run: { font: T.headFont, size: 28, bold: true, color: T.ink }, paragraph: { spacing: { before: 320, after: 120 }, keepNext: true, outlineLevel: 1 } },
      { id: "Heading3", name: "Heading 3", basedOn: "Normal", next: "Normal", quickFormat: true,
        run: { font: T.headFont, size: 24, bold: true, color: T.accent }, paragraph: { spacing: { before: 240, after: 80 }, keepNext: true, outlineLevel: 2 } },
    ],
  },
  numbering: { config: [
    { reference: "heads", levels: [
      { level: 0, format: LevelFormat.DECIMAL, text: "%1.", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 0, hanging: 0 } } } },
      { level: 1, format: LevelFormat.DECIMAL, text: "%1.%2", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 0, hanging: 0 } } } },
      { level: 2, format: LevelFormat.DECIMAL, text: "%1.%2.%3", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 0, hanging: 0 } } } },
    ] },
    { reference: "bullets", levels: [{ level: 0, format: LevelFormat.BULLET, text: "•", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 540, hanging: 270 } } } }] },
    { reference: "numbers", levels: [{ level: 0, format: LevelFormat.DECIMAL, text: "%1.", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 540, hanging: 300 } } } }] },
  ] },
  sections: [{
    properties: { page: { size: { width: PAGE_W, height: 16838 }, margin: { top: MARGIN, bottom: MARGIN, left: MARGIN, right: MARGIN } }, titlePage: true },
    headers: { default: new Header({ children: [new Paragraph({ alignment: AlignmentType.RIGHT,
      children: [new TextRun({ text: `${cfg.shortTitle}  ·  ${cfg.kicker}`, size: 16, color: T.muted })] })] }) },
    footers: { default: new Footer({ children: [new Paragraph({ tabStops: [{ type: TabStopType.RIGHT, position: TabStopPosition.MAX }],
      children: [new TextRun({ text: cfg.footer, size: 16, color: T.muted }), new TextRun({ children: ["\tPage ", PageNumber.CURRENT, " of ", PageNumber.TOTAL_PAGES], size: 16, color: T.muted })] })] }) },
    children: [...(await cover()), ...(await body())],
  }],
});
fs.writeFileSync(OUT, await Packer.toBuffer(doc));
console.log(`wrote ${OUT}: ${figNo} figures, ${tableNo} tables`);
