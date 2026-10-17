"""Regression tests for the local-machine-detail guard (paths, host name, kernel release)."""

import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("check_local_info", ROOT / "tools/check_local_info.py")
guard = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = guard
SPEC.loader.exec_module(guard)


def kinds(text: str, rules=guard.GENERIC_RULES) -> list[str]:
    return [finding.kind for finding in guard.scan_text("x", text, rules)]


class GenericRuleTests(unittest.TestCase):
    def test_mount_points_home_directories_and_agent_profiles_are_refused(self):
        self.assertEqual(kinds("worktree /mnt/disk-7/src/KNXBench-x"), ["local mount path"])  # local-info: allow
        self.assertEqual(kinds('"path": "/media/stick/backup.knxproj"'), ["local mount path"])  # local-info: allow
        self.assertEqual(kinds("moved to /mnt/disk-7 today"), ["local mount path"])  # local-info: allow
        self.assertEqual(kinds("see `/home/jdoe/projects/villa.knxproj`"), ["home directory"])  # local-info: allow
        self.assertEqual(kinds("evidence in ~/.hermes/profiles/demo/x"), ["agent profile path"])  # local-info: allow

    def test_placeholders_and_relative_paths_pass(self):
        for text in ("/home/alice/projects/villa.knxproj", "/home/alice-old is over-redacted",
                     "/home/alicea/secret.knxproj", "/home/someone/.codex/x", "/mnt/private/x",
                     "ETS writes /mnt/system/TEMP/ inside the archive", "story/media/legal/x.md",
                     "<sources>/KNXBench-feature", "<profile>/evidence/run", "~/projects"):
            self.assertEqual(kinds(text), [], text)

    def test_dash_joined_mount_keys_and_arch_kernel_releases_are_refused(self):
        self.assertEqual(kinds("~/.claude/projects/-mnt-disk-7-src-KNXBench/memory"), ["local mount path"])  # local-info: allow
        self.assertEqual(kinds("measured on `Linux 6.1.2-arch1-1 x86_64`"), ["kernel release"])  # local-info: allow
        for text in ("pre-mnt-x", "--mnt-a-b", "version 1.2.3-arch", "1.2.3-arch1-1.tar"):
            self.assertEqual(kinds(text), [], text)

    def test_an_explicit_marker_exempts_exactly_its_own_line(self):
        marked = "path /mnt/disk-7/src  # local-info: allow\nnext /mnt/disk-7/src"  # local-info: allow
        self.assertEqual([(f.line, f.kind) for f in guard.scan_text("x", marked, guard.GENERIC_RULES)],
                         [(2, "local mount path")])

    def test_a_placeholder_prefix_does_not_hide_a_real_name(self):
        self.assertEqual(kinds("/home/jdoe/projects"), ["home directory"])  # local-info: allow
        self.assertEqual(kinds("/home/alicex/projects"), ["home directory"])  # local-info: allow


