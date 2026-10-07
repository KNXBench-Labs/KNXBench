"""Regression tests for KNXBench's AppImage display policy and build hook."""

import hashlib
import importlib.util
import json
from pathlib import Path
import os
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "tools/appimage/display-backend.sh"


class DisplayPolicyTests(unittest.TestCase):
    def run_policy(self, **variables):
        self.assertTrue(POLICY.is_file(), "AppImage has no owned display policy yet")
        env = dict(os.environ)
        for name in ("GDK_BACKEND", "WAYLAND_DISPLAY", "DISPLAY", "XDG_SESSION_TYPE", "WEBKIT_DISABLE_DMABUF_RENDERER"):
            env.pop(name, None)
        env.update(variables)
        result = subprocess.run(
            ["/bin/sh", "-eu", "-c",
             '. "$1"; printf "%s\\n%s\\n" "$GDK_BACKEND" "${WEBKIT_DISABLE_DMABUF_RENDERER-unset}"',
             "test-policy", str(POLICY)],
            env=env, capture_output=True, text=True, check=True,
        )
        return result.stdout.splitlines()

    def test_wayland_session_prefers_native_wayland_with_x11_fallback(self):
        self.assertEqual(
            self.run_policy(WAYLAND_DISPLAY="wayland-1", DISPLAY=":0"),
            ["wayland,x11", "1"],
        )


    def test_wayland_session_type_alone_is_a_backend_hint(self):
        self.assertEqual(self.run_policy(XDG_SESSION_TYPE="wayland"), ["wayland,x11", "1"])

    def test_wayland_without_x_server_uses_the_same_policy(self):
        self.assertEqual(self.run_policy(WAYLAND_DISPLAY="wayland-1"), ["wayland,x11", "1"])

    def test_x11_session_retains_the_previous_renderer(self):
        self.assertEqual(self.run_policy(DISPLAY=":42"), ["x11", "unset"])

    def test_no_display_does_not_invent_a_wayland_session(self):
        self.assertEqual(self.run_policy(), ["x11", "unset"])

    def test_empty_backend_is_treated_as_automatic_selection(self):
        self.assertEqual(self.run_policy(GDK_BACKEND="", WAYLAND_DISPLAY="wayland-1"), ["wayland,x11", "1"])

    def test_explicit_backends_are_preserved(self):
        for backend, renderer in (("x11", "unset"), ("wayland", "1"),
                                  ("x11,wayland", "1"), ("*", "1"), ("broadway", "unset")):
            with self.subTest(backend=backend):
                self.assertEqual(
                    self.run_policy(GDK_BACKEND=backend, WAYLAND_DISPLAY="wayland-1"),
                    [backend, renderer],
                )

    def test_explicit_renderer_settings_are_preserved(self):
        for value in ("0", "1", ""):
            with self.subTest(value=value):
                self.assertEqual(
                    self.run_policy(WAYLAND_DISPLAY="wayland-1", WEBKIT_DISABLE_DMABUF_RENDERER=value),
                    ["wayland,x11", value],
                )

    def test_policy_can_be_sourced_twice(self):
        self.assertTrue(POLICY.is_file())
        env = dict(os.environ, GDK_BACKEND="wayland", WEBKIT_DISABLE_DMABUF_RENDERER="0")
        result = subprocess.run(
            ["/bin/sh", "-eu", "-c", '. "$1"; . "$1"; printf "%s %s" "$GDK_BACKEND" "$WEBKIT_DISABLE_DMABUF_RENDERER"',
             "test-policy", str(POLICY)], env=env, capture_output=True, text=True, check=True,
        )
        self.assertEqual(result.stdout, "wayland 0")


