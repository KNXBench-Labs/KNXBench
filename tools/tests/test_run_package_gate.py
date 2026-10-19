"""Package gate runner: refusals before any stage runs."""
from __future__ import annotations

from pathlib import Path
import subprocess
import tempfile
import unittest

from tools import gate_scope
from tools.run_package_gate import (ISOLATED_STAGES, StageEnvironments, isolated_environment, preflight,
                                    record_user_data, stage_plan, touched_user_data, user_data_fingerprint, user_knx_dirs)


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


class UserDataIsolation(unittest.TestCase):
    """Stages that run the apps' code never see the developer's real knx directories."""

    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)

    def test_the_real_directories_resolve_like_the_apps_resolve_them(self) -> None:
        home = '/home/alice'
        self.assertEqual(user_knx_dirs({'HOME': home}),
                         [Path(home, '.local/share/knx'), Path(home, '.config/knx')])
        self.assertEqual(user_knx_dirs({'HOME': home, 'XDG_DATA_HOME': '/data', 'XDG_CONFIG_HOME': ''}),
                         [Path('/data/knx'), Path(home, '.config/knx')])
        self.assertEqual(user_knx_dirs({}), [])

    def test_isolated_stages_get_empty_scratch_homes_and_the_web_gate_keeps_the_real_ones(self) -> None:
        plain = {'HOME': '/home/alice', 'XDG_DATA_HOME': '/home/alice/.local/share', 'PATH': '/bin'}
        envs = StageEnvironments(plain, isolated_environment(plain, self.root / 'homes'))
        for stage in ('rust-tests', 'bindings', 'corpus', 'tool-tests'):
            self.assertIn(stage, ISOLATED_STAGES)
            env = envs.for_stage(stage)
            for variable in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
                self.assertTrue(Path(env[variable]).is_relative_to(self.root / 'homes'), (stage, variable))
                self.assertEqual(list(Path(env[variable]).iterdir()), [])
            self.assertEqual((env['HOME'], env['PATH']), ('/home/alice', '/bin'))
        for stage in ('web-gate', 'fmt', 'clippy', 'diff-check'):
            self.assertIs(envs.for_stage(stage), plain)
        self.assertEqual(plain['XDG_DATA_HOME'], '/home/alice/.local/share')

    def test_a_created_changed_or_removed_file_counts_as_touched(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        (data / 'products.sqlite').write_bytes(b'v25')
        (data / 'old').write_bytes(b'x')
        dirs = [data, self.root / 'config/knx']
        before = user_data_fingerprint(dirs)
        self.assertEqual(touched_user_data(before, user_data_fingerprint(dirs)), [])
        (data / 'products.sqlite').write_bytes(b'v26 migrated')
        (data / 'old').unlink()
        (self.root / 'config/knx').mkdir(parents=True)
        (self.root / 'config/knx/settings.json').write_text('{}')
        self.assertEqual(touched_user_data(before, user_data_fingerprint(dirs)),
                         sorted(str(p) for p in (data / 'products.sqlite', data / 'old',
                                                 self.root / 'config/knx/settings.json')))

    def test_a_touched_real_directory_fails_the_receipt_and_keeps_an_earlier_error(self) -> None:
        clean = {'planCompleted': True}
        record_user_data(clean, {'/a': [1, 2, 3]}, {'/a': [1, 2, 3]})
        self.assertEqual(clean, {'planCompleted': True, 'userDataTouched': []})
        failed = {'planCompleted': True, 'error': 'failed stage rust-tests'}
        record_user_data(failed, {}, {'/home/alice/.local/share/knx/products.sqlite': [9, 9, 9]})
        self.assertFalse(failed['planCompleted'])
        self.assertEqual(failed['userDataTouched'], ['/home/alice/.local/share/knx/products.sqlite'])
        self.assertTrue(failed['error'].startswith('failed stage rust-tests; the real knx data'))


if __name__ == '__main__':
    unittest.main()
