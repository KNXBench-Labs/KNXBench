"""Candidates: deterministic preparation, immutability, incremental diffs and retained versions."""

from __future__ import annotations

import json
import stat
import unittest

from storytool.candidate import PrepareError, existing_candidates, prepare

from helpers import TempDirTestCase, load_fixture, mutated, write_json


def _snapshot(directory):
    return {path.name: path.read_bytes() for path in sorted(directory.iterdir())}


class CandidateTests(TempDirTestCase):
    def test_preparation_is_deterministic_across_directories(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        first = prepare(path, self.tmp / "a")
        second = prepare(path, self.tmp / "b")
        self.assertEqual(_snapshot(first.directory), _snapshot(second.directory))

    def test_candidate_files_are_read_only_and_include_review_material(self) -> None:
        result = prepare(write_json(self.tmp, "content.json", load_fixture()), self.tmp / "candidates")
        names = sorted(p.name for p in result.directory.iterdir())
        self.assertEqual(names, ["CHANGES.md", "REVIEW.md", "manifest.json", "story.json"])
        for path in result.directory.iterdir():
            self.assertFalse(path.stat().st_mode & stat.S_IWUSR, f"{path.name} should be read-only")
        review = (result.directory / "REVIEW.md").read_text(encoding="utf-8")
        self.assertIn("not approved for publication", review)
        self.assertIn("Synthetic uncertainty that must survive preparation.", review)
        self.assertIn("`rel-right-merge`", review.replace("`right-step` → `merge`", "`rel-right-merge`"))

    def test_repeat_prepare_is_unchanged_and_conflicting_content_is_refused(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        first = prepare(path, self.tmp / "candidates")
        before = _snapshot(first.directory)
        self.assertEqual(prepare(path, self.tmp / "candidates").status, "unchanged")
        write_json(self.tmp, "content.json", mutated(load_fixture(), lambda c: c["events"][0].update(title="Rewritten")))
        with self.assertRaises(PrepareError) as caught:
            prepare(path, self.tmp / "candidates")
        self.assertIn("never rewritten", str(caught.exception))
        self.assertEqual(_snapshot(first.directory), before, "the earlier candidate must stay byte-identical")

    def test_incremental_update_diffs_against_and_retains_the_previous_candidate(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        first = prepare(path, self.tmp / "candidates")
        retained = _snapshot(first.directory)

        def update(c):
            c["edition"]["id"] = "fixture-2"
            c["edition"]["cutoff"]["label"] = "Later synthetic cutoff"
            c["events"].append(dict(c["events"][3], id="after-merge", date="2000-01-06", title="After merge"))
            c["relations"].append({"id": "rel-merge-after", "from": "merge", "to": "after-merge",
                                   "type": "documented_cause", "note": "Fixture."})
            c["chapters"][1]["events"].append("after-merge")
            c["events"][1]["summary"] = "Synthetic left, clarified."
        write_json(self.tmp, "content.json", mutated(load_fixture(), update))
        second = prepare(path, self.tmp / "candidates")
        manifest = json.loads((second.directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["base_candidate"], "fixture-1")
        diff = manifest["diff"]
        self.assertTrue(diff["cutoff_changed"])
        self.assertEqual(diff["events"]["added"], ["after-merge"])
        self.assertEqual(diff["events"]["changed"], {"left-step": ["summary"]})
        self.assertEqual(diff["relations"]["added"], ["rel-merge-after"])
        changes = (second.directory / "CHANGES.md").read_text(encoding="utf-8")
        self.assertIn("Added `after-merge`: After merge", changes)
        self.assertIn("Changed `left-step`: summary", changes)
        self.assertEqual(_snapshot(first.directory), retained)
        self.assertEqual(existing_candidates(self.tmp / "candidates"), ["fixture-1", "fixture-2"])

    def test_candidate_ordering_is_natural(self) -> None:
        for edition in ("e.9", "e.10", "e.2"):
            write_json(self.tmp, "content.json", mutated(load_fixture(), lambda c, e=edition: c["edition"].update(id=e)))
            prepare(self.tmp / "content.json", self.tmp / "candidates")
        self.assertEqual(existing_candidates(self.tmp / "candidates"), ["e.2", "e.9", "e.10"])
        manifest = json.loads((self.tmp / "candidates" / "e.2" / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["base_candidate"], "e.10", "the latest existing candidate is the default base")

    def test_unknown_base_is_refused(self) -> None:
        with self.assertRaises(PrepareError):
            prepare(write_json(self.tmp, "content.json", load_fixture()), self.tmp / "candidates", base_id="nope")

    def test_malformed_json_is_refused_without_writing(self) -> None:
        path = self.tmp / "content.json"
        path.write_text("{ not json", encoding="utf-8")
        with self.assertRaises(PrepareError):
            prepare(path, self.tmp / "candidates")
        self.assertFalse((self.tmp / "candidates").exists())


if __name__ == "__main__":
    unittest.main()
