"""The web gate refuses a successful-looking browser run with missing cases."""
from __future__ import annotations

import copy
import importlib
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from tools import web_gate_support as support


def report(*ids: str, executed: bool = False, tags: dict[str, list[str]] | None = None) -> dict:
    return {
        'errors': [],
        'suites': [{'specs': [
            {'id': id_, 'file': f'{id_}.e2e.ts', 'tags': (tags or {}).get(id_, []), 'tests': [{
                'projectId': '', 'expectedStatus': 'passed',
                'results': [{'status': 'passed', 'retry': 0}] if executed else [],
                'status': 'expected' if executed else 'skipped',
            }]} for id_ in ids
        ]}],
    }


class BrowserEvidence(unittest.TestCase):
    def test_equal_case_ids_are_required_even_when_command_exits_zero(self) -> None:
        want = support.browser_cases(report('one', 'two'))
        with self.assertRaisesRegex(ValueError, 'inventory'):
            support.validate_browser(report('one', executed=True), want)
        self.assertEqual(support.validate_browser(report('one', 'two', executed=True), want), 2)

    def test_bad_browser_evidence_never_becomes_acceptance(self) -> None:
        want = support.browser_cases(report('one'))
        for kind in ('empty', 'errors', 'duplicate', 'skip', 'retry', 'failure', 'expected-failure'):
            with self.subTest(kind=kind):
                data = report('one', executed=True)
                case = data['suites'][0]['specs'][0]['tests'][0]
                if kind == 'empty':
                    data['suites'] = []
                elif kind == 'errors':
                    data['errors'] = [{'message': 'startup failed'}]
                elif kind == 'duplicate':
                    data['suites'].append(copy.deepcopy(data['suites'][0]))
                elif kind == 'skip':
                    case['results'][0]['status'] = 'skipped'
                elif kind == 'retry':
                    case['results'][0]['retry'] = 1
                elif kind == 'failure':
                    case['status'] = 'unexpected'
                    case['results'][0]['status'] = 'failed'
                else:
                    case['expectedStatus'] = 'failed'
                with self.assertRaises(ValueError):
                    support.validate_browser(data, want)


class VitestEvidence(unittest.TestCase):
    def test_missing_failures_and_skips_are_not_successful_stages(self) -> None:
        self.assertTrue(hasattr(support, 'validate_vitest'), 'Vitest evidence validator is missing')
        good = {'numTotalTests': 2, 'numPassedTests': 2, 'numFailedTests': 0,
                'numPendingTests': 0, 'numTotalTestSuites': 1, 'success': True}
        self.assertEqual(support.validate_vitest(good), 2)
        for bad in ({}, {**good, 'numTotalTests': 0}, {**good, 'numPassedTests': 1},
                    {**good, 'numPendingTests': 1}, {**good, 'success': False}):
            with self.assertRaises(ValueError):
                support.validate_vitest(bad)


class LanePolicy(unittest.TestCase):
    def test_only_tagged_cases_run_alone_and_the_union_is_every_case(self) -> None:
        cases = support.browser_cases(report('plain', 'timed', 'both',
                                             tags={'timed': ['exclusive'], 'both': ['slow', 'exclusive']}))
        functional, exclusive = support.split_lanes(cases)
        self.assertEqual(sorted(c['file'] for c in functional.values()), ['plain.e2e.ts'])
        self.assertEqual(sorted(c['file'] for c in exclusive.values()), ['both.e2e.ts', 'timed.e2e.ts'])
        self.assertEqual(functional.keys() | exclusive.keys(), cases.keys())
        self.assertFalse(functional.keys() & exclusive.keys())

    def test_a_malformed_tag_list_is_refused(self) -> None:
        data = report('one')
        data['suites'][0]['specs'][0]['tags'] = 'exclusive'
        with self.assertRaisesRegex(ValueError, 'tag'):
            support.browser_cases(data)

    def test_an_empty_lane_is_recorded_without_pretending_to_run(self) -> None:
        with TemporaryDirectory() as tmp:
            run = support.MeasuredRun(Path(tmp) / 'attempt', ['lane'])
            stage = run.empty('lane', 'no exclusive case')
            self.assertIsNone(stage['command'])
            self.assertEqual(stage['empty'], 'no exclusive case')
            self.assertTrue(run.finish('same', 'same'))
            with self.assertRaises(ValueError):
                run.empty('lane', 'twice')


