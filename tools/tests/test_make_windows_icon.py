"""Tests for the PNG-in-ICO packer used for the Windows desktop icon."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import make_windows_icon as m


class IconTests(unittest.TestCase):
    def test_roundtrip_embeds_source_pngs_unchanged(self):
        images = [(m.ICON_DIR / n).read_bytes() for n in m.SOURCES]
        frames = m.ico_frames(m.build_ico(images))
        self.assertEqual(frames, images)
        self.assertEqual([m.png_size(f) for f in frames], [(128, 128), (256, 256)])

    def test_committed_icon_is_current(self):
        self.assertEqual(m.main(["--check"]), 0)

    def test_rejects_non_png_and_oversize(self):
        with self.assertRaises(ValueError):
            m.build_ico([b"nope" * 10])
        big = m.PNG_MAGIC + b"\0\0\0\rIHDR" + (512).to_bytes(4, "big") * 2
        with self.assertRaises(ValueError):
            m.build_ico([big])


if __name__ == "__main__":
    unittest.main()
