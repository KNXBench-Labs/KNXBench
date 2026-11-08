"""Regression tests for KNXBench's AppImage display policy and build hook."""

import hashlib
import importlib.util
import json
from pathlib import Path
import os
import shlex
import shutil
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "tools/appimage/display-backend.sh"
HOST_LIBRARIES = ROOT / "tools/appimage/host-libraries.sh"
PERMISSIONS = ROOT / "tools/appimage/permissions.sh"
CHECK_PERMISSIONS = ROOT / "tools/appimage/check-permissions.sh"
WEBKIT_TAIL = "find \"$APPDIR\"/usr/lib* -name 'libwebkit*' -exec sed -i -e \"s|/usr|././|g\" '{}' \\;\n"


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
        source = b'cat > "$HOOKFILE" <<\\EOF\nexport GDK_BACKEND=x11\nEOF\n' + WEBKIT_TAIL.encode()
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
            self.assertTrue(first.endswith(PERMISSIONS.read_bytes()))
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


class HostLibraryPolicyTests(unittest.TestCase):
    """Issue #6: the host must provide libwayland-client, not the AppImage."""

    def run_policy(self, appdir):
        self.assertTrue(HOST_LIBRARIES.is_file(), "AppImage has no host-library policy yet")
        return subprocess.run(
            ["/bin/bash", "-eu", "-c", '. "$1"', "test-policy", str(HOST_LIBRARIES)],
            env=dict(os.environ, APPDIR=str(appdir)), capture_output=True, text=True, check=True,
        )

    def make_appdir(self, directory, names):
        lib = Path(directory) / "usr/lib"
        lib.mkdir(parents=True)
        for name in names:
            (lib / name).write_bytes(b"elf")
        return Path(directory)

    def test_bundled_wayland_client_is_removed_and_reported(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, ["libwayland-client.so.0", "libwebkit2gtk-4.1.so.0"])
            (appdir / "usr/lib/libwayland-client.so").symlink_to("libwayland-client.so.0")
            result = self.run_policy(appdir)
            self.assertEqual(sorted(p.name for p in (appdir / "usr/lib").iterdir()),
                             ["libwebkit2gtk-4.1.so.0"])
            self.assertIn("libwayland-client.so.0", result.stdout)

    def test_other_wayland_libraries_stay_bundled(self):
        # WebKitGTK links libwayland-server directly; only the client library
        # conflicts with the host's Mesa EGL vendor (pkg2appimage excludelist).
        kept = ["libwayland-cursor.so.0", "libwayland-egl.so.1", "libwayland-server.so.0"]
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, kept + ["libwayland-client.so.0"])
            self.run_policy(appdir)
            self.assertEqual(sorted(p.name for p in (appdir / "usr/lib").iterdir()), kept)

    def test_policy_is_idempotent_when_nothing_is_bundled(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, ["libwebkit2gtk-4.1.so.0"])
            self.run_policy(appdir)
            self.run_policy(appdir)
            self.assertEqual([p.name for p in (appdir / "usr/lib").iterdir()], ["libwebkit2gtk-4.1.so.0"])

    def test_missing_appdir_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            with self.assertRaises(subprocess.CalledProcessError):
                self.run_policy(Path(directory) / "absent")


class HostLibraryAppendTests(unittest.TestCase):
    def load_preparer(self):
        spec = importlib.util.spec_from_file_location("prepare_appimage_tools", ROOT / "tools/prepare_appimage_tools.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_policy_is_appended_after_the_final_webkit_patch(self):
        preparer = self.load_preparer()
        source = "prefix\n" + WEBKIT_TAIL
        self.assertEqual(preparer.append_host_library_policy(source, "policy\n"), source + "\npolicy\n")

    def test_changed_upstream_tail_is_refused(self):
        preparer = self.load_preparer()
        for source in ("prefix\n", WEBKIT_TAIL + "exit 0\n", WEBKIT_TAIL + WEBKIT_TAIL):
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, "tail"):
                preparer.append_host_library_policy(source, "policy\n")

    def test_empty_policy_is_refused(self):
        preparer = self.load_preparer()
        with self.assertRaisesRegex(ValueError, "policy"):
            preparer.append_host_library_policy(WEBKIT_TAIL, "\n")

    def test_pinned_upstream_plugin_accepts_both_patches(self):
        cached = os.environ.get("KNXBENCH_GTK_PLUGIN_SOURCE")
        if not cached:
            self.skipTest("set KNXBENCH_GTK_PLUGIN_SOURCE to the pinned upstream plugin to run")
        preparer = self.load_preparer()
        source = Path(cached).read_bytes()
        self.assertEqual(hashlib.sha256(source).hexdigest(), preparer.UPSTREAM_SHA256)
        patched = preparer.patch_source(source.decode("utf-8"))
        self.assertIn(HOST_LIBRARIES.read_text().rstrip("\n"), patched)
        self.assertTrue(patched.endswith(PERMISSIONS.read_text()))
        subprocess.run(["/bin/bash", "-n", "-c", patched], check=True)