class MeasuredRun(unittest.TestCase):
    def test_unavailable_child_subreaper_fails_before_creating_an_attempt(self) -> None:
        from unittest.mock import patch
        with patch.object(support.sys, 'platform', 'unsupported'):
            with self.assertRaisesRegex(RuntimeError, 'subreaper'):
                support.enable_child_subreaper()
        with patch.object(support.os, 'pidfd_open', side_effect=OSError(38, 'not implemented')):
            with self.assertRaisesRegex(RuntimeError, 'pidfd'):
                support.enable_child_subreaper()
        with patch.object(support.signal, 'pidfd_send_signal', side_effect=OSError(1, 'denied')):
            with self.assertRaisesRegex(RuntimeError, 'pidfd'):
                support.enable_child_subreaper()
        with TemporaryDirectory() as tmp:
            output = Path(tmp) / 'attempt'
            with patch.object(support, 'enable_child_subreaper',
                              side_effect=RuntimeError('Linux child subreaper unavailable'),
                              create=True):
                with self.assertRaisesRegex(RuntimeError, 'subreaper'):
                    support.MeasuredRun(output)
            self.assertFalse(output.exists())

    def test_real_process_exit_time_and_failure_survive_in_receipt(self) -> None:
        self.assertTrue(hasattr(support, 'MeasuredRun'), 'Measured process receipts are missing')
        with TemporaryDirectory() as tmp:
            output = Path(tmp) / 'attempt'
            run = support.MeasuredRun(output)
            stage = run.step('failure', [sys.executable, '-c', 'print("real result");raise SystemExit(7)'], Path(tmp))
            self.assertEqual(stage['exit'], 7)
            self.assertGreater(stage['wallSeconds'], 0)
            self.assertGreaterEqual(stage['cpuSeconds'], 0)
            self.assertGreater(stage['peakRssKiB'], 0)
            self.assertIn('real result', (output / 'failure.log').read_text())
            run.finish('start', 'start')
            receipt = json.loads((output / 'receipt.json').read_text())
            self.assertFalse(receipt['success'])
            self.assertTrue(receipt['inputsFrozen'])
            self.assertEqual(receipt['stages'][0]['exit'], 7)
            with self.assertRaises(FileExistsError):
                support.MeasuredRun(output)

    def test_missing_stages_drift_and_invalid_proof_fail_closed(self) -> None:
        with TemporaryDirectory() as tmp:
            output = Path(tmp) / 'attempt'
            run = support.MeasuredRun(output, ['first', 'second'])
            run.step('first', [sys.executable, '-c', 'pass'], Path(tmp))
            self.assertFalse(run.finish('same', 'same'))
            run.step('second', [sys.executable, '-c', 'pass'], Path(tmp),
                     verify=lambda: (_ for _ in ()).throw(ValueError('missing proof')))
            self.assertFalse(run.finish('same', 'same'))
            self.assertEqual(run.receipt['stages'][1]['exit'], 0)
            self.assertFalse(run.receipt['stages'][1]['proofValid'])
            self.assertFalse(run.finish('before', 'after'))
            with self.assertRaises(ValueError):
                run.step('first', [sys.executable, '-c', 'pass'], Path(tmp))

    def test_interrupt_during_popen_still_cleans_the_spawned_child(self) -> None:
        import time
        from unittest.mock import patch
        spawned = []
        original = subprocess.Popen

        def interrupt_after_spawn(*args, **kwargs):
            child = original(*args, **kwargs)
            spawned.append(child)
            raise KeyboardInterrupt

        with TemporaryDirectory() as tmp:
            run = support.MeasuredRun(Path(tmp) / 'attempt')
            try:
                with patch.object(support.subprocess, 'Popen', interrupt_after_spawn):
                    with self.assertRaises(KeyboardInterrupt):
                        run.step('spawn', [sys.executable, '-c', 'import time;time.sleep(60)'], Path.cwd())
                self.assertEqual(len(spawned), 1)
                deadline = time.monotonic() + 2
                while spawned[0].poll() is None and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertIsNotNone(spawned[0].returncode,
                                     'child survived interruption before Popen returned')
            finally:
                for child in spawned:
                    if child.poll() is None:
                        child.kill()
                    try:
                        child.wait(timeout=5)
                    except ChildProcessError:
                        pass

    def test_handled_signal_waits_until_a_fast_supervisor_identity_is_attached(self) -> None:
        import os
        import signal
        import time
        from unittest.mock import patch
        spawned = []
        original = subprocess.Popen

        def signal_after_supervisor_exits(*args, **kwargs):
            child = original(*args, **kwargs)
            spawned.append(child)
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                info = support._process_info(child.pid)
                if info is not None and info.state == 'Z':
                    break
                time.sleep(.01)
            os.kill(os.getpid(), signal.SIGTERM)
            return child

        def interrupted(_signum, _frame):
            raise KeyboardInterrupt

        previous = signal.signal(signal.SIGTERM, interrupted)
        try:
            with TemporaryDirectory() as tmp:
                run = support.MeasuredRun(Path(tmp) / 'attempt')
                with (patch.object(support.subprocess, 'Popen', signal_after_supervisor_exits),
                      patch.object(support, 'SPAWN_SETTLE_SECONDS', .05),
                      patch.object(support, 'SUPERVISOR_CLEANUP_SECONDS', .05),
                      patch.object(support, 'KILL_WAIT_SECONDS', .05)):
                    with self.assertRaises(KeyboardInterrupt):
                        run.step('fast', [sys.executable, '-c', 'pass'], Path.cwd())
                self.assertEqual(len(spawned), 1)
                self.assertIsNone(support._process_info(spawned[0].pid),
                                  'already-exited supervisor was not reaped')
        finally:
            signal.signal(signal.SIGTERM, previous)
            for child in spawned:
                child.poll()
                info = support._process_info(child.pid)
                if info is not None:
                    if info.state != 'Z':
                        child.kill()
                    try:
                        child.wait(timeout=5)
                    except ChildProcessError:
                        pass

    def test_cleanup_is_idempotent_and_restores_the_callers_signal_mask(self) -> None:
        import signal
        from unittest.mock import patch
        previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGUSR1})
        expected = previous_mask | {signal.SIGUSR1}
        processes = support._StageProcessTree()

        def cleanup_pass(*_args):
            current = signal.pthread_sigmask(signal.SIG_BLOCK, [])
            self.assertEqual(current, expected | set(support.HANDLED_SIGNALS))
            return True

        try:
            with patch.object(processes, '_wait', side_effect=cleanup_pass):
                processes.terminate(None)
                processes.terminate(None)
            self.assertEqual(signal.pthread_sigmask(signal.SIG_BLOCK, []), expected)
            with (patch.object(processes, '_wait', return_value=False),
                  patch.object(processes, '_discover', return_value={})):
                with self.assertRaisesRegex(RuntimeError, 'cleanup incomplete'):
                    processes.terminate(None)
            self.assertEqual(signal.pthread_sigmask(signal.SIG_BLOCK, []), expected)
        finally:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)

    def test_late_signal_cannot_interrupt_supervisor_normal_completion_cleanup(self) -> None:
        import signal
        self._assert_late_cleanup_signal(signal.SIGTERM, repeated=False)
        self._assert_late_cleanup_signal(signal.SIGINT, repeated=False)

    def test_repeated_signals_cannot_interrupt_in_progress_cleanup(self) -> None:
        import signal
        self._assert_late_cleanup_signal(signal.SIGTERM, repeated=True)
        self._assert_late_cleanup_signal(signal.SIGINT, repeated=True)

    def _assert_late_cleanup_signal(self, signum: int, *, repeated: bool) -> None:
        with self.subTest(signum=signum, repeated=repeated), TemporaryDirectory() as tmp:
            result = subprocess.run(
                [sys.executable, '-m', 'tools.tests.fixtures.gate_cleanup_race', tmp,
                 str(int(signum)), 'repeat' if repeated else 'single'],
                cwd=Path(__file__).resolve().parents[2], capture_output=True, text=True, timeout=40)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            data = json.loads(result.stdout)
            self.assertEqual(data['error'], 'KeyboardInterrupt', data)
            self.assertEqual(len(data['observations']), 1, data)
            self.assertNotEqual(data['observations'][0]['oldParent'],
                                data['observations'][0]['liveParent'], data)
            self.assertEqual(data['forwarded'], [15], data)
            self.assertFalse(data['descendantPresent'],
                             f"late cancellation left scrubbed detached child: {data}")
            self.assertFalse(data['supervisorPresent'], data)
            self.assertTrue(data['unrelatedAlive'], data)
            self.assertFalse(data['unrelatedSignalled'], data)
            self.assertFalse(data['receipt']['success'], data)
            self.assertFalse(data['receipt']['complete'], data)
            self.assertIsNone(data['receipt']['stages'][0]['exit'], data)

    def test_nonzero_supervisor_exit_cannot_accept_a_valid_target_report(self) -> None:
        from unittest.mock import patch
        original = support.os.wait4

        def failed_supervisor(pid, options):
            waited_pid, _status, usage = original(pid, options)
            return waited_pid, 1 << 8, usage

        with TemporaryDirectory() as tmp:
            run = support.MeasuredRun(Path(tmp) / 'attempt')
            with patch.object(support.os, 'wait4', failed_supervisor):
                with self.assertRaisesRegex(RuntimeError, 'supervisor'):
                    run.step('failed-supervisor', [sys.executable, '-c', 'pass'], Path.cwd())

    def test_source_fingerprint_covers_untracked_bytes_and_ignores_output(self) -> None:
        self.assertTrue(hasattr(support, 'source_fingerprint'), 'Source-bound acceptance is missing')
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / '.gitignore').write_text('target/\n')
            (root / 'source.ts').write_text('one')
            before = support.source_fingerprint(root)
            (root / 'target').mkdir()
            (root / 'target/report.json').write_text('{}')
            self.assertEqual(support.source_fingerprint(root), before)
            (root / 'source.ts').write_text('two')
            self.assertNotEqual(support.source_fingerprint(root), before)


