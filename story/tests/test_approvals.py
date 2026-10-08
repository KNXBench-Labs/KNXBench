"""Committed approval records: each one names an existing, unmodified candidate exactly."""

from __future__ import annotations

import unittest
from pathlib import Path

from storytool.release import check_approval

STORY_DIR = Path(__file__).resolve().parent.parent
APPROVALS = STORY_DIR / "approvals"
CANDIDATES = STORY_DIR / "candidates"


class ApprovalRecordTests(unittest.TestCase):
    def test_every_committed_approval_matches_its_candidate(self) -> None:
        # Fails when a candidate is replaced or changed while its approval stays behind:
        # remove or renew the approval together with the candidate.
        records = sorted(APPROVALS.glob("*.json")) if APPROVALS.is_dir() else []
        for record in records:
            with self.subTest(approval=record.name):
                gate = check_approval(CANDIDATES / record.stem, record)
                self.assertTrue(gate.eligible, gate.reasons)


if __name__ == "__main__":
    unittest.main()
