"""Tests for the asset generators: deterministic, right sizes, committed files up to date.

Run: python3 -m unittest discover -s assets -p 'test_*.py'
"""
import pathlib
import subprocess
import sys
import tempfile
import unittest

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent


def run_icon(out: pathlib.Path) -> None:
    subprocess.run([sys.executable, str(ROOT / "assets" / "icon.py"), "--out", str(out)], check=True)


class IconTest(unittest.TestCase):
    def test_outputs_and_sizes(self):
        with tempfile.TemporaryDirectory() as d:
            out = pathlib.Path(d)
            run_icon(out)
            self.assertEqual(Image.open(out / "assets" / "icon.png").size, (256, 256))
            ico = Image.open(out / "assets" / "keypad.ico")
            self.assertIn((16, 16), ico.info["sizes"])
            self.assertIn((256, 256), ico.info["sizes"])
            mark = Image.open(out / "docs" / "assets" / "keypad-mark.png")
            self.assertEqual(mark.mode, "RGBA")
            self.assertEqual(mark.getpixel((0, 0))[3], 0, "mark background must be transparent")

    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            run_icon(pathlib.Path(a))
            run_icon(pathlib.Path(b))
            for rel in ["assets/icon.png", "assets/keypad.ico", "docs/assets/keypad-mark.png"]:
                self.assertEqual((pathlib.Path(a) / rel).read_bytes(), (pathlib.Path(b) / rel).read_bytes(), rel)

    def test_committed_files_are_current(self):
        with tempfile.TemporaryDirectory() as d:
            run_icon(pathlib.Path(d))
            for rel in ["assets/icon.png", "assets/keypad.ico", "docs/assets/keypad-mark.png"]:
                self.assertEqual((pathlib.Path(d) / rel).read_bytes(), (ROOT / rel).read_bytes(),
                                 f"{rel} is stale: run python3 assets/icon.py")


class BannerTest(unittest.TestCase):
    def run_banner(self, out):
        subprocess.run([sys.executable, str(ROOT / "assets" / "banner.py"), "--out", str(out)], check=True)

    def test_size_and_transparency(self):
        with tempfile.TemporaryDirectory() as d:
            self.run_banner(pathlib.Path(d))
            im = Image.open(pathlib.Path(d) / "docs" / "assets" / "keypad-banner.png")
            self.assertEqual(im.size, (2430, 420))
            self.assertEqual(im.getpixel((0, 0))[3], 0)

    def test_deterministic_and_current(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            self.run_banner(pathlib.Path(a))
            self.run_banner(pathlib.Path(b))
            rel = "docs/assets/keypad-banner.png"
            self.assertEqual((pathlib.Path(a) / rel).read_bytes(), (pathlib.Path(b) / rel).read_bytes())
            self.assertEqual((pathlib.Path(a) / rel).read_bytes(), (ROOT / rel).read_bytes(),
                             "banner is stale: run python3 assets/banner.py")


if __name__ == "__main__":
    unittest.main()
