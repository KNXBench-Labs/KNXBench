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
from tools import user_data_guard
from tools.user_data_guard import (Snapshot, isolated_environment, record_user_data, touched_user_data,
                                   user_data_snapshot, user_knx_dirs)

# Independent of the production set: dropping a stage from it must fail a test.
REQUIRED_ISOLATED = ('tool-tests', 'rust-tests', 'bindings', 'corpus', 'website-tests', 'story-tests')


def entries(dirs, home=None):
    snapshot = user_data_snapshot(dirs, home)
    assert snapshot.problems == [], snapshot.problems
    return snapshot.entries


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
        self.assertTrue(set(REQUIRED_ISOLATED) <= ISOLATED_STAGES)
        for stage in REQUIRED_ISOLATED:
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
        before = entries(dirs)
        self.assertEqual(touched_user_data(before, entries(dirs)), [])
        (data / 'products.sqlite').write_bytes(b'v26 migrated')
        (data / 'old').unlink()
        (self.root / 'config/knx').mkdir(parents=True)
        (self.root / 'config/knx/settings.json').write_text('{}')
        self.assertEqual(touched_user_data(before, entries(dirs)),
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
        before = entries([data])
        self.assertIn(str(data / 'linked/remembered'), before)
        self.assertEqual(touched_user_data(before, entries([data])), [])
        (elsewhere / 'products.sqlite').write_bytes(b'v26 migrated database')
        (elsewhere / 'nested/remembered').write_bytes(b'b')
        self.assertEqual(touched_user_data(before, entries([data])),
                         [str(data / 'linked/remembered'), str(data / 'products.sqlite')])

    def test_a_same_size_rewrite_with_the_old_mtime_still_counts_as_touched(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        database = data / 'products.sqlite'
        database.write_bytes(b'v25')
        status = database.stat()
        before = entries([data])
        database.write_bytes(b'v26')
        os.utime(database, ns=(status.st_atime_ns, status.st_mtime_ns))
        self.assertEqual(touched_user_data(before, entries([data])), [str(database)])

    def test_a_touched_real_directory_fails_the_receipt_and_keeps_an_earlier_error(self) -> None:
        clean = {'planCompleted': True}
        record_user_data(clean, Snapshot({'/a': [1, 2, 3]}), Snapshot({'/a': [1, 2, 3]}))
        self.assertEqual(clean, {'planCompleted': True, 'userDataTouched': [], 'userDataUnchecked': []})
        failed = {'planCompleted': True, 'error': 'failed stage rust-tests'}
        record_user_data(failed, Snapshot(), Snapshot({'/home/alice/.local/share/knx/products.sqlite': [9, 9, 9]}))
        self.assertFalse(failed['planCompleted'])
        self.assertEqual(failed['userDataTouched'], ['/home/alice/.local/share/knx/products.sqlite'])
        self.assertTrue(failed['error'].startswith('failed stage rust-tests; the real knx data'))

    def test_a_directory_that_cannot_be_listed_fails_instead_of_counting_as_clean(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        (data / 'products.sqlite').write_bytes(b'v25')
        data.chmod(0o100)  # execute-only: the file opens, the directory does not list
        self.addCleanup(data.chmod, 0o700)
        if os.access(data, os.R_OK):
            self.skipTest('running with a capability that reads any directory')
        snapshot = user_data_snapshot([data])
        self.assertEqual(snapshot.entries, {})
        self.assertEqual(len(snapshot.problems), 1)
        self.assertIn('cannot be listed', snapshot.problems[0])
        receipt = {'planCompleted': True}
        record_user_data(receipt, snapshot, snapshot)
        self.assertFalse(receipt['planCompleted'])
        self.assertIn('could not be fully checked', receipt['error'])

    def test_a_file_removed_while_scanning_is_a_change_not_a_crash(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        (data / 'products.sqlite').write_bytes(b'v25')
        (data / 'products.sqlite-wal').write_bytes(b'wal')
        before = user_data_snapshot([data])
        real_entry = user_data_guard._entry

        def vanishing(path: Path) -> list:
            if path.name == 'products.sqlite-wal' and path.exists():
                path.unlink()  # the app checkpoints between listing and lstat
            return real_entry(path)

        user_data_guard._entry = vanishing
        self.addCleanup(setattr, user_data_guard, '_entry', real_entry)
        after = user_data_snapshot([data])
        self.assertEqual(after.problems, [])
        self.assertEqual(touched_user_data(before.entries, after.entries), [str(data / 'products.sqlite-wal')])

    def test_a_link_to_home_or_an_ancestor_is_reported_and_not_walked(self) -> None:
        home = self.root / 'home'
        (home / 'src/checkout/target').mkdir(parents=True)
        (home / 'src/checkout/target/gate.log').write_text('busy')
        config = home / '.config/knx'
        config.mkdir(parents=True)
        (config / 'legacy-vd-password').write_text('x')
        (config / 'home').symlink_to(home, target_is_directory=True)
        (config / 'up').symlink_to(home / '.config', target_is_directory=True)
        snapshot = user_data_snapshot([config], str(home))
        self.assertEqual(sorted(snapshot.entries), [str(config / 'home'), str(config / 'legacy-vd-password'),
                                                    str(config / 'up')])
        self.assertEqual(len(snapshot.problems), 2)
        self.assertTrue(all('ancestor of the knx directory or of HOME' in p for p in snapshot.problems))
        (home / 'src/checkout/target/gate.log').write_text('busier')
        self.assertEqual(touched_user_data(snapshot.entries, user_data_snapshot([config], str(home)).entries), [])

    def test_more_entries_than_the_bound_are_reported_not_walked_forever(self) -> None:
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        for index in range(4):
            (data / f'f{index}').write_text('x')
        bound = user_data_guard.MAX_ENTRIES
        user_data_guard.MAX_ENTRIES = 2
        self.addCleanup(setattr, user_data_guard, 'MAX_ENTRIES', bound)
        snapshot = user_data_snapshot([data])
        self.assertEqual(len(snapshot.entries), 3)
        self.assertIn('more than 2 entries', snapshot.problems[0])

    def test_retargeting_a_linked_directory_counts_as_touched(self) -> None:
        (self.root / 'one').mkdir()
        (self.root / 'two').mkdir()
        data = self.root / 'share/knx'
        data.mkdir(parents=True)
        (data / 'store').symlink_to(self.root / 'one', target_is_directory=True)
        before = entries([data])
        (data / 'store').unlink()
        (data / 'store').symlink_to(self.root / 'two', target_is_directory=True)
        self.assertEqual(touched_user_data(before, entries([data])), [str(data / 'store')])


if __name__ == '__main__':
    unittest.main()
