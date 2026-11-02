"""Regression tests for the restart-snapshot check and finished-track archiving."""

from contextlib import redirect_stdout
from datetime import date, datetime
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "compact_current_state", Path(__file__).resolve().parents[1] / "compact_current_state.py"
)
snapshot = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = snapshot
SPEC.loader.exec_module(snapshot)

ACTIVE = "## Flag declaration (IN PROGRESS; owner A; observed 2026-11-02 08:15 CET)\n- gate running\n"
DONE = "## Old fix (DELIVERED, RETIRED; observed 2026-10-28 23:01 CET)\n- merged `abc`\n"
OCTOBER = "## Older fix (PUBLISHED AND CLOSED; observed 2026-10-31)\n- merged `def`\n"
NOVEMBER = "## Stats (DONE, LOCAL ONLY; observed 2026-11-01 18:54 CET)\n- report\n"


class StatusTests(unittest.TestCase):
    def test_status_is_the_first_vocabulary_word_opening_a_parenthesis(self):
        cases = {
            "A (DELIVERED, RETIRED; observed 2026-10-28)": "DELIVERED",
            "B (PUBLISHED AND CLOSED; observed 2026-10-31)": "PUBLISHED",
            "C (IN PROGRESS; owner x; observed 2026-11-02)": "IN PROGRESS",
            "Alpha 19 (PUBLISHED; owner tag x (y); observed 2026-11-01)": "PUBLISHED",
            "Issue #30 (owner a; observed 2026-11-02)": None,
            "D (SOFTWARE REVIEWED; DELIVERY GO; observed 2026-10-30)": None,
            "E (DONEISH; observed 2026-10-30)": None,
            "F (DONE: all merged; observed 2026-10-30)": "DONE",
            "Retained boundaries": None,
        }
        for heading, status in cases.items():
            with self.subTest(heading=heading):
                self.assertEqual(snapshot.status_of(heading), status)

    def test_observed_time_with_and_without_clock(self):
        self.assertEqual(snapshot.observed_of("x (DONE; observed 2026-11-02 ~08:15 CET)"),
                         datetime(2026, 11, 2, 8, 15))
        self.assertEqual(snapshot.observed_of("x (DONE; observed 2026-11-01 CET)"), datetime(2026, 11, 1))
        self.assertIsNone(snapshot.observed_of("x (DONE; observed 2026-13-01)"))
        self.assertIsNone(snapshot.observed_of("x (DONE; 26 October 2026)"))


class ProblemTests(unittest.TestCase):
    def test_compact_snapshot_passes(self):
        self.assertEqual(snapshot.problems("# Restart snapshot\n\n" + ACTIVE), [])

    def test_finished_unknown_undated_and_long_snapshots_fail(self):
        text = "# R\n\n" + DONE + "## Notes\n- x\n" + "## Y (ACTIVE)\n" + "- l\n" * 50
        errors = snapshot.problems(text)
        self.assertTrue(any(e.startswith("finished track still listed: Old fix") for e in errors))
        self.assertTrue(any(e.startswith("no status") and e.endswith("Notes") for e in errors))
        self.assertTrue(any(e == "no `observed YYYY-MM-DD` time: Y (ACTIVE)" for e in errors))
        self.assertTrue(any(e.endswith("limit 50") for e in errors))

    def test_stale_warns_only_for_open_non_standing_tracks(self):
        text = (ACTIVE + DONE + "## Rules (STANDING; observed 2026-10-01)\n"
                + "## Parked (PARKED; observed 2026-10-20)\n")
        self.assertEqual(snapshot.stale(text, datetime(2026, 11, 2, 9)), ["Parked (PARKED; observed 2026-10-20)"])

    def test_repository_check_names_the_fix(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertEqual(snapshot.check(root), [])
            (root / ".ai").mkdir()
            (root / ".ai/CURRENT_STATE.md").write_text("# R\n" + DONE)
            [error] = snapshot.check(root)
            self.assertIn("compact_current_state.py", error)


class CompactTests(unittest.TestCase):
    def test_finished_tracks_move_byte_exact_by_observed_month(self):
        text = "# R\n\n" + DONE + "\n" + ACTIVE + OCTOBER + NOVEMBER + "\n\n"
        remaining, archived = snapshot.compact(text, date(2026, 11, 2))
        self.assertEqual(remaining, "# R\n\n" + ACTIVE)
        self.assertEqual(archived, {"2026-10": DONE + OCTOBER, "2026-11": NOVEMBER})

    def test_unknown_status_sections_are_never_archived(self):
        text = "# R\n\n## Notes\n- keep\n" + ACTIVE
        self.assertEqual(snapshot.compact(text, date(2026, 11, 2)), (text, {}))

    def test_run_appends_archive_and_rewrites_snapshot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".ai/archive/local").mkdir(parents=True)
            month = root / ".ai/archive/local/CURRENT_STATE_2026-10.md"
            month.write_text("# Archived restart-snapshot tracks, 2026-10\n\nearlier\n")
            (root / ".ai/CURRENT_STATE.md").write_text("# R\n\n" + DONE + ACTIVE)
            with redirect_stdout(io.StringIO()):
                code = snapshot.run(root, root / ".ai/archive/local", date(2026, 11, 2), dry_run=False)
            self.assertEqual(code, 0)
            self.assertEqual((root / ".ai/CURRENT_STATE.md").read_text(), "# R\n\n" + ACTIVE)
            archive = month.read_text()
            self.assertTrue(archive.startswith("# Archived restart-snapshot tracks, 2026-10\n\nearlier\n"))
            self.assertTrue(archive.endswith("<!-- archived 2026-11-02 from .ai/CURRENT_STATE.md -->\n" + DONE))

    def test_dry_run_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".ai").mkdir()
            (root / ".ai/CURRENT_STATE.md").write_text("# R\n\n" + DONE)
            with redirect_stdout(io.StringIO()):
                snapshot.run(root, root / ".ai/archive/local", date(2026, 11, 2), dry_run=True)
            self.assertEqual((root / ".ai/CURRENT_STATE.md").read_text(), "# R\n\n" + DONE)
            self.assertFalse((root / ".ai/archive").exists())


if __name__ == "__main__":
    unittest.main()
