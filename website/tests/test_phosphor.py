"""Contracts for the original landing-page layout with Story-style CRT colours."""
import unittest

import test_build
from test_build import Document, WEBSITE


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
            self.assertIn("data-motion-toggle", page.split("</header>")[0])
            self.assertEqual(page.count("data-motion-toggle"), 1)
            for kind in ("contact", "privacy"):
                legal = (self.out / lang / kind / "index.html").read_text()
                self.assertIn("assets/site.css", legal)

    def test_browser_recipes_follow_the_current_artifact_origin(self):
        for name in ("verify-browser", "verify-imprint", "verify-default-language", "verify-release",
                     "verify-headlines", "verify-headline-reveal", "verify-demos", "verify-phosphor", "verify-roll-directions"):
            source = (WEBSITE / "tests" / (name + ".js")).read_text()
            self.assertIn("new URL(page.url()).origin", source, name)
            self.assertNotIn("const origin = 'http://127.0.0.1:", source, name)

    def test_original_screenshot_hero_has_no_installation_atlas(self):
        self.require_build()
        for language in ("en", "de"):
            page = (self.out / language / "index.html").read_text()
            doc = Document(page)
            self.assertFalse(any(a.get("id") == "installation-atlas" or "data-atlas-node" in a
                                 for t, a in doc.elements), "owner removed the installation diagram")
            self.assertLess(page.index('class="hero-intro"'), page.index('class="hero-media"'))
            self.assertLess(page.index('class="hero-media"'), page.index('class="benefit-strip"'))
            self.assertNotIn("atlas.js", page)
        self.assertFalse((self.out / "assets/atlas.js").exists())
        self.assertFalse((WEBSITE / "atlas.html").exists())
        self.assertFalse((WEBSITE / "atlas.js").exists())
        self.assertNotIn(".atlas", (self.out / "assets/site.css").read_text())


if __name__ == "__main__":
    unittest.main()