class MachineRuleTests(unittest.TestCase):
    FILES = {"/proc/cpuinfo": "processor : 0\nmodel name : AMD Ryzen 9 1234X 12-Core Processor\n",
             "/etc/os-release": 'NAME="ExampleOS"\nID=exampleos\n'}

    def rules(self, root=Path("/srv/build/KNXBench"), deny="", files=None, deny_file=()):
        files = self.FILES if files is None else files
        with mock.patch.object(guard.socket, "gethostname", return_value="workbench-07.lan"), \
             mock.patch.object(guard.platform, "release", return_value="6.9.1-2-custom"), \
             mock.patch.object(guard.Path, "home", return_value=Path("/var/lib/dev")), \
             mock.patch.object(guard, "read_optional", side_effect=lambda path: files.get(path, "")), \
             mock.patch.object(guard, "deny_file_tokens", return_value=list(deny_file)), \
             mock.patch.dict(os.environ, {"KNXBENCH_LOCAL_INFO_DENY": deny}):
            return guard.machine_rules(root)

    def test_the_running_machine_identifies_itself_without_being_written_down(self):
        rules = self.rules()
        self.assertEqual(kinds("built on workbench-07 today", rules), ["host name"])
        self.assertEqual(kinds("Linux 6.9.1-2-custom x86_64", rules), ["kernel release"])
        self.assertEqual(kinds("cache in /var/lib/dev/.cache", rules), ["home directory"])
        self.assertEqual(kinds("worktree /srv/build/KNXBench-other", rules), ["checkout location"])
        self.assertEqual(kinds("workbench-070 and 6.9.1-2-customer are other words", rules), [])
        self.assertEqual(kinds("key -srv-build-KNXBench", rules), ["checkout location"])
        self.assertEqual(kinds("measured on a Ryzen 9 1234X", rules), ["CPU model"])
        self.assertEqual(kinds("the 1234X\nwas busy", rules), ["CPU model"])
        self.assertEqual(kinds("one ExampleOS host and exampleos tools", rules), ["distribution"])
        self.assertEqual(kinds("on EXAMPLEOS", rules), ["distribution"])

    def test_cpu_model_tokens_drop_vendor_suffix_and_never_use_a_bare_number(self):
        self.assertEqual(guard.cpu_tokens("model name\t: AMD Ryzen 9 1234X 12-Core Processor\n"),
                         ["AMD Ryzen 9 1234X", "Ryzen 9 1234X", "1234X"])
        self.assertEqual(guard.cpu_tokens("model name : Intel(R) Core(TM) i9-0000 CPU @ 3.20GHz\n"),
                         ["Intel(R) Core(TM) i9-0000", "Core(TM) i9-0000", "i9-0000"])
        self.assertEqual(guard.cpu_tokens("model name : AMD EPYC 9999 64-Core Processor\n"),
                         ["AMD EPYC 9999", "EPYC 9999"])
        self.assertEqual(guard.cpu_tokens("no model here"), [])

    def test_common_distributions_are_not_machine_details(self):
        self.assertEqual(guard.distribution_tokens('NAME="Ubuntu"\nID=ubuntu\n'), [])
        self.assertEqual(guard.distribution_tokens('NAME="ExampleOS"\nID=exampleos\n'), ["exampleos", "ExampleOS"])
        rules = self.rules(files={"/etc/os-release": 'NAME="Arch Linux"\nID=arch\n'})
        self.assertEqual(kinds("runs on Arch Linux", rules), [])

    def test_the_clone_local_deny_file_adds_tokens_without_committing_them(self):
        rules = self.rules(deny_file=["198.18.0.9", "old-disk-x"])
        self.assertEqual(kinds("gateway 198.18.0.9 behind old-disk-x", rules), ["denied token"])
        self.assertEqual(kinds("behind old-disk-x", rules), ["denied token"])
        self.assertEqual(kinds("gateway 198.18.0.90", rules), [])

    def test_extra_tokens_come_from_the_environment_only(self):
        rules = self.rules(deny="/old/disk:x")
        self.assertEqual(kinds("moved from /old/disk/src", rules), ["denied token"])
        self.assertEqual(kinds("x marks the spot", rules), [], "tokens shorter than four characters are ignored")

    def test_generic_directories_are_never_machine_tokens(self):
        rules = self.rules(root=Path("/tmp/KNXBench"))
        self.assertEqual(kinds("written to /tmp/out", rules), [])


class RepositoryTests(unittest.TestCase):
    def test_deny_file_lives_in_the_git_common_dir_and_skips_comments(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            self.assertEqual(guard.deny_file_tokens(root), [])
            (root / ".git/info").mkdir(exist_ok=True)
            (root / ".git/info/local-info-deny").write_text("# comment\n\n198.18.0.9\n  old-disk-x  \n")
            self.assertEqual(guard.deny_file_tokens(root), ["198.18.0.9", "old-disk-x"])
        self.assertEqual(guard.deny_file_tokens(Path("/")), [])

    def test_tracked_files_and_commit_message_mode(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            git = lambda *args: subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init", "-q")
            (root / "clean.md").write_text("Built in <sources>/KNXBench.\n")
            (root / "leak.md").write_text("ok\nBuilt in /mnt/disk-9/src/KNXBench.\n")  # local-info: allow
            (root / "blob.bin").write_bytes(b"\0/mnt/disk-9/src/")
            (root / "untracked.md").write_text("/mnt/disk-9/src/\n")  # local-info: allow
            git("add", "clean.md", "leak.md", "blob.bin")
            (root / ".gitignore").write_text("ignored.md\n")
            (root / "ignored.md").write_text("/mnt/disk-9/src/\n")  # local-info: allow
            findings = guard.check(root, list(guard.GENERIC_RULES))
            self.assertEqual(sorted((f.where, f.line, f.kind) for f in findings),
                             [("leak.md", 2, "local mount path"), ("untracked.md", 1, "local mount path")])
            git("-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-qm",
                "Move to /mnt/disk-9/src\n\nbody")  # local-info: allow
            messages = guard.commit_messages(root, "HEAD")
            self.assertEqual(len(messages), 1)
            self.assertEqual(kinds(messages[0][1]), ["local mount path"])

    def test_this_repository_carries_no_local_machine_details(self):
        findings = guard.check(ROOT)
        self.assertEqual([str(finding) for finding in findings], [])

    def test_documentation_gate_and_ci_run_the_guard(self):
        self.assertIn("check_local_info", (ROOT / "tools/check_documentation.py").read_text())
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("python3 -m unittest discover -s tools/tests -p test_check_local_info.py", workflow)


if __name__ == "__main__":
    unittest.main()
