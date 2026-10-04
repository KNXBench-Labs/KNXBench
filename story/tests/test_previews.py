"""Versioned previews: every committed page is exactly what its candidate builds to today."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

from storytool.render import build

from helpers import TempDirTestCase

STORY_DIR = Path(__file__).resolve().parent.parent
PREVIEWS = STORY_DIR / "previews"
CANDIDATES = STORY_DIR / "candidates"
ISLAND = re.compile(r'<script id="story-data" type="application/json">(.*?)</script>', re.S)


def committed_previews() -> list[Path]:
    return sorted(PREVIEWS.glob("*.html")) if PREVIEWS.is_dir() else []


class PreviewTests(TempDirTestCase):
    def test_every_preview_belongs_to_a_candidate(self) -> None:
        for preview in committed_previews():
            with self.subTest(preview=preview.name):
                self.assertTrue((CANDIDATES / preview.stem / "manifest.json").is_file(),
                                f"{preview.name} has no candidate {preview.stem}")

    def test_every_preview_carries_its_candidate_payload_unchanged(self) -> None:
        for preview in committed_previews():
            with self.subTest(preview=preview.name):
                page = preview.read_text(encoding="utf-8")
                island = ISLAND.search(page)
                assert island is not None, f"{preview.name} has no data island"
                candidate = json.loads((CANDIDATES / preview.stem / "story.json").read_text(encoding="utf-8"))
                self.assertEqual(json.loads(island.group(1)), candidate)
                manifest = json.loads((CANDIDATES / preview.stem / "manifest.json").read_text(encoding="utf-8"))
                # The footer shows the first 16 hex digits of the content digest.
                self.assertIn(f"content digest <code>{manifest['story_sha256'][:16]}</code>", page)

    def test_every_preview_matches_a_fresh_build(self) -> None:
        # Fails when the site code or a candidate changed without regenerating the preview:
        # run `python3 -m storytool build <id> --preview` and commit the result.
        for preview in committed_previews():
            with self.subTest(preview=preview.name):
                fresh = build(CANDIDATES / preview.stem, self.tmp, preview.name)
                self.assertEqual(preview.read_bytes(), fresh.read_bytes(),
                                 f"{preview.name} is stale; rebuild it with --preview")

    def test_build_is_deterministic(self) -> None:
        candidates = sorted(path for path in CANDIDATES.iterdir() if (path / "manifest.json").is_file())
        if not candidates:
            self.skipTest("no candidates")
        first = build(candidates[-1], self.tmp / "a").read_bytes()
        second = build(candidates[-1], self.tmp / "b").read_bytes()
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