class GatePlan(unittest.TestCase):
    def test_workbench_splitter_spec_is_in_a_discovered_typecheck_config(self) -> None:
        from tools.run_web_gate import typecheck_configs
        app = Path(__file__).resolve().parents[2] / 'apps/knx-web'
        name = 'tsconfig.workbench-fixtures.json'
        self.assertIn(name, typecheck_configs(app), 'splitter typecheck config is missing')
        config = json.loads((app / name).read_text())
        self.assertEqual(config['extends'], './tsconfig.json')
        self.assertIn('e2e/workbench-splitters.e2e.ts', config['include'])

    def test_app_is_checked_once_and_every_derived_config_is_preserved(self) -> None:
        try:
            runner = importlib.import_module('tools.run_web_gate')
        except ModuleNotFoundError:
            runner = None
        self.assertIsNotNone(runner, 'Canonical web gate planning is missing')
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            for name in ('tsconfig.json', 'tsconfig.flow-study.json', 'tsconfig.theme-fixtures.json'):
                (app / name).write_text('{}')
            self.assertEqual(runner.typecheck_configs(app),
                             ['tsconfig.flow-study.json', 'tsconfig.theme-fixtures.json'])
    def test_cli_and_command_selection_preserve_offline_zero_retry_execution(self) -> None:
        from tools.run_web_gate import browser_command, environment
        command = browser_command('playwright.config.ts', Path('/output'), 4, 'functional')
        self.assertEqual(command[:4], ['unshare', '--user', '--map-root-user', '--net'])
        tag = '@exclusive(?![\\w-])'  # the whole tag: `@exclusive-x` is another tag
        for argument in ('--workers=4', '--retries=0', '--repeat-each=1', '--forbid-only',
                         f'--grep-invert={tag}'):
            self.assertIn(argument, command)
        self.assertFalse([a for a in command if a.startswith('--grep=')])
        exclusive = browser_command('playwright.config.ts', Path('/output'), 1, 'exclusive')
        self.assertIn(f'--grep={tag}', exclusive)
        self.assertFalse([a for a in exclusive if a.startswith('--grep-invert')])
        import re
        for title, hit in (('a > b @exclusive', True), ('a > b @exclusive @slow', True),
                           ('a > b @exclusive-x', False), ('a > exclusive', False)):
            self.assertEqual(bool(re.search(tag, title)), hit, title)
        listing = browser_command('playwright.config.ts', Path('/output'), 1, None, listing=True)
        self.assertFalse(any(a.startswith('--grep') for a in listing))
        selected = browser_command('playwright.config.ts', Path('/output'), 4, 'functional',
                                   files=['safe.e2e.ts'])
        self.assertIn('(?:^|/)safe\\.e2e\\.ts$', selected)
        with self.assertRaises(ValueError):
            browser_command('playwright.config.ts', Path('/output'), 4, 'functional', files=[])
        with self.assertRaises(ValueError):
            browser_command('playwright.config.ts', Path('/output'), 4, 'sideways')
        from unittest.mock import patch
        with patch.dict('os.environ', {'KNX_GATEWAY': 'private', 'FLOW_CAMERA_NEGATIVE': '1',
                                       'PLAYWRIGHT_JSON_OUTPUT_NAME': 'foreign'}):
            env = environment('/owned')
        self.assertNotIn('KNX_GATEWAY', env)
        self.assertNotIn('FLOW_CAMERA_NEGATIVE', env)
        self.assertNotIn('PLAYWRIGHT_JSON_OUTPUT_NAME', env)
        self.assertEqual(env['TMPDIR'], '/owned')

    def test_tagged_scopes_are_counted_from_the_specs(self) -> None:
        from tools.run_web_gate import exclusive_scopes
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            (app / 'e2e').mkdir()
            (app / 'e2e/a.e2e.ts').write_text('test("a", { tag: "@exclusive" }, async () => {});\n'
                                              'test("b", async () => {});\n')
            self.assertEqual(exclusive_scopes(app), 1)
    def test_socket_budget_is_checked_before_running_a_suite(self) -> None:
        from tools import run_web_gate as runner
        self.assertTrue(hasattr(runner, 'check_temp_root'), 'Chromium socket-path preflight is missing')
        runner.check_temp_root(Path('/short'))
        with self.assertRaisesRegex(ValueError, 'socket'):
            runner.check_temp_root(Path('/' + 'deep' * 30))

    def test_profile_plan_has_an_explicit_tail_and_no_duplicate_app_check(self) -> None:
        from tools import run_web_gate as runner
        self.assertTrue(hasattr(runner, 'stage_plan'), 'Persisted profile stage plan is missing')
        plan = runner.stage_plan('web', ['tsconfig.extra.json'])
        self.assertEqual(plan.count('build'), 1)
        self.assertIn('browser-start', plan)
        self.assertIn('browser-functional', plan)
        self.assertIn('browser-exclusive', plan)
        self.assertLess(plan.index('browser-lanes'), plan.index('build'))
        for config in runner.PRODUCTION_CONFIGS:
            # Each suite lists its inventory, then runs the tag lanes (ADR-0154).
            stages = [f'production-{config}-{lane}' for lane in ('inventory', 'functional', 'exclusive')]
            self.assertEqual([plan.index(s) for s in stages], sorted(plan.index(s) for s in stages))
            self.assertNotIn(f'production-{config}', plan)
        self.assertEqual(plan[-1], 'diff-check')
        benchmark = runner.stage_plan('benchmark', [])
        self.assertEqual(benchmark[0], 'browser-lanes')
        self.assertEqual(benchmark[-1], 'benchmark-6-w1')
        self.assertEqual(len(benchmark), 9)

    def test_targeted_plan_keeps_build_typechecks_and_vitest_but_only_selected_suites(self) -> None:
        from tools import run_web_gate as runner
        plan = runner.stage_plan('targeted', ['tsconfig.extra.json'], ('devices',), True)
        for stage in ('browser-lanes', 'build', 'tsconfig.extra', 'vitest', 'browser-inventory',
                      'browser-functional', 'browser-exclusive', 'production-devices-inventory',
                      'production-devices-functional', 'production-devices-exclusive'):
            self.assertIn(stage, plan)
        self.assertFalse([s for s in plan if s.startswith('production-') and 'devices' not in s])
        for stage in ('documentation', 'tool-tests', 'check-layering', 'diff-check'):
            self.assertNotIn(stage, plan)
        quiet = runner.stage_plan('targeted', [], (), False)
        self.assertEqual(quiet, ['browser-lanes', 'build', 'vitest'])

    def test_production_suites_split_by_tag_like_the_default_lane(self) -> None:
        from unittest import mock
        from tools import run_web_gate as runner
        inventory = report('plain', 'other', 'timed', tags={'timed': ['exclusive']})
        calls = []

        def stage(run, app, env, name, config, lane, workers, expected=None, files=None):
            calls.append((name, config, lane, workers, None if expected is None else sorted(expected)))
            if expected is None:
                (run.output / f'{name}.json').write_text(json.dumps(inventory))
            return {}

        with TemporaryDirectory() as tmp, mock.patch.object(runner, 'browser_stage', stage):
            run = support.MeasuredRun(Path(tmp) / 'attempt', [])
            runner.run_production(run, Path(tmp), {}, 'devices', 4)
            cases = support.browser_cases(inventory)
            functional, exclusive = support.split_lanes(cases)
            self.assertEqual(calls, [
                ('production-devices-inventory', 'playwright.devices.config.ts', None, 1, None),
                ('production-devices-functional', 'playwright.devices.config.ts', 'functional', 4,
                 sorted(functional)),
                ('production-devices-exclusive', 'playwright.devices.config.ts', 'exclusive', 1,
                 sorted(exclusive)),
            ])
            self.assertEqual(run.receipt['production']['devices'],
                             {'total': 3, 'functional': 2, 'exclusive': 1})

    def test_production_configs_still_pin_one_worker_for_standalone_runs(self) -> None:
        import inspect
        from tools import run_web_gate as runner
        self.assertIn('run_production(run, app, env, config, args.workers)', inspect.getsource(runner.main))
        app = Path(__file__).resolve().parents[2] / 'apps/knx-web'
        for config in runner.PRODUCTION_CONFIGS:
            text = (app / f'playwright.{config}.config.ts').read_text()
            self.assertNotRegex(text, r'workers:\s*[2-9]', config)
            # A config-level grep would be replaced by the exclusive lane's --grep;
            # only reload-guard has one, and its inventory holds no exclusive case.
            if config != 'reload-guard':
                self.assertNotRegex(text, r'\bgrep\s*:', config)

    def test_lease_timeout_and_release_are_real(self) -> None:
        import fcntl
        from tools.run_web_gate import LOCKS, leases
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            run = support.MeasuredRun(root / 'attempt')
            with (root / '.git' / LOCKS[0]).open('a') as held:
                fcntl.flock(held, fcntl.LOCK_EX)
                with self.assertRaises(TimeoutError):
                    with leases(root, run, 0.02):
                        self.fail('must not bypass another lock holder')
            with leases(root, run, 1):
                self.assertGreaterEqual(run.receipt['queueWaitSeconds'], 0)
            # A new descriptor can take each lease after the context returns.
            for name in LOCKS:
                with (root / '.git' / name).open('a') as released:
                    fcntl.flock(released, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def test_interrupt_at_the_pid_checkpoint_still_terminates_the_child_group(self) -> None:
        """A signal right after the PID is recorded, before the wait, must not orphan."""
        import os
        import time

        class Interrupting(support.MeasuredRun):
            def save(self) -> None:
                super().save()
                if self.receipt['stages'] and self.receipt['stages'][-1].get('pid'):
                    raise KeyboardInterrupt

        with TemporaryDirectory() as tmp:
            run = Interrupting(Path(tmp) / 'attempt')
            with self.assertRaises(KeyboardInterrupt):
                run.step('sleep', [sys.executable, '-c', 'import time;time.sleep(60)'], Path.cwd())
            child = run.receipt['stages'][0]['pid']
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                try:
                    os.kill(child, 0)
                except ProcessLookupError:
                    break
                time.sleep(0.01)
            else:
                os.killpg(child, 9)
                self.fail('child group survived an interrupt at the PID checkpoint')

    def test_interrupt_does_not_signal_a_preexisting_child(self) -> None:
        import os
        import signal
        import time

        class Interrupting(support.MeasuredRun):
            def save(self) -> None:
                super().save()
                if self.receipt['stages'] and self.receipt['stages'][-1].get('pid'):
                    raise KeyboardInterrupt

        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            signal_marker = root / 'preexisting.signal'
            signal_ready = root / 'preexisting.ready'
            unrelated_program = f'''import ctypes
import signal
from pathlib import Path
signal.pthread_sigmask(signal.SIG_BLOCK, {{signal.SIGTERM}})
if ctypes.CDLL(None).prctl(4, 0, 0, 0, 0) != 0:
    raise OSError("PR_SET_DUMPABLE failed")
Path({str(signal_ready)!r}).touch()
while True:
    if signal.sigtimedwait({{signal.SIGTERM}}, .1) is not None:
        Path({str(signal_marker)!r}).write_text("SIGTERM")
'''
            unrelated = subprocess.Popen(
                [sys.executable, '-c', unrelated_program], start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not signal_ready.exists() and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue(signal_ready.exists(), 'pre-existing child did not arm its signal wait')
                fields = Path(f'/proc/{unrelated.pid}/stat').read_text().rsplit(')', 1)[1].split()
                identity = unrelated.pid, int(fields[19])
                run = Interrupting(Path(tmp) / 'attempt')
                with self.assertRaises(KeyboardInterrupt):
                    run.step('sleep', [sys.executable, '-c', 'import time;time.sleep(60)'], Path.cwd())
                self.assertIsNone(unrelated.poll(), 'stage cleanup claimed an unrelated child')
                self.assertFalse(signal_marker.exists(), 'stage cleanup signalled an unrelated child')
                fields = Path(f'/proc/{unrelated.pid}/stat').read_text().rsplit(')', 1)[1].split()
                self.assertEqual((unrelated.pid, int(fields[19])), identity)
            finally:
                if unrelated.poll() is None:
                    os.killpg(unrelated.pid, signal.SIGKILL)
                unrelated.wait()

    def test_normal_exit_cleans_detached_leftover_beside_a_preexisting_child(self) -> None:
        import os
        import signal
        import time
        unrelated = subprocess.Popen(
            [sys.executable, '-c',
             'import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)'],
            start_new_session=True)
        identity = None
        try:
            with TemporaryDirectory() as tmp:
                root = Path(tmp)
                marker = root / 'leftover.pid'
                temporary = root / 'leftover.tmp'
                descendant = (
                    'import os,signal,time;from pathlib import Path;'
                    'signal.signal(signal.SIGTERM,signal.SIG_IGN);'
                    "fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split();"
                    f'tmp=Path({str(temporary)!r});tmp.write_text(f"{{os.getpid()}} {{fields[19]}} ");'
                    f'os.replace(tmp,Path({str(marker)!r}));time.sleep(60)')
                child = ('import subprocess,sys,time;from pathlib import Path;'
                         f'subprocess.Popen([sys.executable,"-c",{descendant!r}],start_new_session=True);'
                         f'marker=Path({str(marker)!r});'
                         'deadline=time.monotonic()+5;'
                         'exec("while not marker.exists() and time.monotonic()<deadline:\\n time.sleep(.01)")')
                run = support.MeasuredRun(root / 'attempt')
                run.step('normal', [sys.executable, '-c', child], Path.cwd())
                identity = tuple(int(value) for value in marker.read_text().split())
                descendant_pid, start_ticks = identity
                try:
                    fields = Path(f'/proc/{descendant_pid}/stat').read_text().rsplit(')', 1)[1].split()
                except FileNotFoundError:
                    fields = None
                self.assertTrue(fields is None or int(fields[19]) != start_ticks,
                                'normal stage exit left its detached descendant alive')
                self.assertIsNone(unrelated.poll(), 'cleanup signalled the pre-existing child')
        finally:
            if unrelated.poll() is None:
                os.killpg(unrelated.pid, signal.SIGKILL)
            unrelated.wait()
            if identity is not None:
                descendant_pid, start_ticks = identity
                try:
                    fields = Path(f'/proc/{descendant_pid}/stat').read_text().rsplit(')', 1)[1].split()
                    if int(fields[19]) == start_ticks:
                        os.kill(descendant_pid, signal.SIGKILL)
                except (FileNotFoundError, ProcessLookupError):
                    pass

    def test_normal_exit_reaps_a_descendant_that_already_became_a_zombie(self) -> None:
        import os
        import time
        identity = None
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            marker = root / 'zombie.pid'
            temporary = root / 'zombie.tmp'
            descendant = (
                'import os;from pathlib import Path;'
                "fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split();"
                f'tmp=Path({str(temporary)!r});tmp.write_text(f"{{os.getpid()}} {{fields[19]}}");'
                f'os.replace(tmp,Path({str(marker)!r}))')
            child = (
                'import os,subprocess,sys,time;from pathlib import Path;'
                f'p=subprocess.Popen([sys.executable,"-c",{descendant!r}],start_new_session=True);'
                f'marker=Path({str(marker)!r});deadline=time.monotonic()+5;'
                'exec("while (not marker.exists() or '
                "Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split()[0] != 'Z') "
                'and time.monotonic()<deadline:\\n time.sleep(.01)");os._exit(0)')
            try:
                run = support.MeasuredRun(root / 'attempt')
                run.step('normal', [sys.executable, '-c', child], Path.cwd())
                identity = tuple(int(value) for value in marker.read_text().split())
                pid, start_ticks = identity
                info = support._process_info(pid)
                self.assertTrue(info is None or info.identity.start_ticks != start_ticks,
                                'stage cleanup left an owned zombie unreaped')
            finally:
                if identity is not None:
                    pid, start_ticks = identity
                    info = support._process_info(pid)
                    if info is not None and info.identity.start_ticks == start_ticks:
                        try:
                            os.waitpid(pid, os.WNOHANG)
                        except ChildProcessError:
                            pass

    def test_normal_exit_cleans_a_detached_descendant_with_a_scrubbed_environment(self) -> None:
        import os
        import signal
        identity = None
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            marker = root / 'scrubbed.pid'
            temporary = root / 'scrubbed.tmp'
            descendant = (
                'import os,signal,time;from pathlib import Path;'
                'signal.signal(signal.SIGTERM,signal.SIG_IGN);'
                "fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split();"
                f'tmp=Path({str(temporary)!r});tmp.write_text(f"{{os.getpid()}} {{fields[19]}}");'
                f'os.replace(tmp,Path({str(marker)!r}));time.sleep(60)')
            child = (
                'import subprocess,sys,time;from pathlib import Path;'
                f'subprocess.Popen([sys.executable,"-c",{descendant!r}],env={{}},start_new_session=True);'
                f'marker=Path({str(marker)!r});deadline=time.monotonic()+5;'
                'exec("while not marker.exists() and time.monotonic()<deadline:\\n time.sleep(.01)")')
            try:
                run = support.MeasuredRun(root / 'attempt')
                run.step('normal', [sys.executable, '-c', child], Path.cwd())
                identity = tuple(int(value) for value in marker.read_text().split())
                pid, start_ticks = identity
                info = support._process_info(pid)
                self.assertTrue(info is None or info.identity.start_ticks != start_ticks,
                                'stage cleanup left an environment-scrubbed descendant alive')
            finally:
                if identity is not None:
                    pid, start_ticks = identity
                    info = support._process_info(pid)
                    if info is not None and info.identity.start_ticks == start_ticks:
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    def test_normal_exit_does_not_claim_a_new_unrelated_child(self) -> None:
        import os
        import signal
        import threading
        import time
        errors = []
        unrelated = None
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            signal_marker = root / 'unrelated.signal'
            signal_ready = root / 'unrelated.ready'
            stage_release = root / 'stage.release'
            run = support.MeasuredRun(root / 'attempt')

            def stage() -> None:
                try:
                    stage_program = ('import time;from pathlib import Path;'
                                     f'p=Path({str(stage_release)!r});deadline=time.monotonic()+5;'
                                     'exec("while not p.exists() and time.monotonic()<deadline:\\n time.sleep(.01)")')
                    run.step('normal', [sys.executable, '-c', stage_program], Path.cwd())
                except BaseException as error:
                    errors.append(error)

            worker = threading.Thread(target=stage)
            worker.start()
            try:
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        data = json.loads((root / 'attempt/receipt.json').read_text())
                        if data['stages'] and data['stages'][0].get('pid'):
                            break
                    except (OSError, ValueError):
                        pass
                    time.sleep(.01)
                else:
                    self.fail('stage did not publish its PID')
                unrelated_program = f'''import signal
from pathlib import Path
signal.pthread_sigmask(signal.SIG_BLOCK, {{signal.SIGTERM}})
Path({str(signal_ready)!r}).touch()
while True:
    if signal.sigtimedwait({{signal.SIGTERM}}, .1) is not None:
        Path({str(signal_marker)!r}).write_text("SIGTERM")
'''
                unrelated = subprocess.Popen(
                    [sys.executable, '-c', unrelated_program],
                    start_new_session=True)
                deadline = time.monotonic() + 5
                while not signal_ready.exists() and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue(signal_ready.exists(), 'unrelated child did not arm its signal wait')
                stage_release.touch()
                worker.join(timeout=10)
                self.assertFalse(worker.is_alive(), 'stage cleanup did not finish')
                if errors:
                    raise errors[0]
                self.assertIsNone(unrelated.poll(), 'stage cleanup claimed a new unrelated child')
                self.assertFalse(signal_marker.exists(), 'stage cleanup signalled a new unrelated child')
            finally:
                if worker.is_alive():
                    worker.join(timeout=5)
                if unrelated is not None:
                    if unrelated.poll() is None:
                        os.killpg(unrelated.pid, signal.SIGKILL)
                    unrelated.wait()

    def test_sigterm_terminates_the_owned_child_group(self) -> None:
        import os
        import signal
        import time
        from tools import run_web_gate as runner
        self.assertTrue(hasattr(runner, 'interrupted'), 'SIGTERM cleanup handler is missing')
        with TemporaryDirectory() as tmp:
            output = Path(tmp) / 'attempt'
            program = ('import signal,sys;from pathlib import Path;'
                       'from tools.run_web_gate import interrupted;'
                       'from tools.web_gate_support import MeasuredRun;'
                       'signal.signal(signal.SIGTERM,interrupted);'
                       f'r=MeasuredRun(Path({str(output)!r}));'
                       'r.step("sleep",[sys.executable,"-c","import time;time.sleep(60)"],Path.cwd())')
            parent = subprocess.Popen([sys.executable, '-c', program],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                deadline = time.monotonic() + 5
                child = None
                while time.monotonic() < deadline:
                    receipt = output / 'receipt.json'
                    if receipt.exists():
                        data = json.loads(receipt.read_text())
                        if data['stages'] and data['stages'][0].get('pid'):
                            child = data['stages'][0]['pid']
                            break
                    time.sleep(0.01)
                self.assertIsNotNone(child, 'child did not start')
                parent.send_signal(signal.SIGTERM)
                parent.wait(timeout=5)
                with self.assertRaises(ProcessLookupError):
                    os.kill(child, 0)
                self.assertFalse(json.loads((output / 'receipt.json').read_text())['success'])
            finally:
                if parent.poll() is None:
                    parent.kill()
                    parent.wait()
    def test_npm_browser_entry_uses_the_measured_runner(self) -> None:
        root = Path(__file__).resolve().parents[2]
        scripts = json.loads((root / 'apps/knx-web/package.json').read_text())['scripts']
        self.assertEqual(scripts['test:e2e'], 'python3 ../../tools/run_web_gate.py --profile browser')
        self.assertEqual(scripts['test:gate'], 'python3 ../../tools/run_web_gate.py --profile web')

    def test_malformed_report_and_unsafe_stage_names_are_refused(self) -> None:
        with self.assertRaises(ValueError):
            support.browser_cases({'suites': [{'specs': None}], 'errors': []})
        with TemporaryDirectory() as tmp:
            run = support.MeasuredRun(Path(tmp) / 'attempt')
            with self.assertRaises(ValueError):
                run.step('../escape', [sys.executable, '-c', 'pass'], Path(tmp))
    def test_runtime_identity_refuses_missing_dependencies_and_binds_build_bytes(self) -> None:
        from tools import run_web_gate as runner
        self.assertTrue(hasattr(runner, 'runtime_identity'), 'Runtime prerequisite identity is missing')
        self.assertTrue(hasattr(runner, 'build_fingerprint'), 'Production build identity is missing')
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            with self.assertRaises((OSError, ValueError)):
                runner.runtime_identity(app)
            (app / 'dist').mkdir()
            (app / 'dist/index.html').write_text('actual build')
            before = runner.build_fingerprint(app)
            (app / 'dist/index.html').write_text('mutated build')
            self.assertNotEqual(runner.build_fingerprint(app), before)
    def test_default_workers_are_bounded_by_proven_counts_and_available_cpus(self) -> None:
        from tools import run_web_gate as runner
        self.assertTrue(hasattr(runner, 'default_workers'), 'Measured worker default is missing')
        for cpus, workers in ((1, 1), (2, 2), (3, 2), (4, 4), (16, 4)):
            self.assertEqual(runner.default_workers(cpus), workers)

    def test_ci_runs_the_gate_regressions(self) -> None:
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/ci.yml').read_text()
        self.assertIn('test_web_gate.py', workflow)
    def test_sigterm_also_cleans_descendants_that_ignore_termination(self) -> None:
        import os
        import signal
        import time
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / 'attempt'
            marker = root / 'descendant.pid'
            descendant = (f'import os,signal,time;from pathlib import Path;'
                          f'signal.signal(signal.SIGTERM,signal.SIG_IGN);'
                          f'Path({str(marker)!r}).touch();time.sleep(0.25);'
                          f'Path({str(marker)!r}).write_text(str(os.getpid()));time.sleep(60)')
            child = f'import subprocess,sys,time;subprocess.Popen([sys.executable,"-c",{descendant!r}]);time.sleep(60)'
            program = ('import signal,sys;from pathlib import Path;'
                       'from tools.run_web_gate import interrupted;'
                       'from tools.web_gate_support import MeasuredRun;'
                       'signal.signal(signal.SIGTERM,interrupted);'
                       f'r=MeasuredRun(Path({str(output)!r}));'
                       f'r.step("sleep",[sys.executable,"-c",{child!r}],Path.cwd())')
            parent = subprocess.Popen([sys.executable, '-c', program],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            grandchild = None
            try:
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        grandchild = int(marker.read_text())
                        break
                    except (OSError, ValueError):
                        pass
                    time.sleep(0.01)
                self.assertIsNotNone(grandchild, 'descendant did not publish a complete PID')
                self.assertGreater(grandchild, 1)
                parent.send_signal(signal.SIGTERM)
                parent.wait(timeout=5)
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    proc = Path(f'/proc/{grandchild}/stat')
                    if not proc.exists() or proc.read_text().split()[2] == 'Z':
                        break
                    time.sleep(0.01)
                else:
                    self.fail('termination-ignoring descendant survived owned-group cleanup')
            finally:
                if parent.poll() is None:
                    parent.kill()
                    parent.wait()
                if grandchild is not None:
                    try:
                        os.kill(grandchild, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_sigterm_cleans_descendant_in_a_new_session(self) -> None:
        import os
        import signal
        import time
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / 'attempt'
            marker = root / 'detached.pid'
            temporary = root / 'detached.tmp'
            descendant = (
                'import os,signal,time;from pathlib import Path;'
                'signal.signal(signal.SIGTERM,signal.SIG_IGN);'
                "fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split();"
                f'tmp=Path({str(temporary)!r});tmp.write_text(f"{{os.getpid()}} {{fields[19]}}");'
                f'os.replace(tmp,Path({str(marker)!r}));time.sleep(60)')
            child = ('import subprocess,sys,time;'
                     f'subprocess.Popen([sys.executable,"-c",{descendant!r}],start_new_session=True);'
                     'time.sleep(60)')
            program = ('import signal,sys;from pathlib import Path;'
                       'from tools.run_web_gate import interrupted;'
                       'from tools.web_gate_support import MeasuredRun;'
                       'signal.signal(signal.SIGTERM,interrupted);'
                       f'r=MeasuredRun(Path({str(output)!r}));'
                       f'r.step("sleep",[sys.executable,"-c",{child!r}],Path.cwd())')
            parent = subprocess.Popen([sys.executable, '-c', program],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            identity = None
            try:
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        identity = tuple(int(value) for value in marker.read_text().split())
                        if len(identity) == 2:
                            break
                    except (OSError, ValueError):
                        pass
                    time.sleep(0.01)
                self.assertIsNotNone(identity, 'detached descendant did not publish its identity')
                descendant_pid, start_ticks = identity
                parent.send_signal(signal.SIGTERM)
                parent.wait(timeout=10)
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    try:
                        fields = Path(f'/proc/{descendant_pid}/stat').read_text().rsplit(')', 1)[1].split()
                    except FileNotFoundError:
                        break
                    if int(fields[19]) != start_ticks:
                        break
                    time.sleep(0.01)
                else:
                    self.fail(f'detached descendant survived cleanup in state {fields[0]}')
            finally:
                if parent.poll() is None:
                    parent.kill()
                    parent.wait()
                if identity is not None:
                    descendant_pid, start_ticks = identity
                    try:
                        fields = Path(f'/proc/{descendant_pid}/stat').read_text().rsplit(')', 1)[1].split()
                        if int(fields[19]) == start_ticks:
                            os.kill(descendant_pid, signal.SIGKILL)
                    except (FileNotFoundError, ProcessLookupError):
                        pass


if __name__ == '__main__':
    unittest.main()
