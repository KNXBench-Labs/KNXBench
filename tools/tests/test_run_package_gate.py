"""Package gate runner: refusals before any stage runs."""
from __future__ import annotations

from pathlib import Path
import subprocess
import tempfile
import unittest

from tools import gate_scope
from tools.run_package_gate import preflight, stage_plan


def git(root: Path, *args: str) -> None:
    subprocess.run(['git', '-c', 'user.name=t', '-c', 'user.email=t@example.invalid', *args],
                   cwd=root, check=True, capture_output=True)


class Preflight(unittest.TestCase):
    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        bindings = self.root / 'apps/knx-web/src/bindings'
        bindings.mkdir(parents=True)
        (bindings / 'Device.ts').write_text('export type Device = { id: number };\n')
        git(self.root, 'init', '-q')
        git(self.root, 'add', '.')
        git(self.root, 'commit', '-q', '-m', 'base')

    def test_an_empty_change_set_is_refused_unless_the_gate_is_complete(self) -> None:
        empty = gate_scope.Plan()
        self.assertEqual(len(preflight(self.root, empty, full=False)), 1)
        self.assertIn('no change', preflight(self.root, empty, full=False)[0])
        self.assertEqual(preflight(self.root, empty, full=True), [])

    def test_uncommitted_bindings_are_refused_before_the_bindings_stage_restores_them(self) -> None:
        planned = gate_scope.Plan(changed=['apps/knx-web/src/bindings/Device.ts'], bindings=True)
        self.assertEqual(preflight(self.root, planned, full=False), [])
        (self.root / 'apps/knx-web/src/bindings/Device.ts').write_text('export type Device = { id: string };\n')
        refusals = preflight(self.root, planned, full=False)
        self.assertEqual(len(refusals), 1)
        self.assertIn('apps/knx-web/src/bindings', refusals[0])
        # Without a bindings stage nothing restores them, so nothing is refused.
        self.assertEqual(preflight(self.root, gate_scope.Plan(changed=['README.md']), full=False), [])

    def test_the_complete_gate_names_every_stage(self) -> None:
        self.assertEqual(stage_plan(gate_scope.Plan(), full=True),
                         ['fmt', 'web-gate', 'clean-desktop', 'clippy', 'rust-tests', 'bindings',
                          'bindings-diff', 'corpus', 'website-tests', 'story-tests'])


if __name__ == '__main__':
    unittest.main()
