"""Tests for the asset generators: deterministic, right sizes, committed files up to date.

Run: python3 -m unittest discover -s assets -p 'test_*.py'
"""
import pathlib
import subprocess
import sys
import tempfile
import unittest

from PIL import Image, ImageChops, ImageStat

ROOT = pathlib.Path(__file__).resolve().parent.parent


def load(path: pathlib.Path) -> Image.Image:
    """Opens and fully reads an image, closing the file (Windows cannot delete open files)."""
    with Image.open(path) as im:
        im.load()
        out = im.copy()
        out.info.update(im.info)
        return out


def assert_same_picture(test, made: pathlib.Path, committed: pathlib.Path, hint: str) -> None:
    """Committed images must match a fresh render. Compared by pixels, not bytes: other
    Pillow/zlib/FreeType versions (CI runners) encode and anti-alias slightly differently."""
    a = load(made).convert("RGBA")
    b = load(committed).convert("RGBA")
    test.assertEqual(a.size, b.size, f"{committed.name}: size changed; {hint}")
    diff = ImageChops.difference(a, b)
    mean = sum(ImageStat.Stat(diff).mean) / 4
    test.assertLess(mean, 1.5, f"{committed.name} is stale (mean diff {mean:.2f}); {hint}")


def run_icon(out: pathlib.Path) -> None:
    subprocess.run([sys.executable, str(ROOT / "assets" / "icon.py"), "--out", str(out)], check=True)


class IconTest(unittest.TestCase):
    def test_outputs_and_sizes(self):
        with tempfile.TemporaryDirectory() as d:
            out = pathlib.Path(d)
            run_icon(out)
            self.assertEqual(load(out / "assets" / "icon.png").size, (256, 256))
            with Image.open(out / "assets" / "keyboard.ico") as ico:
                sizes = ico.info["sizes"]
            self.assertIn((16, 16), sizes)
            self.assertIn((256, 256), sizes)
            mark = load(out / "docs" / "assets" / "keyboard-mark.png")
            self.assertEqual(mark.mode, "RGBA")
            self.assertEqual(mark.getpixel((0, 0))[3], 0, "mark background must be transparent")

    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            run_icon(pathlib.Path(a))
            run_icon(pathlib.Path(b))
            for rel in ["assets/icon.png", "assets/keyboard.ico", "docs/assets/keyboard-mark.png"]:
                self.assertEqual((pathlib.Path(a) / rel).read_bytes(), (pathlib.Path(b) / rel).read_bytes(), rel)

    def test_committed_files_are_current(self):
        with tempfile.TemporaryDirectory() as d:
            run_icon(pathlib.Path(d))
            for rel in ["assets/icon.png", "assets/keyboard.ico", "docs/assets/keyboard-mark.png"]:
                assert_same_picture(self, pathlib.Path(d) / rel, ROOT / rel, "run python3 assets/icon.py")


class BannerTest(unittest.TestCase):
    def run_banner(self, out):
        subprocess.run([sys.executable, str(ROOT / "assets" / "banner.py"), "--out", str(out)], check=True)

    def test_size_and_transparency(self):
        with tempfile.TemporaryDirectory() as d:
            self.run_banner(pathlib.Path(d))
            im = load(pathlib.Path(d) / "docs" / "assets" / "keyboard-banner.png")
            self.assertEqual(im.size, (2430, 420))
            self.assertEqual(im.getpixel((0, 0))[3], 0)

    def test_deterministic_and_current(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            self.run_banner(pathlib.Path(a))
            self.run_banner(pathlib.Path(b))
            rel = "docs/assets/keyboard-banner.png"
            self.assertEqual((pathlib.Path(a) / rel).read_bytes(), (pathlib.Path(b) / rel).read_bytes())
            assert_same_picture(self, pathlib.Path(a) / rel, ROOT / rel, "run python3 assets/banner.py")


if __name__ == "__main__":
    unittest.main()
