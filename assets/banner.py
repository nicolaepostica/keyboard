#!/usr/bin/env python3
"""Draws the README banner from assets/theme.toml: the name spelled on key caps, then the
media keys keypad gives you (previous, play/pause, next). Transparent background.

Output (relative to --out, default: the repo root): docs/assets/keypad-banner.png (2430x420)
"""
import argparse
import pathlib
import tomllib

from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parent.parent
W, H = 2430, 420
SS = 2


def rgba(h: str) -> tuple:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def darker(c: tuple) -> tuple:
    return (round(c[0] * 0.8), round(c[1] * 0.8), round(c[2] * 0.8), c[3])


def cap(d: ImageDraw.ImageDraw, x: float, y: float, s: float, face, edge) -> tuple:
    """Draws one key cap; returns the centre of its top face."""
    r = s * 0.2
    depth = s * 0.07
    d.rounded_rectangle([x, y + depth, x + s, y + s], radius=r, fill=edge)
    d.rounded_rectangle([x, y, x + s, y + s - depth], radius=r, fill=face)
    return x + s / 2, y + (s - depth) / 2


def tri(d, cx, cy, h, fill, left=False):
    w = h * 0.85
    if left:
        d.polygon([(cx + w / 2, cy - h / 2), (cx + w / 2, cy + h / 2), (cx - w / 2, cy)], fill=fill)
    else:
        d.polygon([(cx - w / 2, cy - h / 2), (cx - w / 2, cy + h / 2), (cx + w / 2, cy)], fill=fill)


def bar(d, cx, cy, h, w, fill):
    d.rounded_rectangle([cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2], radius=w * 0.3, fill=fill)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=pathlib.Path, default=ROOT)
    args = ap.parse_args()
    t = tomllib.loads((ROOT / "assets" / "theme.toml").read_text())
    accent, edge, glyph, muted = rgba(t["accent"]), rgba(t["accent_press"]), rgba(t["on_accent"]), rgba(t["muted"])
    img = Image.new("RGBA", (W * SS, H * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    s = 230 * SS
    gap = 24 * SS
    y = (H * SS - s) / 2
    font = ImageFont.load_default(size=int(s * 0.55))
    total = 9 * s + 8 * gap + 2 * gap
    x = (W * SS - total) / 2
    for ch in t["name"]:
        cx, cy = cap(d, x, y, s, accent, edge)
        d.text((cx, cy), ch, font=font, fill=glyph, anchor="mm")
        x += s + gap
    x += gap * 2
    gh = s * 0.32
    # previous | play/pause | next, on muted caps
    for kind in ("prev", "play", "next"):
        cx, cy = cap(d, x, y, s, muted, darker(muted))
        if kind == "prev":
            bar(d, cx - gh * 0.55, cy, gh, gh * 0.18, glyph)
            tri(d, cx + gh * 0.15, cy, gh, glyph, left=True)
        elif kind == "play":
            tri(d, cx - gh * 0.35, cy, gh, glyph)
            bar(d, cx + gh * 0.35, cy, gh, gh * 0.2, glyph)
            bar(d, cx + gh * 0.7, cy, gh, gh * 0.2, glyph)
        else:
            tri(d, cx - gh * 0.15, cy, gh, glyph)
            bar(d, cx + gh * 0.55, cy, gh, gh * 0.18, glyph)
        x += s + gap
    out = args.out / "docs" / "assets"
    out.mkdir(parents=True, exist_ok=True)
    img.resize((W, H), Image.LANCZOS).save(out / "keypad-banner.png", optimize=False)


if __name__ == "__main__":
    main()
