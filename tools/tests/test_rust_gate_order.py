"""Keep knx-desktop inside every documented Rust workspace gate."""

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
VERIFICATION = ROOT / "docs/VERIFICATION.md"


def step_index(workflow: str, name: str) -> int:
    prefix = f"      - name: {name}\n"
    if workflow.count(prefix) != 1:
        raise AssertionError(f"expected exactly one CI step named {name!r}")
    return workflow.index(prefix)


class RustGateOrder(unittest.TestCase):
    """knx-desktop's Tauri build script reads apps/knx-web/dist.

    A gate that lints or tests the workspace before the web bundle exists
    fails at that build script; excluding the crate instead hides whether
    it still builds. Both entry points therefore build the bundle first.
    """

    def test_ci_builds_the_web_bundle_before_rust_lints_and_tests(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        build = step_index(workflow, "Frontend build")
        self.assertLess(step_index(workflow, "Frontend install"), build)
        for rust_step in ("Clippy", "Test"):
            with self.subTest(step=rust_step):
                self.assertLess(build, step_index(workflow, rust_step))

    def test_ci_never_excludes_a_workspace_member(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        cargo = [line for line in workflow.splitlines()
                 if "cargo " in line and not line.lstrip().startswith("#")]
        self.assertTrue(cargo)
        for line in cargo:
            with self.subTest(line=line.strip()):
                self.assertNotIn("--exclude", line)

    def test_the_documented_rust_gate_builds_the_bundle_first(self) -> None:
        text = VERIFICATION.read_text(encoding="utf-8")
        match = re.search(r"^## Rust workspace gate\n(.*?)(?=^## )", text, re.M | re.S)
        if match is None:
            self.fail("VERIFICATION.md has no Rust workspace gate section")
        section = match.group(1)
        commands = [line.strip() for line in section.splitlines()
                    if line.strip().startswith(("cargo ", "(cd apps/knx-web"))]
        self.assertTrue(commands, "the section lists no commands")
        self.assertIn("npm run build", commands[0])
        for expected in ("cargo clippy --workspace --all-targets -- -D warnings",
                         "cargo test --workspace"):
            with self.subTest(command=expected):
                self.assertIn(expected, commands)
        for command in commands:
            with self.subTest(command=command):
                self.assertNotIn("--exclude", command)


if __name__ == "__main__":
    unittest.main()
