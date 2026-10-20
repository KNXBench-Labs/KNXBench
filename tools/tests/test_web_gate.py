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
            # One serial stage per suite: the configs pin `workers: 1` (ADR-0144 keeps that).
            self.assertIn(f'production-{config}', plan)
        self.assertFalse([s for s in plan if s.startswith('production-') and s.endswith('-exclusive')])
        self.assertEqual(plan[-1], 'diff-check')
        benchmark = runner.stage_plan('benchmark', [])
        self.assertEqual(benchmark[0], 'browser-lanes')
        self.assertEqual(benchmark[-1], 'benchmark-6-w1')
        self.assertEqual(len(benchmark), 9)

    def test_targeted_plan_keeps_build_typechecks_and_vitest_but_only_selected_suites(self) -> None:
        from tools import run_web_gate as runner
        plan = runner.stage_plan('targeted', ['tsconfig.extra.json'], ('devices',), True)
        for stage in ('browser-lanes', 'build', 'tsconfig.extra', 'vitest', 'browser-inventory',
                      'browser-functional', 'browser-exclusive', 'production-devices'):
            self.assertIn(stage, plan)
        self.assertNotIn('production-devices-exclusive', plan)
        self.assertFalse([s for s in plan if s.startswith('production-') and 'devices' not in s])
        for stage in ('documentation', 'tool-tests', 'check-layering', 'diff-check'):
            self.assertNotIn(stage, plan)
        quiet = runner.stage_plan('targeted', [], (), False)
        self.assertEqual(quiet, ['browser-lanes', 'build', 'vitest'])

    def test_production_suites_run_with_the_one_worker_their_configs_pin(self) -> None:
        import inspect
        from tools import run_web_gate as runner
        source = inspect.getsource(runner.main)
        self.assertIn("browser_stage(run, app, env, name, file, None, 1, expected)", source)
        app = Path(__file__).resolve().parents[2] / 'apps/knx-web'
        for config in runner.PRODUCTION_CONFIGS:
            text = (app / f'playwright.{config}.config.ts').read_text()
            self.assertNotRegex(text, r'workers:\s*[2-9]', config)

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
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(marker.exists())
                grandchild = int(marker.read_text())
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


if __name__ == '__main__':
    unittest.main()
