// Contact sheet for visual QA: node montage.mjs <out.png> <cols> <cellWidth> <files...>
import sharp from "sharp";
const [, , out, colsS, cellS, ...files] = process.argv;
const cols = +colsS, cw = +cellS, gap = 10;
const metas = await Promise.all(files.map((f) => sharp(f).metadata()));
const ch = Math.round(cw * (metas[0].height / metas[0].width));
const rows = Math.ceil(files.length / cols);
const tiles = await Promise.all(files.map((f) => sharp(f).resize(cw, ch, { fit: "contain", background: "#888" }).toBuffer()));
await sharp({ create: { width: cols * cw + (cols - 1) * gap, height: rows * ch + (rows - 1) * gap, channels: 3, background: "#ff00ff" } })
  .composite(tiles.map((input, i) => ({ input, left: (i % cols) * (cw + gap), top: Math.floor(i / cols) * (ch + gap) })))
  .png().toFile(out);
console.log("montage", out, files.length);
