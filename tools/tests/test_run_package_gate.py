"""Package gate runner: refusals before any stage runs."""
from __future__ import annotations

from contextlib import redirect_stdout
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from tools import gate_scope
from tools import run_web_gate
from tools.run_package_gate import ISOLATED_STAGES, StageEnvironments, check, preflight, stage_plan
from tools.user_data_guard import (isolated_environment, record_user_data, touched_user_data,
                                   user_data_fingerprint, user_knx_dirs)


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


class RecordingRun:
    """Stands in for MeasuredRun: keeps the environment each step was given."""

    def __init__(self) -> None:
        self.envs: dict[str, dict[str, str]] = {}

    def step(self, name, command, cwd, env, verify=None) -> dict:
        self.envs[name] = env
        return {'exit': 0, 'wallSeconds': 0.0, 'proofValid': True}


class UserDataIsolation(unittest.TestCase):
    """Stages that run the branch's tests never see the developer's real knx directories."""

    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.plain = {'HOME': '/home/alice', 'XDG_DATA_HOME': '/home/alice/.local/share', 'PATH': '/bin',
                      'TMPDIR': str(self.root / 'tmp')}

    def assert_scratch(self, env: dict[str, str], below: Path, label: str) -> None:
        for variable in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
            self.assertTrue(Path(env[variable]).is_relative_to(below), (label, variable, env.get(variable)))
            self.assertEqual(list(Path(env[variable]).iterdir()), [], (label, variable))
        self.assertEqual((env['HOME'], env['PATH']), ('/home/alice', '/bin'), label)

    def test_the_real_directories_resolve_like_the_apps_resolve_them(self) -> None:
        home = '/home/alice'
        self.assertEqual(user_knx_dirs({'HOME': home}),
                         [Path(home, '.local/share/knx'), Path(home, '.config/knx')])
        self.assertEqual(user_knx_dirs({'HOME': home, 'XDG_DATA_HOME': '/data', 'XDG_CONFIG_HOME': ''}),
                         [Path('/data/knx'), Path(home, '.config/knx')])
        self.assertEqual(user_knx_dirs({}), [])

    def test_the_package_gate_gives_its_test_stages_scratch_homes_through_check(self) -> None:
        envs = StageEnvironments(self.plain, isolated_environment(self.plain, self.root / 'homes'))
        run = RecordingRun()
        with redirect_stdout(io.StringIO()):
            for stage in ('tool-tests', 'rust-tests', 'bindings', 'corpus', 'website-tests', 'story-tests',
                          'web-gate', 'fmt', 'clippy', 'diff-check'):
                check(run, stage, ['true'], self.root, envs)
            check(run, 'bindings', ['true'], self.root, envs, extra={'TS_RS_EXPORT_DIR': 'out'})
        for stage in ISOLATED_STAGES:
            self.assert_scratch(run.envs[stage], self.root / 'homes', stage)
        for stage in ('web-gate', 'fmt', 'clippy', 'diff-check'):
            self.assertEqual(run.envs[stage], self.plain, stage)
        self.assertEqual(run.envs['bindings']['TS_RS_EXPORT_DIR'], 'out')
        self.assert_scratch(run.envs['bindings'], self.root / 'homes', 'bindings + extra')

    def test_the_complete_web_profile_runs_its_tool_tests_with_scratch_homes(self) -> None:
        run = RecordingRun()
        with redirect_stdout(io.StringIO()):
            for step in ('tool-tests', 'documentation', 'build', 'vitest', 'browser-functional'):
                run_web_gate.checked_step(run, step, ['true'], self.root, self.plain)
        self.assert_scratch(run.envs['tool-tests'], self.root / 'tmp', 'web tool-tests')
        for step in ('documentation', 'build', 'vitest', 'browser-functional'):
            self.assertIs(run.envs[step], self.plain, step)

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

    def test_a_write_through_a_symlinked_database_or_directory_counts_as_touched(self) -> None:
        elsewhere = self.root / 'elsewhere'
        (elsewhere / 'nested').mkdir(parents=True)
        (elsewhere / 'products.sqlite').write_bytes(b'v25')
        (elsewhere / 'nested/remembered').write_bytes(b'a')
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        (data / 'products.sqlite').symlink_to(elsewhere / 'products.sqlite')
        (data / 'linked').symlink_to(elsewhere / 'nested', target_is_directory=True)
        (data / 'loop').symlink_to(data, target_is_directory=True)  # a cycle must not hang the walk
        before = user_data_fingerprint([data])
        self.assertIn(str(data / 'linked/remembered'), before)
        self.assertEqual(touched_user_data(before, user_data_fingerprint([data])), [])
        (elsewhere / 'products.sqlite').write_bytes(b'v26 migrated database')
        (elsewhere / 'nested/remembered').write_bytes(b'b')
        self.assertEqual(touched_user_data(before, user_data_fingerprint([data])),
                         [str(data / 'linked/remembered'), str(data / 'products.sqlite')])

    def test_a_same_size_rewrite_with_the_old_mtime_still_counts_as_touched(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        database = data / 'products.sqlite'
        database.write_bytes(b'v25')
        status = database.stat()
        before = user_data_fingerprint([data])
        database.write_bytes(b'v26')
        os.utime(database, ns=(status.st_atime_ns, status.st_mtime_ns))
        self.assertEqual(touched_user_data(before, user_data_fingerprint([data])), [str(database)])

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
