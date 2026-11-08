"""Verify the release glibc ceiling counts real undefined symbols in every packaged ELF."""

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("check_appimage_glibc", ROOT / "tools/check_appimage_glibc.py")
assert spec is not None and spec.loader is not None
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class GlibcSymbolTests(unittest.TestCase):
    def test_versions_are_compared_numerically(self):
        self.assertGreater(checker.version_tuple("2.10"), checker.version_tuple("2.9"))
        self.assertLess(checker.version_tuple("2.3.4"), checker.version_tuple("2.39"))
        for value in ("", "GLIBC_2.39", "2", "2.39junk", "-2.39"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                checker.version_tuple(value)

    def test_only_required_symbols_count_including_weak_symbols(self):
        text = """
  1: 0000000000000000 0 FUNC GLOBAL DEFAULT UND free_sized@GLIBC_2.43 (2)
  2: 0000000000000000 0 FUNC WEAK DEFAULT UND pidfd_spawnp@GLIBC_2.39 (3)
  3: 0000000000001234 8 FUNC GLOBAL DEFAULT 12 defined@@GLIBC_99.0
  4: 0000000000000000 0 FUNC GLOBAL DEFAULT UND GLIBC_PRIVATE
  5: 0000000000000000 0 FUNC GLOBAL DEFAULT UND foreign@OTHER_9.9 (4)
  6: 0000000000000000 0 FUNC GLOBAL DEFAULT UND free_sized@GLIBC_2.43 (2)
"""
        self.assertEqual(checker.required_symbols(text), [("2.39", "pidfd_spawnp"), ("2.43", "free_sized")])

    def test_version_needs_are_not_confused_with_definitions(self):
        output = """
Version definition section '.gnu.version_d' contains 1 entry:
  0x001c: Rev: 1 Flags: none Index: 2 Cnt: 1 Name: GLIBC_99.0
Version needs section '.gnu.version_r' contains 2 entries:
  0x0010: Name: GLIBC_2.43 Flags: none Version: 3
  0x0020: Name: GLIBC_2.3.4 Flags: none Version: 4
Version symbols section '.gnu.version' contains 1 entry:
  Name: GLIBC_88.0
"""
        self.assertEqual(checker.required_versions(output), {"2.43", "2.3.4"})

    def test_real_elf_definition_does_not_inflate_required_version(self):
        with tempfile.TemporaryDirectory(prefix="appimage-glibc-test-") as directory:
            root = Path(directory)
            (root / "lib.c").write_text("int example(void) { return 42; }\n")
            (root / "versions.map").write_text("GLIBC_99.0 { global: example; };\n")
            library = root / "libexample.so"
            subprocess.run(
                ["cc", "-shared", "-fPIC", str(root / "lib.c"),
                 f"-Wl,--version-script={root / 'versions.map'}", "-o", str(library)],
                capture_output=True, text=True, check=True,
            )
            requirements = checker.inspect_elf(library)
            self.assertIsNotNone(requirements)
            self.assertNotIn("99.0", [version for version, _ in requirements])
            # Compiler startup files may add weak __cxa_finalize; it is a real need.
            self.assertTrue(all(checker.version_tuple(version) < (99, 0) for version, _ in requirements))

    def test_non_elf_is_ignored_but_corrupt_elf_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="appimage-glibc-test-") as directory:
            path = Path(directory) / "data"
            path.write_bytes(b"not ELF")
            self.assertIsNone(checker.inspect_elf(path))
            path.write_bytes(b"\x7fELFbroken")
            with self.assertRaisesRegex(ValueError, "cannot inspect ELF"):
                checker.inspect_elf(path)


class GlibcInventoryTests(unittest.TestCase):
    def fixture(self, directory):
        root = Path(directory)
        appdir = root / "AppDir"
        lib = appdir / "usr/lib"
        lib.mkdir(parents=True)
        (lib / "old.so").write_bytes(b"old")
        (lib / "new.so").write_bytes(b"new")
        (lib / "new-link.so").symlink_to("new.so")
        (appdir / "resource.txt").write_text("not ELF")
        runtime = root / "image.AppImage"
        runtime.write_bytes(b"runtime")
        return appdir, runtime

    def test_newest_requirement_in_a_library_is_rejected_and_link_is_not_double_counted(self):
        with tempfile.TemporaryDirectory(prefix="appimage-glibc-test-") as directory:
            appdir, runtime = self.fixture(directory)
            def inspect(path):
                return {"image.AppImage": [], "old.so": [("2.3.4", "old")],
                        "new.so": [("2.43", "free_sized")], "resource.txt": None}[path.name]
            with patch.object(checker, "inspect_elf", side_effect=inspect) as inspect_mock:
                report = checker.inspect_appdir(appdir, runtime, "2.39")
            self.assertEqual(report["elfFiles"], 3)
            self.assertEqual(report["maximumRequired"], "2.43")
            self.assertEqual(report["violations"], [{"path": "usr/lib/new.so", "symbols": [{"version": "2.43", "symbol": "free_sized"}]}])
            self.assertNotIn("new-link.so", [call.args[0].name for call in inspect_mock.call_args_list])

    def test_runtime_is_checked_and_exact_baseline_is_accepted(self):
        with tempfile.TemporaryDirectory(prefix="appimage-glibc-test-") as directory:
            appdir, runtime = self.fixture(directory)
            with patch.object(checker, "inspect_elf", return_value=[("2.39", "boundary")]):
                self.assertEqual(checker.inspect_appdir(appdir, runtime, "2.39")["violations"], [])
            def inspect(path):
                return [("2.43", "runtime_symbol")] if path == runtime else []
            with patch.object(checker, "inspect_elf", side_effect=inspect):
                report = checker.inspect_appdir(appdir, runtime, "2.39")
            self.assertEqual([v["path"] for v in report["violations"]], ["AppImage runtime"])

    def test_missing_payload_and_non_elf_runtime_are_refused(self):
        with tempfile.TemporaryDirectory(prefix="appimage-glibc-test-") as directory:
            appdir, runtime = self.fixture(directory)
            with patch.object(checker, "inspect_elf", side_effect=lambda p: [] if p == runtime else None):
                with self.assertRaisesRegex(ValueError, "ELF payload"):
                    checker.inspect_appdir(appdir, runtime, "2.39")
            with patch.object(checker, "inspect_elf", return_value=None):
                with self.assertRaisesRegex(ValueError, "runtime is not ELF"):
                    checker.inspect_appdir(appdir, runtime, "2.39")
            with self.assertRaisesRegex(ValueError, "missing or symlinked"):
                checker.inspect_appdir(appdir / "missing", runtime, "2.39")

    def test_invalid_ceiling_is_refused_before_extraction(self):
        with patch.object(checker.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                checker.check_image(Path("missing.AppImage"), "invalid")
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
