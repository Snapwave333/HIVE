import json
import re
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parent.parent
SOURCE_FONT = ROOT / "node_modules" / "material-symbols" / "material-symbols-rounded.woff2"
OUT_DIR = ROOT / "src" / "assets" / "icons"
OUT_FONT = OUT_DIR / "material-symbols-rounded-subset.woff2"
OUT_MANIFEST = OUT_DIR / "icons.json"
PINNED_AXES = {"GRAD": 0, "wght": 400, "opsz": 20}

ICON_TAG = re.compile(r"<M3Icon\b(.*?)/>", re.S)
NAME_LITERAL = re.compile(r'\bname="([a-z0-9_]+)"')
NAME_EXPR = re.compile(r"\bname=\{(.*?)\}", re.S)
ICON_PROP = re.compile(r'\bicon:\s*"([a-z0-9_]+)"')
STRING = re.compile(r'(?:^\s*|[?:]\s*)"([a-z0-9_]+)"')


def used_icon_names(src_dir: Path) -> list[str]:
    names = set()
    for path in sorted(src_dir.rglob("*.tsx")):
        text = path.read_text(encoding="utf-8")
        for tag in ICON_TAG.findall(text):
            names.update(NAME_LITERAL.findall(tag))
            for expr in NAME_EXPR.findall(tag):
                names.update(STRING.findall(expr))
        names.update(ICON_PROP.findall(text))
    return sorted(names)


def ligature_map(font: TTFont) -> tuple[dict[tuple[str, ...], str], dict[int, str]]:
    cmap = font.getBestCmap()
    by_components: dict[tuple[str, ...], str] = {}
    for lookup in font["GSUB"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            table = sub.ExtSubTable if lookup.LookupType == 7 else sub
            if type(table).__name__ != "LigatureSubst":
                continue
            for first, ligs in table.ligatures.items():
                for lig in ligs:
                    by_components[(first, *lig.Component)] = lig.LigGlyph
    return by_components, cmap


def resolve(names: list[str], font: TTFont) -> tuple[dict[str, str], set[str]]:
    by_components, cmap = ligature_map(font)
    resolved = {}
    letters = set()
    missing = []
    for name in names:
        glyphs = tuple(cmap.get(ord(ch)) for ch in name)
        if None in glyphs or glyphs not in by_components:
            missing.append(name)
            continue
        resolved[name] = by_components[glyphs]
        letters.update(glyphs)
    if missing:
        raise SystemExit(f"Not Material Symbols icons: {', '.join(missing)}")
    return resolved, letters


def fill_alternates(font: TTFont, glyphs: set[str]) -> set[str]:
    extra = set()
    for lookup in font["GSUB"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            table = sub.ExtSubTable if lookup.LookupType == 7 else sub
            if type(table).__name__ == "SingleSubst":
                for src, dst in table.mapping.items():
                    if src in glyphs:
                        extra.add(dst)
    return extra


def main() -> int:
    names = used_icon_names(ROOT / "src")
    if not names:
        raise SystemExit("No icon names found in src/")
    font = TTFont(SOURCE_FONT)
    resolved, letters = resolve(names, font)
    keep = set(resolved.values()) | letters | {".notdef", "space"}
    keep |= fill_alternates(font, set(resolved.values()))
    keep &= set(font.getGlyphOrder())

    options = subset.Options()
    options.layout_closure = False
    options.layout_features = ["*"]
    options.notdef_outline = True
    options.name_IDs = ["*"]
    options.glyph_names = False
    subsetter = subset.Subsetter(options)
    subsetter.populate(glyphs=sorted(keep))
    subsetter.subset(font)

    axes = {a.axisTag for a in font["fvar"].axes}
    font = instancer.instantiateVariableFont(font, {k: v for k, v in PINNED_AXES.items() if k in axes})
    font.flavor = "woff2"
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    font.save(OUT_FONT)
    OUT_MANIFEST.write_text(json.dumps({"pinned_axes": PINNED_AXES, "icons": names}, indent=2) + "\n", encoding="utf-8")

    before = SOURCE_FONT.stat().st_size
    after = OUT_FONT.stat().st_size
    print(f"{len(names)} icons, {len(keep)} glyphs: {before:,} -> {after:,} bytes ({after / before:.1%})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
