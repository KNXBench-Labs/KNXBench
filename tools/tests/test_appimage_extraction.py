"""Check permission modes read from real SquashFS payloads (synthetic AppImage-like fixtures)."""

import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CHECK = ROOT / "tools/appimage/check-permissions.sh"
OFFSET = 1024  # the wrapper script is padded to this size; the SquashFS starts right after it
TOOLS = shutil.which("mksquashfs") and shutil.which("unsquashfs")

# Not a real AppImage: a script that answers the two runtime options the tools use, followed by a
# genuine SquashFS (mksquashfs) at the reported offset. `--appimage-extract` imitates the
# type2-runtime behaviour found in alpha.19: it creates every directory 0700.
WRAPPER = """#!/bin/sh
case "$1" in
    --appimage-offset) echo "{offset_output}" ;;
    --appimage-extract)
        unsquashfs -no-progress -o {offset} -d squashfs-root "$0" >/dev/null
        find squashfs-root -type d -exec chmod 700 {{}} +
        ;;
esac
exit 0
"""


def build_fixture(directory, tree_edit=None, offset_output=None):
    """Return a synthetic image whose raw SquashFS directories are 0755 unless edited."""
    directory = Path(directory)
    tree = directory / "tree"
    (tree / "usr/lib").mkdir(parents=True)
    (tree / "AppRun").write_text('#!/bin/sh\nexec "$APPDIR/AppRun.wrapped"\n')
    (tree / "AppRun.wrapped").write_text("#!/bin/sh\n")
    (tree / "usr/lib/libx.so").write_bytes(b"elf")
    for path in tree.rglob("*"):
        path.chmod(0o755 if path.is_dir() or path.name.startswith("AppRun") else 0o644)
    tree.chmod(0o755)
    if tree_edit:
        tree_edit(tree)
    payload = directory / "payload.squashfs"
    subprocess.run(["mksquashfs", str(tree), str(payload), "-processors", "1", "-noappend",
                    "-no-progress", "-all-root"], check=True, capture_output=True)
    image = directory / "fixture.AppImage"
    shown = str(OFFSET) if offset_output is None else offset_output
    script = WRAPPER.format(offset=OFFSET, offset_output=shown).encode()
    assert len(script) <= OFFSET
    image.write_bytes(script.ljust(OFFSET, b"\n") + payload.read_bytes())
    image.chmod(0o755)
    return image


def with_path(env_path):
    return dict(os.environ, PATH=env_path)


@unittest.skipUnless(TOOLS, "needs mksquashfs and unsquashfs (squashfs-tools) on PATH")
class RawModeCheckTests(unittest.TestCase):
    def check(self, image):
        return subprocess.run([str(CHECK), str(image)], capture_output=True, text=True)

    def fixture(self, **kwargs):
        directory = tempfile.mkdtemp(prefix="appimage-extract-")
        self.addCleanup(shutil.rmtree, directory, True)
        return build_fixture(directory, **kwargs)

    def test_runtime_extraction_makes_directories_0700_but_the_raw_image_is_fine(self):
        image = self.fixture()
        with tempfile.TemporaryDirectory(prefix="appimage-extract-") as scratch:
            subprocess.run([str(image), "--appimage-extract"], cwd=scratch, check=True, env=with_path(os.environ["PATH"]))
            modes = {p.stat().st_mode & 0o777 for p in Path(scratch, "squashfs-root").rglob("*") if p.is_dir()}
            self.assertEqual(modes, {0o700})  # what the old method would have judged
        result = self.check(image)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_raw_0700_directory_is_rejected(self):
        image = self.fixture(tree_edit=lambda tree: (tree / "usr/lib").chmod(0o700))
        result = self.check(image)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("directories not readable", result.stderr)

    def test_raw_0770_wrapped_launcher_is_rejected_by_name(self):
        image = self.fixture(tree_edit=lambda tree: (tree / "AppRun.wrapped").chmod(0o770))
        result = self.check(image)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("AppRun.wrapped is not readable and executable", result.stderr)

    def test_all_defects_are_reported_together(self):
        def edit(tree):
            (tree / "AppRun.wrapped").chmod(0o770)
            (tree / "usr/lib/libx.so").chmod(0o777)
            (tree / "usr").chmod(0o777)
        result = self.check(self.fixture(tree_edit=edit))
        self.assertNotEqual(result.returncode, 0)
        for text in ("AppRun.wrapped is not readable", "world-writable entries"):
            self.assertIn(text, result.stderr)

    def test_missing_or_malformed_offset_fails_closed(self):
        for shown in ("", "abc", "0", "-5", "12 34", "0123", "1.5"):
            with self.subTest(offset=shown):
                result = self.check(self.fixture(offset_output=shown))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("--appimage-offset", result.stderr)

    def test_a_restrictive_caller_umask_does_not_hide_world_writable_files(self):
        image = self.fixture(tree_edit=lambda tree: (tree / "usr/lib/libx.so").chmod(0o777))
        result = subprocess.run(["/bin/sh", "-c", 'umask 077; exec "$0" "$1"', str(CHECK), str(image)],
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("world-writable entries", result.stderr)

    def test_missing_unsquashfs_fails_closed(self):
        image = self.fixture()
        stubs = tempfile.mkdtemp(prefix="appimage-extract-")
        self.addCleanup(shutil.rmtree, stubs, True)
        for name in ("sh", "readlink", "mktemp", "rm", "chmod", "find", "grep"):
            shutil.copy(shutil.which(name), stubs)
        result = subprocess.run(["/bin/sh", str(CHECK), str(image)], env=with_path(stubs), capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsquashfs", result.stderr)


@unittest.skipUnless(TOOLS, "needs mksquashfs and unsquashfs (squashfs-tools) on PATH")
class SmokeExtractionTests(unittest.TestCase):
    def load(self):
        spec = importlib.util.spec_from_file_location("smoke_cross_user", ROOT / "tools/appimage/smoke_cross_user.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_direct_unsquashfs_keeps_raw_modes_and_foreign_ownership(self):
        smoke = self.load()
        with tempfile.TemporaryDirectory(prefix="appimage-extract-") as directory:
            image = build_fixture(directory, tree_edit=lambda tree: (tree / "AppRun.wrapped").chmod(0o770))
            work = Path(directory) / "work"
            work.mkdir()
            root = smoke.extract_image(image, work)
            self.assertEqual((root / "usr/lib").stat().st_mode & 0o777, 0o755)
            self.assertEqual((root / "AppRun.wrapped").stat().st_mode & 0o777, 0o770)
            found = smoke.assert_foreign_owned(root, smoke.TARGET_UID)
            self.assertEqual(found["AppRun.wrapped"]["mode"], "0o770")

    def test_malformed_offset_is_refused_before_any_extraction(self):
        smoke = self.load()
        for shown in ("", "abc", "0", "-1", "7 8"):
            with self.subTest(offset=shown), tempfile.TemporaryDirectory(prefix="appimage-extract-") as directory:
                image = build_fixture(directory, offset_output=shown)
                work = Path(directory) / "work"
                work.mkdir()
                with self.assertRaisesRegex(ValueError, "appimage-offset"):
                    smoke.extract_image(image, work)
                self.assertFalse((work / "extract/squashfs-root").exists())


if __name__ == "__main__":
    unittest.main()