class PermissionPolicyTests(unittest.TestCase):
    """The Tauri AppRun copy is mode 0770; other users must still be able to start the image."""

    def make_appdir(self, directory, launcher_mode=0o770, wrapped=True):
        appdir = Path(directory) / "AppDir"
        (appdir / "usr/lib").mkdir(parents=True)
        (appdir / "usr/share").mkdir()
        appdir.chmod(0o700)
        (appdir / "usr/lib").chmod(0o700)
        launcher = appdir / "AppRun"
        launcher.write_text("#!/bin/sh\nexec \"$0.wrapped\"\n")
        launcher.chmod(launcher_mode)
        if wrapped:
            (appdir / "AppRun.wrapped").write_text("#!/bin/sh\n")
            (appdir / "AppRun.wrapped").chmod(0o700)
        (appdir / "usr/share/data.txt").write_text("data")
        (appdir / "usr/share/data.txt").chmod(0o600)
        (appdir / "usr/lib/libx.so").write_bytes(b"elf")
        (appdir / "usr/lib/libx.so").chmod(0o770)
        return appdir

    def run_policy(self, appdir, check=True):
        return subprocess.run(
            ["/bin/bash", "-eu", "-c", '. "$1"', "test-policy", str(PERMISSIONS)],
            env=dict(os.environ, APPDIR=str(appdir)), capture_output=True, text=True, check=check,
        )

    def mode(self, path):
        return path.stat().st_mode & 0o7777

    def test_restrictive_launchers_and_directories_become_readable_and_executable(self):
        for launcher_mode in (0o770, 0o700):
            with self.subTest(mode=oct(launcher_mode)), tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
                appdir = self.make_appdir(directory, launcher_mode)
                self.run_policy(appdir)
                self.assertEqual(self.mode(appdir / "AppRun"), 0o755)
                self.assertEqual(self.mode(appdir / "AppRun.wrapped"), 0o755)
                self.assertEqual(self.mode(appdir), 0o755)
                self.assertEqual(self.mode(appdir / "usr/lib"), 0o755)

    def test_data_files_stay_non_executable_and_nothing_gains_others_write(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory)
            self.run_policy(appdir)
            self.assertEqual(self.mode(appdir / "usr/share/data.txt"), 0o644)
            # An already executable file keeps its execute bits and gains read for all.
            self.assertEqual(self.mode(appdir / "usr/lib/libx.so"), 0o755)
            for path in appdir.rglob("*"):
                self.assertEqual(self.mode(path) & 0o022, 0, path)

    def test_policy_is_idempotent_and_accepts_an_already_correct_tree(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, 0o755)
            self.run_policy(appdir)
            before = {p: self.mode(p) for p in appdir.rglob("*")}
            self.run_policy(appdir)
            self.assertEqual(before, {p: self.mode(p) for p in appdir.rglob("*")})

    def test_missing_launcher_or_appdir_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory)
            (appdir / "AppRun").unlink()
            self.assertIn("AppRun", self.run_policy(appdir, check=False).stderr)
            self.assertNotEqual(self.run_policy(appdir, check=False).returncode, 0)
            self.assertNotEqual(self.run_policy(Path(directory) / "absent", check=False).returncode, 0)

    def test_wrapped_launcher_may_be_absent_at_packing_time(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, wrapped=False)
            self.run_policy(appdir)
            self.assertEqual(self.mode(appdir / "AppRun"), 0o755)

    def test_launcher_symlink_inside_appdir_is_followed_but_outside_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory, wrapped=False)
            real = appdir / "usr/bin-real"
            real.write_text("#!/bin/sh\n")
            real.chmod(0o600)
            (appdir / "AppRun.wrapped").symlink_to("usr/bin-real")
            self.run_policy(appdir)
            self.assertEqual(self.mode(real), 0o755)

            outside = Path(directory) / "outside"
            outside.write_text("x")
            outside.chmod(0o600)
            (appdir / "AppRun.wrapped").unlink()
            (appdir / "AppRun.wrapped").symlink_to(outside)
            self.assertNotEqual(self.run_policy(appdir, check=False).returncode, 0)
            self.assertEqual(self.mode(outside), 0o600)

    def test_symlinks_to_outside_files_are_never_chmodded(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            appdir = self.make_appdir(directory)
            outside = Path(directory) / "outside"
            outside.write_text("x")
            outside.chmod(0o600)
            (appdir / "usr/share/link").symlink_to(outside)
            self.run_policy(appdir)
            self.assertEqual(self.mode(outside), 0o600)


class FinishedImagePermissionCheckTests(unittest.TestCase):
    def check(self, root):
        return subprocess.run([str(CHECK_PERMISSIONS), str(root)], capture_output=True, text=True)

    def make_root(self, directory):
        root = Path(directory) / "squashfs-root"
        (root / "usr/lib").mkdir(parents=True)
        for path in (root, root / "usr", root / "usr/lib"):
            path.chmod(0o755)
        (root / "AppRun").write_text("#!/bin/sh\nexec \"$APPDIR/AppRun.wrapped\"\n")
        (root / "AppRun").chmod(0o755)
        (root / "AppRun.wrapped").write_text("#!/bin/sh\n")
        (root / "AppRun.wrapped").chmod(0o755)
        return root

    def add_file(self, root, mode):
        (root / "usr/lib/f").write_text("x")
        (root / "usr/lib/f").chmod(mode)

    def test_correct_tree_passes(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            result = self.check(self.make_root(directory))
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_each_permission_defect_is_rejected(self):
        defects = {
            "apprun-0770": lambda r: (r / "AppRun").chmod(0o770),
            "wrapped-0700": lambda r: (r / "AppRun.wrapped").chmod(0o700),
            "wrapped-missing": lambda r: (r / "AppRun.wrapped").unlink(),
            "apprun-missing": lambda r: (r / "AppRun").unlink(),
            "private-dir": lambda r: (r / "usr/lib").chmod(0o750),
            "private-file": lambda r: self.add_file(r, 0o600),
            "world-writable": lambda r: self.add_file(r, 0o666),
            "launcher-outside": lambda r: ((r / "AppRun.wrapped").unlink(), (r / "AppRun.wrapped").symlink_to("/bin/sh")),
        }
        for name, defect in defects.items():
            with self.subTest(defect=name), tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
                root = self.make_root(directory)
                defect(root)
                self.assertNotEqual(self.check(root).returncode, 0)

    def test_not_an_extracted_image_fails_the_positive_control(self):
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            self.assertNotEqual(self.check(directory).returncode, 0)


class SmokeHelperTests(unittest.TestCase):
    def load(self):
        spec = importlib.util.spec_from_file_location("smoke_cross_user", ROOT / "tools/appimage/smoke_cross_user.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def xwd(self, pixels, width, height):
        fields = [0] * 25
        fields[0], fields[5], fields[4], fields[11], fields[12] = 100, height, width, 32, width * 4
        return struct.pack(">25I", *fields) + b"".join(pixels)

    def framed(self, interior, frame=(1, 2, 3, 0), size=20):
        """A size x size dump: a coloured one-pixel border around `interior(x, y)` pixels."""
        pixels = []
        for y in range(size):
            for x in range(size):
                edge = x in (0, size - 1) or y in (0, size - 1)
                pixels.append(bytes(frame) if edge else interior(x, y))
        return self.xwd(pixels, size, size)

    def test_blank_window_has_one_colour_and_a_painted_one_many(self):
        smoke = self.load()
        blank = self.xwd([b"\xff\xff\xff\x00"] * 100, 10, 10)
        self.assertEqual(smoke.central_colours(blank), 1)
        painted = self.xwd([bytes([i, 7, 9, 0]) for i in range(100)], 10, 10)
        self.assertGreaterEqual(smoke.central_colours(painted), smoke.MIN_COLOURS)

    def test_decorations_around_a_blank_interior_do_not_count(self):
        smoke = self.load()
        decorated = self.framed(lambda x, y: b"\xff\xff\xff\x00")
        # The border alone carries more than MIN_COLOURS in a full-frame count.
        gradient = [bytes([i, 0, 0, 0]) for i in range(20)]
        titlebar = self.xwd([gradient[x] if y in (0, 19) else b"\xff\xff\xff\x00"
                             for y in range(20) for x in range(20)], 20, 20)
        self.assertEqual(smoke.central_colours(decorated), 1)
        self.assertEqual(smoke.central_colours(titlebar), 1)
        rendered = self.framed(lambda x, y: bytes([x, y, 5, 0]))
        self.assertGreaterEqual(smoke.central_colours(rendered), smoke.MIN_COLOURS)

    def test_truncated_or_unsupported_dump_is_refused(self):
        smoke = self.load()
        with self.assertRaises(ValueError):
            smoke.central_colours(self.xwd([b"\0\0\0\0"] * 5, 10, 10))

    def test_target_uid_runner_is_refused(self):
        smoke = self.load()
        with patch.object(smoke.os, "geteuid", return_value=smoke.TARGET_UID):
            self.assertEqual(smoke.main(__file__), 2)

    GOOD_PAGE = ('<html><head><title>KNXBench</title><script type="module" crossorigin '
                 'src="/assets/index-abc.js"></script></head><body><div id="root"></div></body></html>')

    def responses(self, **changes):
        base = {"/": {"status": 200, "body": self.GOOD_PAGE},
                "/api/version": {"status": 200, "body": '{"version":"0.1.0-alpha.19+gabc"}'},
                "/assets/index-abc.js": {"status": 200, "length": 1234}}
        base.update(changes)
        return base

    def test_only_the_real_frontend_and_version_endpoint_count_as_ready(self):
        smoke = self.load()
        self.assertEqual(smoke.check_frontend(self.responses()), [])
        bad = {
            "any-200-page": {"/": {"status": 200, "body": "<html>hello</html>"}},
            "no-script": {"/": {"status": 200, "body": self.GOOD_PAGE.replace("src=", "data-x=")}},
            "script-missing": {"/assets/index-abc.js": {"status": 404, "length": 9}},
            "script-empty": {"/assets/index-abc.js": {"status": 200, "length": 0}},
            "version-html": {"/api/version": {"status": 200, "body": "<html></html>"}},
            "version-word": {"/api/version": {"status": 200, "body": '{"version":"x"}'}},
            "version-404": {"/api/version": {"status": 404, "body": '{"version":"0.1.0"}'}},
        }
        for name, change in bad.items():
            with self.subTest(case=name):
                self.assertNotEqual(smoke.check_frontend(self.responses(**change)), [])

    def test_launchers_owned_by_the_target_user_void_the_test(self):
        smoke = self.load()
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            root = Path(directory)
            (root / "AppRun").write_text("#!/bin/sh\n")
            (root / "AppRun").chmod(0o755)
            found = smoke.assert_foreign_owned(root, os.geteuid() + 1)
            self.assertEqual(found["AppRun"]["uid"], os.geteuid())
            with self.assertRaisesRegex(ValueError, "target uid"):
                smoke.assert_foreign_owned(root, os.geteuid())
            (root / "AppRun").unlink()
            with self.assertRaisesRegex(ValueError, "no AppRun"):
                smoke.assert_foreign_owned(root, os.geteuid() + 1)

    def test_extraction_precedes_the_uid_switch_and_runs_the_packaged_apprun(self):
        smoke = self.load()
        events = []
        with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
            source = Path(directory) / "KNXBench.AppImage"
            source.write_bytes(b"image")
            fake_root = Path(directory) / "squashfs-root"
            fake_root.mkdir()
            (fake_root / "AppRun").write_text("#!/bin/sh\n")

            def record_run(argv, *args, **kwargs):
                events.append(("run", argv[:2]))
                return subprocess.CompletedProcess(argv, 0, b"", b"")

            class Gone:
                pid, returncode = 1, 1

                def poll(self):
                    return 1

                def wait(self, timeout=None):
                    return 1

                def terminate(self):
                    events.append(("terminate", None))

            def record_popen(argv, *args, **kwargs):
                events.append(("popen", argv))
                return Gone()

            def extract(src, work):
                events.append(("extract", src))
                return fake_root

            with patch.object(smoke, "extract_image", extract), \
                    patch.object(smoke, "start_xvfb", lambda env: env.update(DISPLAY=":9") or Gone()), \
                    patch.object(smoke.subprocess, "run", record_run), \
                    patch.object(smoke.subprocess, "Popen", record_popen), \
                    patch.dict(os.environ, {"APPIMAGE_SMOKE_EVIDENCE": str(Path(directory) / "evidence")}):
                self.assertEqual(smoke.main(str(source)), 1)
        self.assertEqual(events[0][0], "extract")
        launch = next(argv for kind, argv in events if kind == "popen")
        self.assertLess(events.index(("extract", source.resolve())), events.index(("popen", launch)))
        switch = [i for i, (kind, what) in enumerate(events) if kind == "run" and what[:1] == ["sudo"]]
        self.assertTrue(switch and min(switch) > 0, "no privileged step may precede extraction")
        command = launch[5]
        self.assertIn("unshare", launch)
        self.assertIn("--net", launch)
        self.assertIn("setpriv --reuid=65534", command)
        self.assertEqual(launch[-1], str(fake_root / "AppRun"))
        for forbidden in ("APPIMAGE_EXTRACT_AND_RUN", "knx-desktop", "--appimage-extract"):
            self.assertNotIn(forbidden, command + launch[-1])

    def test_launcher_shell_arguments_preserve_spaces_and_metacharacters(self):
        smoke = self.load()
        work = Path("/tmp/owned path; not-a-command")
        image = work / "image with apostrophe's.AppImage"
        root = work / "squashfs-root"
        tokens = shlex.split(smoke.launch_command(image, root, work, ":9")[5])
        self.assertIn(str(work), tokens)
        self.assertIn(f"APPIMAGE={image}", tokens)
        self.assertIn(f"APPDIR={root}", tokens)
        self.assertIn(f"HOME={work / 'home'}", tokens)

    def test_failed_xvfb_start_reaps_its_process(self):
        smoke = self.load()
        with patch.object(smoke.subprocess, "Popen") as popen, \
                patch.object(smoke.select, "select", return_value=([1], [], [])), \
                patch.object(smoke.os, "read", return_value=b""):
            with self.assertRaisesRegex(RuntimeError, "usable display"):
                smoke.start_xvfb({})
        popen.return_value.terminate.assert_called_once()
        popen.return_value.wait.assert_called_once_with(timeout=10)

    def test_restrictive_wrapped_launcher_fails_for_a_foreign_user_and_0755_starts(self):
        smoke = self.load()
        if not shutil.which("sudo"):
            self.skipTest("needs passwordless sudo, setpriv, unshare and ip")
        probe = subprocess.run(["sudo", "-n", "true"], capture_output=True)
        if probe.returncode != 0 or not shutil.which("setpriv") or not shutil.which("unshare") or not shutil.which("ip"):
            self.skipTest("needs passwordless sudo, setpriv, unshare and ip")
        outcomes = {}
        for mode in (0o770, 0o755):
            with tempfile.TemporaryDirectory(prefix="appimage-test-") as directory:
                work = Path(directory)
                work.chmod(0o755)
                root = work / "squashfs-root"
                root.mkdir(mode=0o755)
                (root / "AppRun").write_text('#!/bin/sh\nexec "$APPDIR/AppRun.wrapped"\n')
                (root / "AppRun").chmod(0o755)
                (root / "AppRun.wrapped").write_text("#!/bin/sh\necho started\n")
                (root / "AppRun.wrapped").chmod(mode)
                result = subprocess.run(smoke.launch_command(work / "x.AppImage", root, work, ":0"),
                                        capture_output=True, text=True, timeout=60)
                outcomes[mode] = result
        self.assertNotEqual(outcomes[0o770].returncode, 0)
        self.assertIn("Permission denied", outcomes[0o770].stdout + outcomes[0o770].stderr)
        self.assertEqual(outcomes[0o755].returncode, 0, outcomes[0o755].stderr)
        self.assertIn("started", outcomes[0o755].stdout)


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
