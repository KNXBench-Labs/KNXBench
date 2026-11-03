"""Check gate admission, stage order, diagnostic proof and environment isolation."""
from __future__ import annotations

from contextlib import redirect_stdout
from contextlib import contextmanager
import io
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

from tools import gate_scope
from tools import run_web_gate
from tools import run_package_gate as runner
from tools.run_package_gate import ISOLATED_STAGES, StageEnvironments, check, preflight, stage_plan
from tools import user_data_guard
from tools.user_data_guard import (CARGO_KEEPS_EXPORTED_HOMES, USER_HOMES, Snapshot, isolated_environment,
                                   record_user_data, touched_user_data, user_data_snapshot, user_knx_dirs)

# Independent of the production set: dropping a stage from it must fail a test.
REQUIRED_ISOLATED = ('tool-tests', 'rust-tests', 'diagnostic-sections-tests', 'bindings',
                     'corpus', 'website-tests', 'story-tests')


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

    def test_diagnostic_data_has_an_explicit_narrow_stage_but_full_stays_full(self):
        result = gate_scope.Plan(changed=['apps/knx-server/src/diagnostic_bundle_sections.rs'],
                                 diagnostic_sections=True)
        stages = stage_plan(result, full=False)
        self.assertIn('diagnostic-sections-tests', stages)
        self.assertNotIn('rust-tests', stages)
        self.assertNotIn('corpus', stages)
        self.assertNotIn('diagnostic-sections-tests', stage_plan(result, full=True))


class RecordingRun:
    """Stands in for MeasuredRun: keeps the environment each step was given."""

    def __init__(self) -> None:
        self.envs: dict[str, dict[str, str]] = {}
        self.commands: dict[str, list[str]] = {}

    def step(self, name, command, cwd, env, verify=None) -> dict:
        self.envs[name] = env
        self.commands[name] = command
        return {'exit': 0, 'wallSeconds': 0.0, 'proofValid': True}


