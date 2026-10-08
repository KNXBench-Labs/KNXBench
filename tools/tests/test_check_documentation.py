"""Regression tests for documentation links and first/last chapter navigation."""

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "check_documentation", Path(__file__).resolve().parents[1] / "check_documentation.py"
)
checker = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checker
SPEC.loader.exec_module(checker)


class DocumentationTests(unittest.TestCase):
    def test_fences_inline_code_and_comments_are_not_links(self):
        text = "```md\n[x](missing)\n```\n`[y](missing)`\n<!--[z](missing)-->\n[real](ok.md)"
        self.assertEqual([link.target for link in checker.links_in(text)], ["ok.md"])

    def test_multiline_image_and_balanced_parentheses(self):
        links = checker.links_in("![A screenshot\nwith a caption](a(b).png) [page](x.md#part)")
        self.assertEqual(links[0], checker.Link("a(b).png", True, "A screenshot\nwith a caption"))
        self.assertEqual(links[1].target, "x.md#part")

    def test_html_images_and_reference_links(self):
        links = checker.links_in('[page][ref]\n[ref]: target.md\n<img src="view.png" alt="Real view">')
        self.assertEqual(links, [checker.Link("target.md"), checker.Link("view.png", True, "Real view")])

    def test_undefined_reference_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "Undefined reference"):
            checker.links_in("[page][missing]")

    def test_unclosed_destination_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "Unclosed"):
            checker.links_in("[page](unclosed")

    def test_empty_destination_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "Empty"):
            checker.links_in("[page]()")

    def test_remote_urls_and_heading_only_links_are_not_local_files(self):
        source = Path("/docs/example.md")
        for target in ["https://example.com/x", "mailto:example@example.com", "#section", "//example.com/x"]:
            self.assertIsNone(checker.local_target(source, target))
        self.assertEqual(checker.local_target(source, "a%20b.md#part"), Path("/docs/a b.md"))

    def fixture(self, root):
        manual = root / "docs/manual"
        manual.mkdir(parents=True)
        (manual / "README.md").write_text("1. [One](one.md)\n2. [Two](two.md)\n")
        (manual / "one.md").write_text("Previous: [Index](README.md)\n\n# One\n\nNext: [Two](two.md)\n")
        (manual / "two.md").write_text("Previous: [One](one.md)\n\n# Two\n\nNext: [Index](README.md)\n")
        return manual

    def test_complete_chapter_chain(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            counts, errors = checker.check(root)
            self.assertEqual(errors, [])
            self.assertEqual(counts["chapters"], 2)

    def test_wrong_or_nonboundary_navigation_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manual = self.fixture(root)
            (manual / "two.md").write_text("Previous: [Index](README.md)\n# Two\nNext: [Index](README.md)\nExtra text\n")
            _, errors = checker.check(root)
            self.assertEqual(len(errors), 2)

    def test_duplicate_or_unindexed_chapter_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manual = self.fixture(root)
            (manual / "unlisted.md").write_text("# Unlisted\n")
            _, errors = checker.check(root)
            self.assertTrue(any("exactly once" in error for error in errors))

    def test_secondary_link_cannot_hide_wrong_navigation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manual = self.fixture(root)
            (manual / "one.md").write_text("Previous: [Wrong](two.md) · [Index](README.md)\n# One\nNext: [Two](two.md)\n")
            _, errors = checker.check(root)
            self.assertEqual(len(errors), 1)
            self.assertIn("incorrect Previous:", errors[0])

    def test_missing_file_and_empty_image_description_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            (root / "README.md").write_text('[missing](lost.md)\n<img src="lost.png" alt="">')
            _, errors = checker.check(root)
            self.assertEqual(len(errors), 3)


if __name__ == "__main__":
    unittest.main()
