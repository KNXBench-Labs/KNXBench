"""Check the AppImage workflow against real installed and missing native bindings."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
WEB = ROOT / "apps/knx-web"
WORKFLOW = ROOT / ".github/workflows/linux-appimage.yml"
CHECK_STEP = "Check frontend native binding"


class AppImageFrontendBindingTests(unittest.TestCase):
    def setUp(self):
        self.workflow = WORKFLOW.read_text(encoding="utf-8")
        self.build, self.release = self.workflow.split("\n  release:", 1)

    def check_step(self):
        prefix = f"      - name: {CHECK_STEP}\n"
        self.assertEqual(self.build.count(prefix), 1, "missing post-install binding check")
        return self.build.split(prefix, 1)[1].split("\n      - ", 1)[0]

    def run_check(self, directory):
        step = self.check_step()
        self.assertIn("working-directory: apps/knx-web", step)
        command = step.split("        run: ", 1)[1].strip()
        self.assertNotIn("\n", command, "update extraction for a multiline workflow command")
        env = dict(os.environ)
        for name in ("NODE_OPTIONS", "NODE_PATH", "NAPI_RS_NATIVE_LIBRARY_PATH", "NAPI_RS_FORCE_WASI"):
            env.pop(name, None)
        return subprocess.run(
            ["/bin/sh", "-eu", "-c", command], cwd=directory, env=env,
            capture_output=True, text=True, timeout=30,
        )

    def test_check_is_unconditional_between_install_and_build(self):
        step = self.check_step()
        self.assertNotIn("if:", step)
        self.assertNotIn("continue-on-error", step)
        self.assertLess(self.build.index("name: Install frontend dependencies"), self.build.index(f"name: {CHECK_STEP}"))
        self.assertLess(self.build.index(f"name: {CHECK_STEP}"), self.build.index("      - name: Build AppImage\n"))

    def test_installed_native_binding_loads(self):
        result = self.run_check(WEB)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Rolldown native binding loaded", result.stdout)

    def isolated_dependencies(self, directory, *, include_native):
        # Copy real loader code away from the checkout so resolution cannot find
        # the working binding in an ancestor or follow a symlink back to it.
        modules = directory / "node_modules"
        for package in ("rolldown", "@rolldown/pluginutils", "@oxc-project/types"):
            source = WEB / "node_modules" / package
            self.assertTrue(source.is_dir(), "run npm ci --prefix apps/knx-web first")
            shutil.copytree(source, modules / package)
        self.assertFalse(list(modules.rglob("*.node")))
        binaries = []
        if include_native:
            manifest = json.loads((modules / "rolldown/package.json").read_text())
            for package in manifest["optionalDependencies"]:
                source = WEB / "node_modules" / package
                if not source.is_dir():
                    continue
                target = modules / package
                shutil.copytree(source, target)
                binaries.extend(target.rglob("*.node"))
            self.assertTrue(binaries, "no installed native binding to copy")
        return binaries

    def test_missing_native_binding_refuses_the_workflow_check(self):
        with tempfile.TemporaryDirectory(prefix="binding-missing-") as temporary:
            directory = Path(temporary)
            self.isolated_dependencies(directory, include_native=False)
            result = self.run_check(directory)
        self.assertNotEqual(result.returncode, 0, "missing native binding was accepted")
        self.assertIn("Cannot find native binding", result.stderr)
        self.assertNotIn("Rolldown native binding loaded", result.stdout)

    def test_corrupt_native_binding_refuses_the_workflow_check(self):
        with tempfile.TemporaryDirectory(prefix="binding-corrupt-") as temporary:
            directory = Path(temporary)
            binaries = self.isolated_dependencies(directory, include_native=True)
            intact = self.run_check(directory)
            self.assertEqual(intact.returncode, 0, intact.stderr)
            self.assertIn("Rolldown native binding loaded", intact.stdout)
            for binary in binaries:
                binary.write_bytes(b"not a native library\n")
            result = self.run_check(directory)
            self.assertTrue(
                any(str(binary.resolve()) in result.stderr for binary in binaries),
                "refusal did not name a copied native library",
            )
        self.assertNotEqual(result.returncode, 0, "corrupt native binding was accepted")
        self.assertIn("Cannot find native binding", result.stderr)
        self.assertNotIn("Rolldown native binding loaded", result.stdout)

    def test_manual_builds_still_cannot_publish(self):
        self.assertIn("  workflow_dispatch:", self.workflow)
        self.assertIn("    needs: build\n", self.release)
        self.assertIn("    if: github.event_name == 'push' && github.ref_type == 'tag'\n", self.release)


if __name__ == "__main__":
    unittest.main()