class DiagnosticProof(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        # The CLI canonicalizes --root, including symlinked temporary parents.
        self.root = Path(directory.name).resolve()
        self.log = self.root / 'diagnostic.log'

    def test_inventory_requires_nonzero_expected_prefix_and_summary_agreement(self):
        prefix = 'diagnostic_bundle_routes::sections::tests::'
        text = f'test {prefix}privacy ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out\n'
        self.log.write_text(text)
        proof = runner.diagnostic_counts(self.log)
        self.assertEqual(proof['tests'], [prefix + 'privacy'])
        self.assertEqual(proof['passed'], 1)
        for value in ('', 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 filtered out\n',
                      f'test {prefix}privacy ... ignored\ntest result: ok. 0 passed; 0 failed; 1 ignored\n',
                      text.replace(prefix, 'unrelated::'), text.replace('1 passed', '2 passed'),
                      text.replace('0 failed', '1 failed'), text + text):
            with self.subTest(value=value):
                self.log.write_text(value)
                with self.assertRaises(ValueError):
                    runner.diagnostic_counts(self.log)

    def test_narrow_stage_uses_lease_exact_filter_scratch_scrubbing_and_receipt(self):
        self._assert_narrow_orchestration()

    def test_narrow_mixed_independent_rust_stages_finish_in_planned_order(self):
        self._assert_narrow_orchestration(["xtask"])

    @unittest.skipUnless(os.name == 'posix', 'POSIX temporary-directory symlink fixture')
    def test_narrow_orchestration_with_a_symlinked_temporary_directory(self):
        with tempfile.TemporaryDirectory() as parent:
            base = Path(parent).resolve()
            target = base / 'actual'
            target.mkdir()
            alias = base / 'alias'
            alias.symlink_to(target, target_is_directory=True)
            # Force the fixture to encounter a symlink even with a canonical TMPDIR.
            with patch.object(tempfile, 'tempdir', str(alias)):
                case = DiagnosticProof('test_narrow_stage_uses_lease_exact_filter_scratch_scrubbing_and_receipt')
                case.setUp()
                try:
                    case._assert_narrow_orchestration()
                finally:
                    case.doCleanups()

    def _assert_narrow_orchestration(self, rust_packages=()):
        git(self.root, 'init', '-q')
        (self.root / 'README.md').write_text('# fixture')
        (self.root / '.gitignore').write_text('private-output\n')
        git(self.root, 'add', '.')
        git(self.root, 'commit', '-qm', 'base')
        result = gate_scope.Plan(changed=['apps/knx-server/src/diagnostic_bundle_sections.rs'],
                                 diagnostic_sections=True, rust_packages=list(rust_packages),
                                 corpus_packages=list(rust_packages))
        events = []
        instances = []
        output = self.root / 'private-output'

        class FakeRun(runner.MeasuredRun):
            # Commands are simulated; receipt completion uses the real finalizer.
            def __init__(self, path, stages):
                super().__init__(path, stages)
                instances.append(self)

            def step(self, name, command, cwd, env, verify):
                evidence = None
                if name == 'diagnostic-sections-tests':
                    self.command, self.env, self.cwd = command, env, cwd
                    self.in_lease = events[-1] == 'lease-enter'
                    (output / (name + '.log')).write_text(
                        'test diagnostic_bundle_routes::sections::tests::privacy ... ok\n'
                        'test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out\n')
                    evidence = verify()
                stage = {'exit': 0, 'proofValid': True, 'wallSeconds': 0.0, 'evidence': evidence}
                self.receipt['stages'].append({'name': name, 'command': command, **stage})
                return stage

        real_subprocess_run = subprocess.run

        def subprocess_run(command, *args, **kwargs):
            if command[:2] == ["python3", "tools/run_corpus_tests.py"] and "--list" in command:
                return subprocess.CompletedProcess(command, 0, "0 corpus tests\n", "")
            return real_subprocess_run(command, *args, **kwargs)

        @contextmanager
        def lease(*args):
            events.append('lease-enter')
            yield
            events.append('lease-exit')

        with patch.object(runner.gate_scope, 'changed_paths', return_value=result.changed), \
             patch.object(runner.gate_scope, 'plan', return_value=result) as planner, \
             patch.object(runner, 'preflight', return_value=[]), \
             patch.object(runner, 'MeasuredRun', FakeRun), \
             patch.object(runner, 'workspace_lease', lease), \
             patch.object(runner.subprocess, 'run', side_effect=subprocess_run), \
             patch.object(runner, 'source_fingerprint', return_value='frozen'), \
             patch.object(runner, 'user_knx_dirs', return_value=[]), \
             patch.dict(os.environ, {'KNX_FORBIDDEN_HARDWARE': 'never'}), \
             redirect_stdout(io.StringIO()):
            exit_code = runner.main(['--root', str(self.root), '--base', 'actual-base', '--output', str(output)])
        self.assertEqual(exit_code, 0)
        planner.assert_called_once_with(self.root, result.changed, comparison_base='actual-base')
        run = instances[0]
        self.assertEqual(run.command, ['cargo', *CARGO_KEEPS_EXPORTED_HOMES, 'test', '--offline', '--locked',
                                      '-p', 'knx-server', '--lib', 'diagnostic_bundle_routes::sections::tests::'])
        self.assertTrue(run.in_lease)
        self.assertEqual(events, ['lease-enter', 'lease-exit'])
        self.assertFalse(any(key.startswith('KNX_') for key in run.env))
        for variable, _ in USER_HOMES:
            self.assertTrue(Path(run.env[variable]).is_relative_to(output / 'scratch-homes'))
        self.assertEqual(run.cwd, self.root)
        stage = next(s for s in run.receipt['stages'] if s['name'] == 'diagnostic-sections-tests')
        self.assertEqual((stage['exit'], stage['evidence']['passed']), (0, 1))
        self.assertTrue(run.receipt['plan']['diagnostic_sections'])
        self.assertTrue(run.receipt['success'])
        self.assertTrue(run.receipt['complete'])
        self.assertEqual([stage['name'] for stage in run.receipt['stages']], run.expected)

    def test_real_python_child_exit_and_zero_inventory_fail_proof(self):
        good = ('test diagnostic_bundle_routes::sections::tests::privacy ... ok\n'
                'test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out\n')
        zero = 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 filtered out\n'
        for index, (text, exit_code, accepted) in enumerate(((good, 0, True), (zero, 0, False), (good, 7, False))):
            with self.subTest(exit=exit_code, accepted=accepted):
                run = runner.MeasuredRun(self.root / str(index), ['diagnostic-sections-tests'])
                env = {k: v for k, v in os.environ.items() if not k.startswith('KNX_')}
                envs = StageEnvironments(env, isolated_environment(env, self.root / ('homes-' + str(index))))
                command = ['python3', '-c', f'print({text!r}); raise SystemExit({exit_code})']
                def execute():
                    with redirect_stdout(io.StringIO()):
                        return check(run, 'diagnostic-sections-tests', command, self.root, envs,
                                     lambda: runner.diagnostic_counts(run.output / 'diagnostic-sections-tests.log'))
                if accepted:
                    self.assertEqual(execute()['evidence']['passed'], 1)
                else:
                    with self.assertRaises(ValueError):
                        execute()
                self.assertEqual(run.receipt['stages'][0]['exit'], exit_code)
                self.assertEqual(run.finish('frozen', 'frozen'), accepted)

    def test_broad_server_selection_prevents_redundant_targeted_stage(self):
        result = gate_scope.Plan(diagnostic_sections=True, rust_packages=['knx-server'])
        self.assertNotIn('diagnostic-sections-tests', stage_plan(result, full=False))


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
            for stage in ('tool-tests', 'rust-tests', 'diagnostic-sections-tests', 'bindings', 'corpus', 'website-tests', 'story-tests',
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

    def test_a_plain_cargo_run_gets_forced_scratch_homes_below_target(self) -> None:
        # Sessions and CI images export the real homes, so `force = false`
        # would leave a plain `cargo test` on them.
        config = tomllib.loads((Path(__file__).resolve().parents[2] / '.cargo/config.toml').read_text())
        for variable, _ in USER_HOMES:
            entry = config['env'][variable]
            self.assertEqual((entry['relative'], entry['force']), (True, True), variable)
            self.assertTrue(entry['value'].startswith('target/dev-home/'), entry)
        self.assertEqual(len({config['env'][v]['value'] for v, _ in USER_HOMES}), len(USER_HOMES))

    def test_isolated_cargo_stages_keep_their_scratch_homes_over_the_cargo_default(self) -> None:
        envs = StageEnvironments(self.plain, isolated_environment(self.plain, self.root / 'homes'))
        run = RecordingRun()
        with redirect_stdout(io.StringIO()):
            check(run, 'rust-tests', ['cargo', 'test', '--locked'], self.root, envs)
            check(run, 'bindings', ['cargo', 'test', '-p', 'knx-projection'], self.root, envs)
            check(run, 'tool-tests', ['python3', '-m', 'unittest'], self.root, envs)
            check(run, 'clippy', ['cargo', 'clippy'], self.root, envs)
        self.assertEqual(CARGO_KEEPS_EXPORTED_HOMES,
                         ('--config', 'env.XDG_DATA_HOME.force=false', '--config', 'env.XDG_CONFIG_HOME.force=false'))
        self.assertEqual(run.commands['rust-tests'], ['cargo', *CARGO_KEEPS_EXPORTED_HOMES, 'test', '--locked'])
        self.assertEqual(run.commands['bindings'][-3:], ['test', '-p', 'knx-projection'])
        self.assertEqual(run.commands['bindings'][1:5], list(CARGO_KEEPS_EXPORTED_HOMES))
        self.assertEqual(run.commands['tool-tests'], ['python3', '-m', 'unittest'])
        self.assertEqual(run.commands['clippy'], ['cargo', 'clippy'])

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

    def test_a_parent_that_cannot_be_searched_fails_instead_of_counting_as_absent(self) -> None:
        share = self.root / 'share'
        (share / 'knx').mkdir(parents=True)
        (share / 'knx/products.sqlite').write_bytes(b'v25')
        share.chmod(0o600)  # readable, not searchable: `exists()` would say "absent"
        self.addCleanup(share.chmod, 0o700)
        if os.access(share / 'knx', os.F_OK):
            self.skipTest('running with a capability that searches any directory')
        snapshot = user_data_snapshot([share / 'knx'])
        self.assertEqual(snapshot.entries, {})
        self.assertEqual(len(snapshot.problems), 1)
        self.assertIn('cannot be checked', snapshot.problems[0])
        self.assertEqual(user_data_snapshot([self.root / 'missing/knx']).problems, [])


if __name__ == '__main__':
    unittest.main()
