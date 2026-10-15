"""Static contracts for the fictional Phosphor Atlas marketing illustration."""
import unittest

import test_build
from test_build import Document


class PhosphorTests(unittest.TestCase):
    setUp = test_build.BuildTests.setUp
    build = test_build.BuildTests.build
    require_build = test_build.BuildTests.require_build

    def test_palette_and_header_motion_control_are_site_wide(self):
        self.require_build()
        css = (self.out / "assets/site.css").read_text()
        for color in ("#03150c", "#0b2d1c", "#eef5e9", "#88ffad", "#c1aff2", "#e7c688"):
            self.assertIn(color, css)
        for lang in ("en", "de"):
            page = (self.out / lang / "index.html").read_text()
            header = page.split("</header>")[0]
            self.assertIn("data-motion-toggle", header)
            self.assertEqual(page.count("data-motion-toggle"), 1)
            for kind in ("contact", "privacy"):
                legal = (self.out / lang / kind / "index.html").read_text()
                self.assertIn("assets/site.css", legal)
    def test_fictional_atlas_precedes_real_product_evidence(self):
        self.require_build()
        for language in ("en", "de"):
            page = (self.out / language / "index.html").read_text()
            doc = Document(page)
            atlases = [a for t, a in doc.elements if a.get("id") == "installation-atlas"]
            self.assertEqual(len(atlases), 1, "missing fictional installation hero")
            self.assertLess(page.index('id="installation-atlas"'), page.index('class="hero-media"'))
            nodes = [a for t, a in doc.elements if "data-atlas-node" in a]
            self.assertEqual({a["data-atlas-node"] for a in nodes},
                             {"line", "switch", "sensor", "actuator", "group"})
            notes = {a.get("id") for t, a in doc.elements if t == "details"}
            self.assertTrue(all(a["href"][1:] in notes for a in nodes))
            for kind in ("topology", "logical"):
                self.assertTrue(any(a.get("data-relation") == kind for t, a in doc.elements))
            self.assertIn("1.1.10", page)
            self.assertIn("0/0/1", page)


if __name__ == "__main__":
    unittest.main()
