"""Offline community demo tooling; real generators, never mocked app responses."""
from __future__ import annotations

import subprocess
import sys
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory
from xml.etree import ElementTree as ET

SCRIPT = Path(__file__).resolve().parents[1] / "community_demos.py"


class CommunityDemoTests(unittest.TestCase):
    def test_fictional_catalogue_is_a_deterministic_product_not_project_archive(self):
        with TemporaryDirectory() as tmp:
            outputs = []
            for name in ("first.knxprod", "second.knxprod"):
                path = Path(tmp) / name
                result = subprocess.run([sys.executable, str(SCRIPT), "catalog", str(path)],
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                outputs.append(path.read_bytes())
            self.assertEqual(outputs[0], outputs[1])
            with zipfile.ZipFile(Path(tmp) / "first.knxprod") as archive:
                self.assertIn("knx_master.xml", archive.namelist())
                self.assertFalse(any(p.startswith("P-") or p.endswith(".signature")
                                     for p in archive.namelist()))
                for name in archive.namelist():
                    root = ET.fromstring(archive.read(name))
                    self.assertTrue(root.tag.endswith("}KNX"))
                master = archive.read("knx_master.xml").decode()
                self.assertIn("KNXBench Demo Devices (fictional)", master)

    def test_three_specs_have_realistic_scales_and_typed_complete_links(self):
        import json
        with TemporaryDirectory() as tmp:
            out = Path(tmp) / "specs"
            result = subprocess.run([sys.executable, str(SCRIPT), "specs", str(out)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            data = json.loads((out / "projects.json").read_text())
        self.assertEqual([p["key"] for p in data["projects"]],
                         ["single-family-home", "multi-unit-residential", "office-building"])
        products = {p["key"]: p for p in data["products"]}
        for project, (minimum, maximum) in zip(data["projects"], [(25, 40), (80, 120), (150, 250)], strict=True):
            self.assertGreaterEqual(len(project["devices"]), minimum, project["key"])
            self.assertLessEqual(len(project["devices"]), maximum, project["key"])
            addresses = [tuple(d["address"]) for d in project["devices"]]
            self.assertEqual(len(addresses), len(set(addresses)))
            groups = {g["address"]: g for g in project["groups"]}
            self.assertEqual(len(groups), len(project["groups"]))
            linked = {g: set() for g in groups}
            parts = {p["key"] for p in project["parts"]}
            for device in project["devices"]:
                self.assertIn(device["part"], parts)
                product = products[device["product"]]
                objs = {o["number"]: o for o in product["objects"]}
                for binding in device["links"]:
                    obj = objs[binding["number"]]
                    for ga in binding["groups"]:
                        self.assertEqual(obj["dpt"], groups[ga]["dpt"], (device["name"], ga))
                        linked[ga].add(obj["direction"])
            self.assertTrue(all(directions == {"Send", "Receive"} for directions in linked.values()),
                            {g: v for g, v in linked.items() if v != {"Send", "Receive"}})

    def test_ui_script_binds_the_three_generated_targets(self):
        import json
        with TemporaryDirectory() as tmp:
            specs = Path(tmp) / "specs"
            result = subprocess.run([sys.executable, str(SCRIPT), "specs", str(specs)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            output = Path(tmp) / "verify.js"
            result = subprocess.run([sys.executable, str(SCRIPT), "ui-script", str(specs),
                                     str(Path(tmp) / "evidence"), str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            text = output.read_text()
            self.assertNotIn("/* DEMO_CONFIG */ null", text)
            self.assertNotIn("page.route(", text)
            raw = text.split("const config = ", 1)[1].split(";\n  if (!config)", 1)[0]
            config = json.loads(raw)
            self.assertEqual(len(config["targets"]), 3)
            self.assertEqual([p["objectNumber"] for p in config["targets"]], [4, 4, 8])
            self.assertEqual(config["origin"], "http://127.0.0.1:4826")
            result = subprocess.run([sys.executable, str(SCRIPT), "ui-script", str(specs),
                                     str(Path(tmp) / "evidence"), str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(output.read_text(), text)

    def test_build_produces_three_reproducible_downloads_and_never_overwrites(self):
        import hashlib
        with TemporaryDirectory() as tmp:
            outputs = []
            for name in ("first", "second"):
                out = Path(tmp) / name
                result = subprocess.run([sys.executable, str(SCRIPT), "build", str(out)],
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                zips = {p.name: p.read_bytes() for p in out.glob("*.zip")}
                self.assertEqual(len(zips), 4)  # three individual downloads plus one combined
                outputs.append(zips)
                for filename, blob in zips.items():
                    with zipfile.ZipFile(out / filename) as archive:
                        self.assertIn("SHA256SUMS", archive.namelist())
                        self.assertIn("fictional-demo-devices.knxprod", archive.namelist())
                        self.assertFalse(any(n.endswith(".knxproj") for n in archive.namelist()))
                        sums = archive.read("SHA256SUMS").decode().splitlines()
                        for row in sums:
                            digest, member = row.split("  ", 1)
                            self.assertEqual(hashlib.sha256(archive.read(member)).hexdigest(), digest)
                        self.assertIn("LICENSE", archive.namelist())
                        self.assertIn("source/projects.json", archive.namelist())
                        self.assertIn("source/tools/community_demo_layouts.py", archive.namelist())
                        guides = [n for n in archive.namelist() if n.endswith(".md")]
                        prose = "\n".join(archive.read(n).decode() for n in guides)
                        self.assertIn("description", prose)
                        self.assertNotIn("Rename a device", prose)
                        self.assertIn("auto-save", prose)
            self.assertEqual(outputs[0], outputs[1])
            result = subprocess.run([sys.executable, str(SCRIPT), "build", str(Path(tmp) / "first")],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual({p.name: p.read_bytes() for p in (Path(tmp) / "first").glob("*.zip")}, outputs[0])


if __name__ == "__main__":
    unittest.main()
