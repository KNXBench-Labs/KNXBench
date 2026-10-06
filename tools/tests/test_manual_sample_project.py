"""Checks for the fictional manual sample project (tools/manual_sample_project.py)."""
from __future__ import annotations

import hashlib
import re
import subprocess
import sys
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT = Path(__file__).resolve().parent.parent / "manual_sample_project.py"


def build(target: Path) -> bytes:
    subprocess.run([sys.executable, str(SCRIPT), str(target)], check=True)
    return target.read_bytes()


class ManualSampleProjectTests(unittest.TestCase):
    def test_build_is_deterministic(self) -> None:
        with TemporaryDirectory() as tmp:
            first = build(Path(tmp) / "a.knxproj")
            second = build(Path(tmp) / "b.knxproj")
        self.assertEqual(hashlib.sha256(first).hexdigest(), hashlib.sha256(second).hexdigest())

    def test_archive_has_project_master_and_product_members(self) -> None:
        with TemporaryDirectory() as tmp:
            target = Path(tmp) / "sample.knxproj"
            build(target)
            with zipfile.ZipFile(target) as archive:
                names = set(archive.namelist())
                texts = {name: archive.read(name).decode("utf-8") for name in names}
        self.assertIn("P-0001/0.xml", names)
        self.assertIn("knx_master.xml", names)
        self.assertEqual(4, sum(1 for n in names if re.fullmatch(r"M-7FF0/M-7FF0_A-\d{4}-10-0001\.xml", n)))
        topology = texts["P-0001/0.xml"]
        self.assertEqual(8, topology.count("<DeviceInstance "))
        self.assertEqual(15, topology.count("<GroupAddress "))
        for text in texts.values():
            if text.startswith("<?xml"):
                self.assertIn('xmlns="http://knx.org/xml/project/11"', text)

    def test_only_the_fictional_manufacturer_appears(self) -> None:
        with TemporaryDirectory() as tmp:
            target = Path(tmp) / "sample.knxproj"
            build(target)
            with zipfile.ZipFile(target) as archive:
                blob = "".join(archive.read(n).decode("utf-8") for n in archive.namelist())
        self.assertEqual({"M-7FF0"}, set(re.findall(r"M-[0-9A-F]{4}", blob)))
        self.assertIn("Example Devices (fictional)", blob)


if __name__ == "__main__":
    unittest.main()
