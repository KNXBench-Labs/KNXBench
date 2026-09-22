from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from tools.agent_memory_sync import SourceRoot, scan_sources


class ScanSourcesTests(unittest.TestCase):
    def test_extracts_bounded_markdown_topic_with_provenance(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp) / "claude"
            root.mkdir()
            note = root / "KNX Safety.md"
            note.write_text(
                "# KNX safety\n\nNever write to live hardware without approval.\n",
                encoding="utf-8",
            )

            result = scan_sources([SourceRoot("claude", root)])

            self.assertEqual(len(result.notes), 1)
            self.assertEqual(result.notes[0].topic, "knx-safety")
            self.assertEqual(result.notes[0].title, "KNX safety")
            self.assertEqual(
                result.notes[0].summary,
                "Never write to live hardware without approval.",
            )
            self.assertEqual(result.notes[0].owner, "claude")
            self.assertEqual(result.notes[0].relative_path, Path("KNX Safety.md"))
            self.assertEqual(len(result.notes[0].digest), 64)

    def test_discovery_rejects_escaping_symlink(self) -> None:
        with TemporaryDirectory() as tmp:
            base = Path(tmp)
            root = base / "codex"
            root.mkdir()
            outside = base / "outside.md"
            outside.write_text("# Secret\n\noutside\n", encoding="utf-8")
            (root / "escape.md").symlink_to(outside)

            result = scan_sources([SourceRoot("codex", root)])

            self.assertEqual(result.notes, ())
            self.assertEqual(result.skipped[0].reason, "path escapes source root")

    def test_discovery_skips_oversized_file(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "large.md").write_text("x" * 33, encoding="utf-8")

            result = scan_sources(
                [SourceRoot("hermes", root)], max_file_bytes=32
            )

            self.assertEqual(result.notes, ())
            self.assertEqual(result.skipped[0].reason, "file exceeds 32 bytes")

    def test_missing_optional_root_is_warning(self) -> None:
        with TemporaryDirectory() as tmp:
            missing = Path(tmp) / "missing"

            result = scan_sources([SourceRoot("hermes", missing)])

            self.assertEqual(result.errors, ())
            self.assertIn("optional source root missing", result.warnings[0])

    def test_unreadable_configured_root_is_error(self) -> None:
        with TemporaryDirectory() as tmp:
            not_a_directory = Path(tmp) / "memory.md"
            not_a_directory.write_text("# note\n", encoding="utf-8")

            result = scan_sources(
                [SourceRoot("codex", not_a_directory, required=True)]
            )

            self.assertIn("source root is not a directory", result.errors[0])

    def test_unreadable_existing_entry_is_error(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "broken.md").symlink_to(root / "missing-target.md")

            result = scan_sources([SourceRoot("claude", root)])

            self.assertEqual(result.notes, ())
            self.assertIn("unreadable path", result.errors[0])

    def test_malformed_utf8_is_replaced_and_reported(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "broken.md").write_bytes(b"# Broken\n\ntext \xff value\n")

            result = scan_sources([SourceRoot("claude", root)])

            self.assertIn("\ufffd", result.notes[0].summary)
            self.assertIn("invalid UTF-8 replaced", result.warnings[0])

    def test_scan_order_is_stable_across_creation_order(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "zeta.md").write_text("# Zeta\n\nlast\n", encoding="utf-8")
            (root / "alpha.md").write_text("# Alpha\n\nfirst\n", encoding="utf-8")

            result = scan_sources([SourceRoot("codex", root)])

            self.assertEqual(
                [note.relative_path.as_posix() for note in result.notes],
                ["alpha.md", "zeta.md"],
            )


if __name__ == "__main__":
    unittest.main()
