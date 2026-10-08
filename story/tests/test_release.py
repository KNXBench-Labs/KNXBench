"""Publication gate: preparing never publishes, and stale or mismatched approvals are refused."""

from __future__ import annotations

import contextlib
import io
import json
import unittest

from storytool.__main__ import main
from storytool.candidate import prepare
from storytool.release import APPROVAL_DECISION, APPROVAL_SCHEMA, check_approval

from helpers import TempDirTestCase, load_fixture, mutated, write_json


class ReleaseGateTests(TempDirTestCase):
    def setUp(self) -> None:
        super().setUp()
        self.content = write_json(self.tmp, "content.json", load_fixture())
        self.first = prepare(self.content, self.tmp / "candidates")

    def approval(self, name: str, **overrides) -> dict:
        record = {"schema": APPROVAL_SCHEMA, "decision": APPROVAL_DECISION, "candidate_id": "fixture-1",
                  "story_sha256": self.first.story_sha256, "approved_by": "reviewer", "approved_at": "2000-01-02"}
        record.update(overrides)
        write_json(self.tmp, name, record)
        return record

    def run_cli(self, *argv: str) -> tuple[int, str]:
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = main(list(argv))
        return code, err.getvalue()

    def test_prepare_never_creates_an_approval(self) -> None:
        self.assertFalse(any("approv" in p.name.lower() for p in self.tmp.rglob("*") if p.is_file()))
        manifest = json.loads((self.first.directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertIs(manifest["publication"]["approved"], False)
        gate = check_approval(self.first.directory, None)
        self.assertFalse(gate.eligible)
        self.assertIn("no approval record", gate.reasons[0])

    def test_exact_approval_is_eligible(self) -> None:
        self.approval("approval.json")
        self.assertTrue(check_approval(self.first.directory, self.tmp / "approval.json").eligible)

    def test_stale_approval_for_older_content_is_refused(self) -> None:
        self.approval("approval.json")
        write_json(self.tmp, "content.json", mutated(load_fixture(), lambda c: (
            c["edition"].update(id="fixture-2"), c["events"][0].update(summary="Changed after approval."))))
        second = prepare(self.content, self.tmp / "candidates")
        self.approval("stale.json", candidate_id="fixture-2")  # right id, but the sha of fixture-1
        gate = check_approval(second.directory, self.tmp / "stale.json")
        self.assertFalse(gate.eligible)
        self.assertTrue(any("stale" in reason for reason in gate.reasons))
        gate = check_approval(second.directory, self.tmp / "approval.json")
        self.assertTrue(any("not 'fixture-2'" in reason for reason in gate.reasons))

    def test_tampering_after_approval_is_refused(self) -> None:
        self.approval("approval.json")
        story = self.first.directory / "story.json"
        story.chmod(0o644)
        story.write_text(story.read_text(encoding="utf-8").replace("Synthetic seed.", "Quietly improved."),
                         encoding="utf-8")
        gate = check_approval(self.first.directory, self.tmp / "approval.json")
        self.assertFalse(gate.eligible)
        self.assertIn("modified after preparation", gate.reasons[0])

    def test_wrong_decision_or_incomplete_record_is_refused(self) -> None:
        self.approval("prepare-only.json", decision="prepare-an-update")
        self.assertFalse(check_approval(self.first.directory, self.tmp / "prepare-only.json").eligible)
        self.approval("anonymous.json", approved_by="")
        self.assertFalse(check_approval(self.first.directory, self.tmp / "anonymous.json").eligible)
        (self.tmp / "broken.json").write_text("{", encoding="utf-8")
        self.assertFalse(check_approval(self.first.directory, self.tmp / "broken.json").eligible)

    def test_publish_command_always_refuses_even_with_valid_approval(self) -> None:
        self.approval("approval.json")
        code, err = self.run_cli("publish", "fixture-1")
        self.assertEqual(code, 3)
        self.assertIn("outside this tool", err)
        code, _ = self.run_cli("release-check", "fixture-1", "--candidates", str(self.tmp / "candidates"),
                               "--approval", str(self.tmp / "approval.json"))
        self.assertEqual(code, 0)

    def test_preview_server_refuses_non_loopback_addresses(self) -> None:
        for host in ("0.0.0.0", "192.0.2.1", "::"):
            code, err = self.run_cli("serve", "fixture-1", "--host", host)
            self.assertEqual(code, 3)
            self.assertIn("loopback", err)


if __name__ == "__main__":
    unittest.main()


class PublishedVariantTests(TempDirTestCase):
    """The published page variant exists only behind an exact approval record."""

    def setUp(self) -> None:
        super().setUp()
        self.content = write_json(self.tmp, "content.json", load_fixture())
        self.first = prepare(self.content, self.tmp / "candidates")

    approval = ReleaseGateTests.approval
    run_cli = ReleaseGateTests.run_cli

    def build_cli(self, approval: str) -> tuple[int, str]:
        return self.run_cli("build", "fixture-1", "--candidates", str(self.tmp / "candidates"),
                            "--out", str(self.tmp / "published"), "--approval", str(self.tmp / approval))

    def test_matching_approval_renders_the_published_variant(self) -> None:
        self.approval("ok.json")
        code, err = self.build_cli("ok.json")
        self.assertEqual(code, 0, err)
        page = (self.tmp / "published" / "index.html").read_text(encoding="utf-8")
        self.assertNotIn('class="preview-banner"', page)
        self.assertNotIn("noindex", page)
        self.assertNotIn("Not published", page)
        self.assertIn("Published edition", page)
        self.assertIn("approved for publication by the project owner", page)

    def test_stale_or_missing_approval_writes_nothing(self) -> None:
        self.approval("stale.json", story_sha256="0" * 64)
        for name in ("stale.json", "missing.json"):
            code, err = self.build_cli(name)
            self.assertNotEqual(code, 0)
            self.assertIn("refused", err)
            self.assertFalse((self.tmp / "published").exists())

    def test_approval_never_rewrites_a_committed_preview(self) -> None:
        self.approval("ok.json")
        code, err = self.run_cli("build", "fixture-1", "--candidates", str(self.tmp / "candidates"),
                                 "--preview", "--approval", str(self.tmp / "ok.json"))
        self.assertNotEqual(code, 0)
        self.assertIn("never a committed preview", err)

    def test_preview_without_approval_keeps_its_banner(self) -> None:
        code, err = self.run_cli("build", "fixture-1", "--candidates", str(self.tmp / "candidates"),
                                 "--out", str(self.tmp / "preview"))
        self.assertEqual(code, 0, err)
        page = (self.tmp / "preview" / "index.html").read_text(encoding="utf-8")
        self.assertIn('class="preview-banner"', page)
        self.assertIn("noindex", page)
