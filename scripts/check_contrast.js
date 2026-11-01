import config from "../tailwind.config.js";

const c = config.theme.extend.colors.hive;

function luminance(hex) {
  const n = hex.replace("#", "");
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(n.slice(i, i + 2), 16) / 255);
  const f = (v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

function ratio(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const backgrounds = ["base", "panel", "surface", "surfaceHover", "incoming", "outgoing", "highlight"];
const textColors = ["dimText", "brightText", "lime"];
const AA_TEXT = 4.5;

let failures = 0;
let worst = Infinity;
for (const fg of textColors) {
  for (const bg of backgrounds) {
    const r = ratio(c[fg], c[bg]);
    worst = Math.min(worst, r);
    if (r < AA_TEXT) {
      console.error(`FAIL: ${fg} (${c[fg]}) on ${bg} (${c[bg]}) is ${r.toFixed(2)}:1, needs ${AA_TEXT}:1`);
      failures++;
    }
  }
}

if (failures > 0) process.exit(1);
console.log(`✓ WCAG AA contrast: ${textColors.length * backgrounds.length} text/background pairs pass (lowest ${worst.toFixed(2)}:1).`);
