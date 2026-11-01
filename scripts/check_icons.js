import fs from "fs";
import path from "path";

const root = process.cwd();
const manifestPath = path.join(root, "src", "assets", "icons", "icons.json");
const fontPath = path.join(root, "src", "assets", "icons", "material-symbols-rounded-subset.woff2");

const iconTag = /<M3Icon\b([\s\S]*?)\/>/g;
const nameLiteral = /\bname="([a-z0-9_]+)"/g;
const nameExpr = /\bname=\{([\s\S]*?)\}/g;
const iconProp = /\bicon:\s*"([a-z0-9_]+)"/g;
const stringLit = /"([a-z0-9_]+)"/g;

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (entry.name.endsWith(".tsx")) out.push(full);
  }
  return out;
}

const used = new Set();
for (const file of walk(path.join(root, "src"))) {
  const text = fs.readFileSync(file, "utf-8");
  for (const [, tag] of text.matchAll(iconTag)) {
    for (const [, n] of tag.matchAll(nameLiteral)) used.add(n);
    for (const [, expr] of tag.matchAll(nameExpr)) for (const [, n] of expr.matchAll(stringLit)) used.add(n);
  }
  for (const [, n] of text.matchAll(iconProp)) used.add(n);
}

if (!fs.existsSync(manifestPath) || !fs.existsSync(fontPath)) {
  console.error("FAIL: icon subset missing. Run: python scripts/subset_icons.py");
  process.exit(1);
}

const bundled = new Set(JSON.parse(fs.readFileSync(manifestPath, "utf-8")).icons);
const missing = [...used].filter((n) => !bundled.has(n)).sort();
const unused = [...bundled].filter((n) => !used.has(n)).sort();

if (missing.length > 0) {
  console.error(`FAIL: icons used but not in the subset font: ${missing.join(", ")}`);
  console.error("Run: python scripts/subset_icons.py");
  process.exit(1);
}
const kb = (fs.statSync(fontPath).size / 1024).toFixed(1);
console.log(`✓ Icon subset covers all ${used.size} icons in use (${kb} KB)${unused.length ? `, ${unused.length} unused: ${unused.join(", ")}` : ""}.`);
