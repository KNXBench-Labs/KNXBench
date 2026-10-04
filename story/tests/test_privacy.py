"""Privacy boundary: scanner coverage, refusal before writing, and no private data in shipped files."""

from __future__ import annotations

import json
import os
import unittest
from pathlib import Path

from storytool import privacy
from storytool.candidate import PrepareError, prepare
from storytool.render import build

from helpers import STORY_DIR, TempDirTestCase, load_fixture, mutated, write_json

PRIVATE_PROVENANCE = Path(os.environ.get(
    "KNXBENCH_STORY_PRIVATE_PROVENANCE",
    "/mnt/daten-i/Sourcecode/KNXBench.story-private/provenance.json"))

SYNTHETIC_PROVENANCE = {
    "schema": "knxbench-evolution-provenance/1",
    "sources": {"claude": "/home/someone/.claude/projects/-private-project/*.jsonl"},
    "events": {
        "seed": [{"kind": "claude", "session": "a976d9eb-06d*", "ts": "2000-01-01T00:00:00Z"}],
        "left-step": [{"kind": "hermes", "session": "20000101_123456*"}],
        "right-step": [{"kind": "codex", "path": "/home/someone/.codex/sessions/x.jsonl"}],
        "merge": [{"kind": "git", "commit": "abcdef1"}],
        "undated": [{"kind": "local", "note": "nothing private"}],
    },
}


class ScannerTests(unittest.TestCase):
    def categories(self, text: str, where=lambda c, t: c["events"][2].update(summary=t)) -> set[str]:
        return {f.category for f in privacy.scan(mutated(load_fixture(), lambda c: where(c, text)))
                if f.severity == "error"}

    def test_hard_patterns_inside_nested_event_fields(self) -> None:
        # Regression: an early version skipped the whole top-level events list.
        self.assertIn("local filesystem path", self.categories("see /home/alice/project"))
        self.assertIn("email address", self.categories("mail alice@example.org"))
        self.assertIn("IPv4 address", self.categories("gateway 192.0.2.10"))
        self.assertIn("UUID or session identifier", self.categories("id 1e318879-008e-40ce-a0ea-5e286717f162"))
        self.assertIn("credential assignment", self.categories("password: hunter2"))
        self.assertIn("provider API key", self.categories("sk-abcdefghijklmnopqrstuv"))

    def test_hard_patterns_in_excerpts_evidence_and_chapters(self) -> None:
        self.assertTrue(self.categories("/mnt/private/x", lambda c, t: c["events"][0]["excerpts"][0].update(text=t)))
        self.assertTrue(self.categories("/mnt/private/x", lambda c, t: c["events"][0]["evidence"][0].update(label=t)))
        self.assertTrue(self.categories("/mnt/private/x", lambda c, t: c["chapters"][0]["body"].append(t)))
        self.assertTrue(self.categories("/mnt/private/x", lambda c, t: c["gaps"][0].update(detail=t)))

    def test_soft_patterns_are_warnings(self) -> None:
        content = mutated(load_fixture(), lambda c: c["events"][1].update(summary="device 1.1.20 and 2/0/53"))
        findings = privacy.scan(content)
        self.assertTrue(findings)
        self.assertTrue(all(f.severity == "warning" for f in findings))

    def test_canaries_cover_paths_and_session_ids(self) -> None:
        canaries = privacy.private_canaries(SYNTHETIC_PROVENANCE)
        self.assertIn("a976d9eb-06d", canaries)
        self.assertIn("20000101_123456", canaries)
        self.assertIn("/home/someone/.codex/sessions/x.jsonl", canaries)


class BoundaryTests(TempDirTestCase):
    def test_hard_finding_refuses_and_writes_nothing(self) -> None:
        content = mutated(load_fixture(), lambda c: c["events"][0].update(why="ask alice@example.org"))
        path = write_json(self.tmp, "content.json", content)
        with self.assertRaises(PrepareError):
            prepare(path, self.tmp / "candidates")
        self.assertFalse((self.tmp / "candidates").exists())

    def test_leaked_private_locator_refuses(self) -> None:
        content = mutated(load_fixture(), lambda c: c["events"][1].update(why="as in session 20000101_123456"))
        path = write_json(self.tmp, "content.json", content)
        provenance = write_json(self.tmp, "provenance.json", SYNTHETIC_PROVENANCE)
        with self.assertRaises(PrepareError) as caught:
            prepare(path, self.tmp / "candidates", provenance_path=provenance)
        self.assertIn("private source locators", str(caught.exception))
        self.assertFalse((self.tmp / "candidates" / "fixture-1").exists())

    def test_private_provenance_never_reaches_candidate_or_preview(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        provenance = write_json(self.tmp, "provenance.json", SYNTHETIC_PROVENANCE)
        result = prepare(path, self.tmp / "candidates", provenance_path=provenance)
        html = build(result.directory, self.tmp / "dist").read_text(encoding="utf-8")
        shipped = [html] + [p.read_text(encoding="utf-8") for p in result.directory.iterdir()]
        for canary in privacy.private_canaries(SYNTHETIC_PROVENANCE) | {"/home/someone", "-private-project"}:
            for text in shipped:
                self.assertNotIn(canary, text)
        manifest = json.loads((result.directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["provenance"]["events_without_private_provenance"], [])
        self.assertEqual(manifest["provenance"]["private_locators_found_in_payload"], 0)

    @unittest.skipUnless(PRIVATE_PROVENANCE.is_file(),
                         "SKIP: private provenance ledger not present on this machine")
    def test_real_candidates_do_not_contain_real_private_locators(self) -> None:
        canaries = privacy.private_canaries(privacy.load_provenance(PRIVATE_PROVENANCE))
        self.assertGreater(len(canaries), 20, "the real ledger should yield many distinctive locators")
        candidates = sorted((STORY_DIR / "candidates").glob("*/story.json"))
        self.assertTrue(candidates, "expected at least one prepared real candidate")
        for story in candidates:
            text = story.read_text(encoding="utf-8")
            built = build(story.parent, self.tmp / story.parent.name).read_text(encoding="utf-8")
            for canary in canaries:
                self.assertNotIn(canary, text)
                self.assertNotIn(canary, built)


if __name__ == "__main__":
    unittest.main()
