"""Pack the project's own PNG icons into a Windows icon.ico without resampling."""

import struct
import sys
from pathlib import Path

ICON_DIR = Path(__file__).resolve().parent.parent / "apps/knx-desktop/src-tauri/icons"
SOURCES = ("128x128.png", "256x256.png")
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"


def png_size(data):
    """Return (width, height) from a PNG IHDR; reject anything else."""
    if data[:8] != PNG_MAGIC or data[12:16] != b"IHDR":
        raise ValueError("not a PNG")
    return struct.unpack(">II", data[16:24])


def build_ico(images):
    """Build an ICO container embedding each PNG unchanged (PNG-in-ICO)."""
    entries, offset = [], 6 + 16 * len(images)
    for data in images:
        w, h = png_size(data)
        if w > 256 or h > 256:
            raise ValueError("ICO frames are limited to 256x256")
        entries.append(struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    return struct.pack("<HHH", 0, 1, len(images)) + b"".join(entries) + b"".join(images)


def ico_frames(ico):
    """Return the embedded PNG payloads of an ICO built by build_ico."""
    reserved, kind, count = struct.unpack("<HHH", ico[:6])
    if (reserved, kind) != (0, 1):
        raise ValueError("not an ICO")
    frames = []
    for i in range(count):
        size, off = struct.unpack("<II", ico[6 + 16 * i + 8 : 6 + 16 * i + 16])
        frames.append(ico[off : off + size])
    return frames


def main(argv):
    if any(arg != "--check" for arg in argv):
        sys.stderr.write("usage: make_windows_icon.py [--check]\n")
        return 2
    images = [(ICON_DIR / name).read_bytes() for name in SOURCES]
    target = ICON_DIR / "icon.ico"
    ico = build_ico(images)
    if "--check" in argv:
        return 0 if target.read_bytes() == ico else 1
    target.write_bytes(ico)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
