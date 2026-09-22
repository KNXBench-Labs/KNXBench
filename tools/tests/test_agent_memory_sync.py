from __future__ import annotations

import unittest
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory

from tools.agent_memory_sync import (
    ScanResult,
    SourceRoot,
    build_manifest,
    reconcile,
    render_index,
    render_report,
    scan_sources,
)


def make_note(owner: str, filename: str, text: str):
    with TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / filename).write_text(text, encoding="utf-8")
        return scan_sources([SourceRoot(owner, root)]).notes[0]


def make_scan(*notes) -> ScanResult:
    return ScanResult(tuple(notes), (), (), ())


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


class ReconciliationTests(unittest.TestCase):
    def test_reconcile_collapses_normalized_exact_duplicates(self) -> None:
        first = make_note("claude", "safety.md", "# Safety\n\nNo bus writes.")
        second = make_note(
            "codex", "safety-copy.md", "# SAFETY\n\nNo   bus writes."
        )

        result = reconcile(make_scan(first, second))

        self.assertEqual(len(result.entries), 1)
        self.assertEqual(len(result.entries[0].notes), 2)
        self.assertFalse(result.entries[0].conflict)
        self.assertEqual(result.duplicate_groups, 1)

    def test_reconcile_preserves_same_topic_conflict(self) -> None:
        first = make_note("claude", "gateway.md", "# Gateway\n\nUse address A.")
        second = make_note("codex", "gateway.md", "# Gateway\n\nUse address B.")

        result = reconcile(make_scan(first, second))

        self.assertEqual(len(result.entries[0].notes), 2)
        self.assertTrue(result.entries[0].conflict)
        self.assertEqual(result.conflicts, ("gateway",))
        self.assertIn("CONFLICT", render_index(result))

    def test_secret_like_summary_is_skipped_without_value_leak(self) -> None:
        secret = make_note(
            "hermes", "credentials.md", "# Login\n\nOPENAI_API_KEY=sk-example-value"
        )

        result = reconcile(make_scan(secret))
        report = render_report(result)

        self.assertEqual(result.entries, ())
        self.assertIn("secret-like content", report)
        self.assertNotIn("sk-example-value", report)

    def test_private_key_marker_is_filtered(self) -> None:
        secret = make_note(
            "claude",
            "key.md",
            "# Key\n\n-----BEGIN OPENSSH PRIVATE KEY-----",
        )

        result = reconcile(make_scan(secret))

        self.assertEqual(result.entries, ())
        self.assertEqual(result.skipped[0].reason, "secret-like content")

    def test_rendering_and_manifest_are_deterministic(self) -> None:
        zeta = make_note("codex", "zeta.md", "# Zeta\n\nlast")
        alpha = make_note("claude", "alpha.md", "# Alpha\n\nfirst")
        sources = (
            SourceRoot("codex", Path("/codex")),
            SourceRoot("claude", Path("/claude")),
        )
        generated_at = datetime(2026, 9, 22, tzinfo=timezone.utc)

        first = reconcile(make_scan(zeta, alpha))
        second = reconcile(make_scan(alpha, zeta))
        first_index = render_index(first)
        second_index = render_index(second)
        first_report = render_report(first)

        self.assertEqual(first_index, second_index)
        self.assertLess(first_index.index("## Alpha"), first_index.index("## Zeta"))
        manifest = build_manifest(first, sources, generated_at)
        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual(len(manifest["output_sha256"]), 64)
        self.assertEqual(
            manifest["output_sha256"],
            __import__("hashlib")
            .sha256((first_index + "\0" + first_report).encode())
            .hexdigest(),
        )


if __name__ == "__main__":
    unittest.main()
