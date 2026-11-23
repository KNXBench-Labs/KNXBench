"""Tests for the PNG-in-ICO packer used for the Windows desktop icon."""

import struct
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

    def test_rejects_bad_magic_with_valid_ihdr(self):
        good = (m.ICON_DIR / m.SOURCES[0]).read_bytes()
        bad = b"\x88" + good[1:]
        self.assertEqual(bad[12:24], good[12:24])
        with self.assertRaisesRegex(ValueError, "not a PNG"):
            m.build_ico([bad])

    def test_ico_table_matches_independent_layout(self):
        ico = (m.ICON_DIR / "icon.ico").read_bytes()
        self.assertEqual(struct.unpack("<HHH", ico[:6]), (0, 1, 2))
        first = struct.unpack("<BBBBHHII", ico[6:22])
        second = struct.unpack("<BBBBHHII", ico[22:38])
        self.assertEqual(first[:6], (128, 128, 0, 0, 1, 32))
        self.assertEqual(second[:6], (0, 0, 0, 0, 1, 32))
        self.assertEqual((first[7], second[7]), (38, 38 + first[6]))
        self.assertEqual(len(ico), second[7] + second[6])

    def test_unknown_argument_is_refused_without_writing(self):
        target = m.ICON_DIR / "icon.ico"
        before = target.read_bytes()
        mtime = target.stat().st_mtime_ns
        self.assertEqual(m.main(["--chek"]), 2)
        self.assertEqual(target.read_bytes(), before)
        self.assertEqual(target.stat().st_mtime_ns, mtime)


if __name__ == "__main__":
    unittest.main()
