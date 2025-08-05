#!/usr/bin/env python3
"""Draws the keypad icon from assets/theme.toml: a function-key cap with a play/pause glyph.

Outputs (relative to --out, default: the repo root):
  assets/icon.png            256x256, window icon / .desktop / favicon
  assets/keypad.ico          16..256, Windows .exe resource
  docs/assets/keypad-mark.png  512x512 transparent, README header
"""
import argparse
import pathlib
import tomllib

from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
SS = 4  # supersampling factor for smooth edges


def hex_rgba(h: str) -> tuple:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def draw_mark(size: int, theme: dict) -> Image.Image:
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    accent, edge, glyph = hex_rgba(theme["accent"]), hex_rgba(theme["accent_press"]), hex_rgba(theme["on_accent"])
    m = round(s * 0.06)
    r = round(s * 0.2)
    # Key cap: the lower edge first (depth), then the top face slightly raised.
    d.rounded_rectangle([m, m + round(s * 0.05), s - m, s - m], radius=r, fill=edge)
    d.rounded_rectangle([m, m, s - m, s - m - round(s * 0.07)], radius=r, fill=accent)
    # Play triangle + pause bars, centred on the face.
    cy = (m + s - m - round(s * 0.07)) / 2
    h = s * 0.30
    x0 = s * 0.25
    d.polygon([(x0, cy - h / 2), (x0, cy + h / 2), (x0 + h * 0.85, cy)], fill=glyph)
    bw = s * 0.075
    gap = s * 0.06
    bx = x0 + h * 0.85 + s * 0.07
    for i in range(2):
        x = bx + i * (bw + gap)
        d.rounded_rectangle([x, cy - h / 2, x + bw, cy + h / 2], radius=round(bw * 0.3), fill=glyph)
    return img.resize((size, size), Image.LANCZOS)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=pathlib.Path, default=ROOT)
    args = ap.parse_args()
    theme = tomllib.loads((ROOT / "assets" / "theme.toml").read_text())
    (args.out / "assets").mkdir(parents=True, exist_ok=True)
    (args.out / "docs" / "assets").mkdir(parents=True, exist_ok=True)
    draw_mark(256, theme).save(args.out / "assets" / "icon.png", optimize=False)
    big = draw_mark(256, theme)
    big.save(args.out / "assets" / "keypad.ico",
             sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    draw_mark(512, theme).save(args.out / "docs" / "assets" / "keypad-mark.png", optimize=False)


if __name__ == "__main__":
    main()