class PluginPreparationTests(unittest.TestCase):
    def load_preparer(self):
        path = ROOT / "tools/prepare_appimage_tools.py"
        self.assertTrue(path.is_file(), "no reproducible GTK plugin preparation exists")
        spec = importlib.util.spec_from_file_location("prepare_appimage_tools", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_missing_or_duplicate_assignments_are_refused(self):
        preparer = self.load_preparer()
        for source in ("no assignment\n", "export GDK_BACKEND=x11\n" * 2):
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, "exactly one"):
                preparer.patch_plugin(source, POLICY.read_text())

    def test_empty_policy_or_heredoc_terminator_is_refused(self):
        preparer = self.load_preparer()
        source = 'cat > "$HOOKFILE" <<\\EOF\nexport GDK_BACKEND=x11\nEOF\n'
        for policy in ("", "\n", "echo before\nEOF\necho after\n"):
            with self.subTest(policy=policy), self.assertRaisesRegex(ValueError, "policy"):
                preparer.patch_plugin(source, policy)

    def test_cached_plugin_is_verified_and_preparation_is_idempotent(self):
        preparer = self.load_preparer()
        source = b'cat > "$HOOKFILE" <<\\EOF\nexport GDK_BACKEND=x11\nEOF\n'
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            tools = Path(directory)
            (tools / "knxbench-gtk-upstream.sh").write_bytes(source)
            with patch.object(preparer, "UPSTREAM_SHA256", hashlib.sha256(source).hexdigest()), \
                    patch.object(preparer.urllib.request, "urlopen", side_effect=AssertionError("cached build must be offline")):
                plugin = preparer.prepare(tools)
                first = plugin.read_bytes()
                self.assertEqual(preparer.prepare(tools).read_bytes(), first)
            self.assertEqual(plugin.stat().st_mode & 0o777, 0o700)
            self.assertIn(POLICY.read_bytes(), first)
            self.assertNotIn(b"export GDK_BACKEND=x11\nEOF", first)
            self.assertEqual((tools / "knxbench-gtk-upstream.sh").read_bytes(), source)
            self.assertEqual(sorted(path.name for path in tools.iterdir()),
                             ["knxbench-gtk-upstream.sh", "linuxdeploy-plugin-gtk.sh"])

    def test_corrupt_cache_is_refused_without_overwriting_the_plugin(self):
        preparer = self.load_preparer()
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            tools = Path(directory)
            (tools / "knxbench-gtk-upstream.sh").write_bytes(b"corrupt")
            plugin = tools / "linuxdeploy-plugin-gtk.sh"
            plugin.write_bytes(b"previous good plugin")
            with patch.object(preparer.urllib.request, "urlopen", side_effect=AssertionError("no network repair")), \
                    self.assertRaisesRegex(ValueError, "checksum mismatch"):
                preparer.prepare(tools)
            self.assertEqual(plugin.read_bytes(), b"previous good plugin")
            self.assertEqual((tools / "knxbench-gtk-upstream.sh").read_bytes(), b"corrupt")

    def test_download_is_bounded_and_verified_before_any_plugin_write(self):
        preparer = self.load_preparer()
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            tools = Path(directory)
            from unittest.mock import MagicMock
            response = MagicMock()
            response.__enter__.return_value = response
            response.read.return_value = b"unexpected upstream bytes"
            with patch.object(preparer.urllib.request, "urlopen", return_value=response) as request, \
                    self.assertRaisesRegex(ValueError, "checksum mismatch"):
                preparer.prepare(tools)
            request.assert_called_once_with(preparer.UPSTREAM_URL, timeout=30)
            response.read.assert_called_once_with(preparer.MAX_PLUGIN_BYTES + 1)
            self.assertEqual(list(tools.iterdir()), [])

    def test_main_uses_tauris_metadata_directory_not_the_callers_cwd(self):
        preparer = self.load_preparer()
        output = subprocess.CompletedProcess([], 0, stdout=json.dumps({"target_directory": "/owned/custom-target"}))
        with patch.object(preparer.subprocess, "run", return_value=output) as run, \
                patch.object(preparer, "prepare", return_value=Path("prepared")) as prepare:
            preparer.main()
        run.assert_called_once_with(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=ROOT / "apps/knx-desktop/src-tauri", capture_output=True, text=True, check=True,
        )
        prepare.assert_called_once_with(Path("/owned/custom-target/.tauri"))

    def test_assignment_outside_the_quoted_hook_is_refused(self):
        preparer = self.load_preparer()
        source = 'cat > "$HOOKFILE" <<\\EOF\nfirst hook\nEOF\nexport GDK_BACKEND=x11\nEOF\n'
        with self.assertRaisesRegex(ValueError, "quoted hook"):
            preparer.patch_plugin(source, POLICY.read_text())

    def test_only_forced_backend_line_is_replaced_inside_quoted_hook(self):
        preparer = self.load_preparer()
        upstream = 'prefix\ncat > "$HOOKFILE" <<\\EOF\nexport GDK_BACKEND=x11 # upstream rationale\nsuffix\nEOF\n'
        policy = POLICY.read_text()
        self.assertEqual(
            preparer.patch_plugin(upstream, policy),
            'prefix\ncat > "$HOOKFILE" <<\\EOF\n' + policy + 'suffix\nEOF\n',
        )


class PackagingContractTests(unittest.TestCase):
    def test_bundle_always_prepares_the_owned_hook(self):
        config = json.loads(
            (ROOT / "apps/knx-desktop/src-tauri/tauri.conf.json").read_text()
        )
        self.assertEqual(
            config["build"].get("beforeBundleCommand"),
            "python3 ../../tools/prepare_appimage_tools.py",
        )

    def test_bundler_uses_project_local_tools(self):
        config = json.loads(
            (ROOT / "apps/knx-desktop/src-tauri/tauri.conf.json").read_text()
        )
        self.assertTrue(
            config["bundle"].get("useLocalToolsDir"),
            "the owned GTK plugin must not modify the user's shared Tauri cache",
        )


if __name__ == "__main__":
    unittest.main()
