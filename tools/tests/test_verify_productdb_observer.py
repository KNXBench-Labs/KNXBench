"""Maintained opt-in observer controls; ordinary tests need no private helper."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import time
from unittest.mock import patch
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CLI = ROOT / 'tools/verify_productdb_observer.py'


class ObserverControlTests(unittest.TestCase):
    def test_maintained_control_api_exists(self):
        self.assertIsNotNone(importlib.util.find_spec('tools.verify_productdb_observer'),
                             'maintained observer calibration API is missing')
        from tools import verify_productdb_observer as control
        self.assertTrue(callable(control.main))
        self.assertTrue(callable(control.validate_calibration))

    def invoke(self, *args):
        return subprocess.run([sys.executable, str(CLI), *map(str, args)],
                              cwd=ROOT, capture_output=True, timeout=15)

    def test_identity_streams_bytes_without_polluting_calibration_high_water(self):
        from hashlib import sha256
        from unittest.mock import patch
        from tools import verify_productdb_observer as control
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'identity-input'
            path.write_bytes(b'identity-control' * 100000)
            expected = sha256(path.read_bytes()).hexdigest()
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('must stream identity')):
                self.assertEqual(control.identity(path), {'path': str(path), 'sha256': expected})

    def test_real_cli_refuses_paths_without_writing_or_disclosing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            sentinel = root / 'existing'
            sentinel.mkdir()
            (sentinel / 'sentinel').write_text('untouched')
            link = root / 'link'
            link.symlink_to(sentinel, target_is_directory=True)
            dangling = root / 'dangling'
            dangling.symlink_to(root / 'absent', target_is_directory=True)
            helper = root / 'helper'
            helper.symlink_to('/usr/bin/true')
            (root / 'helper-directory').mkdir()
            script = root / 'helper-directory/script'
            script.write_text('#!/usr/bin/python3\n')
            script.chmod(0o700)
            cases = [[], ['--supervisor', '/usr/bin/true'],
                     ['--supervisor', 'relative', '--output', root / 'new'],
                     ['--supervisor', '/usr/bin/true', '--output', 'relative'],
                     ['--supervisor', '/usr/bin/true', '--output', sentinel],
                     ['--supervisor', '/usr/bin/true', '--output', link],
                     ['--supervisor', '/usr/bin/true', '--output', dangling],
                     ['--supervisor', '/usr/bin/true', '--output', link / 'new'],
                     ['--supervisor', helper, '--output', root / 'new'],
                     ['--supervisor', script, '--output', root / 'new'],
                     ['--supervisor', '/usr/bin/true', '--output', ROOT / 'observer-control-refused'],
                     ['--supervisor', '/usr/bin/true', '--output', '/usr/bin/observer-control-refused'],
                     ['--supervisor', '/usr/bin/true', '--output', root / 'missing/new']]
            for args in cases:
                with self.subTest(args=args):
                    result = self.invoke(*args)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, b'')
                    self.assertEqual(result.stderr, b'observer controls refused or failed\n')
            self.assertEqual((sentinel / 'sentinel').read_text(), 'untouched')
            self.assertFalse((root / 'new').exists())
            self.assertFalse((root / 'absent').exists())
            self.assertFalse((ROOT / 'observer-control-refused').exists())

    def test_real_cli_rejects_elf_nonobserver_without_crediting_wrapper_metrics(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp).resolve() / 'rejected-protocol'
            result = self.invoke('--supervisor', '/usr/bin/true', '--output', output)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, b'')
            self.assertEqual(result.stderr, b'observer controls refused or failed\n')
            self.assertEqual(output.stat().st_mode & 0o777, 0o700)
            proof = json.loads((output / 'proof.json').read_text())
            self.assertIs(proof['success'], False)
            self.assertEqual(proof['helperStart'], proof['helperEnd'])
            target = json.loads((output / 'calibration/results.json').read_text())['fresh']['target']
            self.assertIsNone(target['peakRssKiB'])
            self.assertIsNotNone(target['observerError'])

    def test_calibration_requires_real_growth_and_discriminates_wrapper(self):
        from tools import verify_productdb_observer as control
        # Threshold fixtures only, never emitted as native/integration evidence.
        data = {name: {'target': {'peakRssKiB': 2048}, 'wrapper': {'peakRssKiB': wrapper}}
                for name, wrapper in [('fresh', 20000), ('held64MiB', 90000),
                                      ('freed64MiB', 90000), ('afterHeavyChild', 90000)]}
        data['heavyTarget'] = {'target': {'peakRssKiB': 80000}, 'wrapper': {'peakRssKiB': 90000}}
        control.validate_calibration(data)
        for name, kind, peak in [('held64MiB', 'target', 20000),
                                 ('heavyTarget', 'target', 40000),
                                 ('held64MiB', 'wrapper', 60000),
                                 ('freed64MiB', 'wrapper', 60000),
                                 ('afterHeavyChild', 'target', 11000)]:
            bad = copy.deepcopy(data)
            bad[name][kind]['peakRssKiB'] = peak
            with self.subTest(name=name, kind=kind), self.assertRaises(ValueError):
                control.validate_calibration(bad)

    def policy(self, mode=None, changed_source='verify_productdb_observer.py'):
        """Synthetic identity/write policy only; never native/process evidence."""
        from contextlib import redirect_stderr, redirect_stdout
        from io import StringIO
        from tools import verify_productdb_observer as control
        original_identity, original_write = control.identity, control.write
        calls, writes = {}, []

        def identity(path):
            value = original_identity(path)
            calls[path] = calls.get(path, 0) + 1
            drift = ((mode == 'source-drift' and path.name == changed_source) or
                     (mode == 'helper-drift' and path == Path('/usr/bin/true')))
            if drift and calls[path] > 1:
                value['sha256'] = '0' * 64  # Clearly synthetic end-identity seam.
            return value

        def write(path, value):
            writes.append(copy.deepcopy(value))
            if mode == 'persistence' and value.get('success'):
                raise OSError('synthetic persistence refusal')
            return original_write(path, value)

        def controller(name, *_args):
            return {'name': name, 'exit': 0, 'hardTimeout': False}

        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp).resolve() / 'policy'
            stdout, stderr = StringIO(), StringIO()
            with patch.object(control, 'identity', identity), patch.object(control, 'write', write), \
                    patch.object(control, 'run_controller', controller, create=True), \
                    patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0)), \
                    redirect_stdout(stdout), redirect_stderr(stderr):
                code = control.main(['--supervisor', '/usr/bin/true', '--output', str(output)])
            return code, read_json(output / 'proof.json'), writes, stdout.getvalue(), stderr.getvalue()

    def test_explicit_executing_source_manifest_is_required_for_success(self):
        code, proof, _, _, _ = self.policy()
        self.assertEqual(code, 0)
        self.assertIn('sourceStart', proof)
        self.assertEqual(set(proof['sourceStart']), {'verify_productdb_observer.py',
                         'productdb_process_observer.py', 'web_gate_support.py'})
        self.assertEqual(proof['sourceStart'], proof['sourceEnd'])
        self.assertIs(proof['success'], True)

    def test_source_drift_refused_before_any_persisted_success(self):
        for name in ('verify_productdb_observer.py', 'productdb_process_observer.py', 'web_gate_support.py'):
            with self.subTest(source=name):
                code, proof, writes, stdout, stderr = self.policy('source-drift', name)
                self.assertEqual(code, 1, 'source drift was credited as success')
                self.assertIs(proof['success'], False)
                self.assertNotEqual(proof['sourceStart'], proof['sourceEnd'])
                self.assertFalse(any(value['success'] for value in writes))
                self.assertEqual(stdout, '')
                self.assertEqual(stderr, 'observer controls refused or failed\n')

    def test_helper_drift_remains_fail_closed(self):
        code, proof, _, stdout, stderr = self.policy('helper-drift')
        self.assertEqual(code, 1)
        self.assertIs(proof['success'], False)
        self.assertNotEqual(proof['helperStart'], proof['helperEnd'])
        self.assertEqual(stdout, '')
        self.assertEqual(stderr, 'observer controls refused or failed\n')

    def test_final_success_persistence_failure_returns_failure(self):
        code, proof, _, stdout, stderr = self.policy('persistence')
        self.assertEqual(code, 1)
        self.assertIs(proof['success'], False)
        self.assertEqual(stdout, '')
        self.assertEqual(stderr, 'observer controls refused or failed\n')

    def test_final_adapter_proof_validation_and_signal_mapping_are_not_reimplemented(self):
        from tools import verify_productdb_observer as control
        from tools.productdb_process_observer import target_observation
        self.assertIs(control.observed_step, __import__('tools.productdb_process_observer',
                                                       fromlist=['observed_step']).observed_step)
        with tempfile.TemporaryDirectory() as tmp:
            receipt = Path(tmp) / 'target.json'
            wrapper = {'pid': 71, 'exit': 143, 'peakRssKiB': 999999,
                       'cpuSeconds': 50, 'wallSeconds': 60}
            proof = {'format': 1, 'complete': True, 'pid': 72, 'exit': -15,
                     'signal': 15, 'waitStatus': signal.SIGTERM, 'peakRssKiB': 2048,
                     'cpuSeconds': 0, 'elapsedSeconds': 0.01}
            receipt.write_text(json.dumps(proof))
            actual = target_observation(wrapper, receipt)
            self.assertEqual(actual['exit'], -15)
            self.assertEqual(actual['pid'], 72)
            self.assertEqual(actual['peakRssKiB'], 2048)
            for change in [{'complete': False}, {'pid': 71}, {'exit': 143},
                           {'waitStatus': 0}, {'peakRssKiB': 0}, {'cpuSeconds': float('nan')}]:
                receipt.write_text(json.dumps({**proof, **change}))
                with self.subTest(change=change), self.assertRaises(ValueError):
                    target_observation(wrapper, receipt)


# This selects an opt-in test binary; no helper/runtime fault-injection API exists.
NATIVE_HELPER = os.environ.get('KNXBENCH_OBSERVER_TEST_HELPER')


def read_json(path):
    return json.loads(path.read_text())


def native_controller(helper, output, mode):
    """Private scoped Popen fixtures in a disposable controller, never the suite."""
    from tools import verify_productdb_observer as control
    original = subprocess.Popen
    children = []
    signals = []
    gap = mode == 'checkpoint-gap'
    marker = output / 'gap-owned.json'
    unrelated = {}

    def process(pid):
        root = Path(f'/proc/{pid}')
        fields = (root / 'stat').read_text().rsplit(')', 1)[1].split()
        return dict(pid=pid, ppid=int(fields[1]), pgid=int(fields[2]), sid=int(fields[3]),
                    startTicks=int(fields[19]), cwd=str((root / 'cwd').resolve()),
                    command=(root / 'cmdline').read_bytes().rstrip(b'\0').decode().split('\0'))

    def popen(command, *args, **kwargs):
        if command[0] == str(helper) and mode == 'term-ignore':
            kwargs['preexec_fn'] = lambda: signal.signal(signal.SIGTERM, signal.SIG_IGN)
        if mode in ('deadline', 'hard-timeout', 'checkpoint-gap') and '-c' in command:
            command = list(command)
            index = command.index('-c') + 1
            if 'tools.verify_productdb_observer' in command[index]:
                # Scoped controller fixture, not an environment knob in runtime APIs.
                seconds = 1 if mode == 'deadline' else 0
                prefix = ('import tools.verify_productdb_observer as c; '
                          f'c.TARGET=c.TREE; c.CONTROLLER_SECONDS={seconds}; ')
                if gap:
                    # Same actual delayed MeasuredRun.save seam as the review probe:
                    # real spawn, untouched on-disk PID-less stage/native checkpoint.
                    prefix = '''from pathlib import Path
import json,sys,time
from tools import verify_productdb_observer as c
from tools.web_gate_support import MeasuredRun
original_save=MeasuredRun.save
def save(self):
    if self.receipt['stages']:
        stage=self.receipt['stages'][-1]
        if stage['name']=='heavyTarget' and type(stage.get('pid')) is int:
            Path(sys.argv[3]).parents[1].joinpath('gap-owned.json').write_text(json.dumps(stage))
            time.sleep(12)
    return original_save(self)
MeasuredRun.save=save
c.TARGET=c.TREE
c.CONTROLLER_SECONDS=0
'''
                command[index] = prefix + command[index]
        child = original(command, *args, **kwargs)
        children.append(child)
        if gap:
            # Readiness outside the controller makes the widened window deterministic.
            until = time.monotonic() + 5
            stage = None
            while time.monotonic() < until:
                try:
                    stage = read_json(marker)
                    break
                except (OSError, ValueError):
                    time.sleep(.01)
            assert stage is not None, 'delayed PID save was not reached'
            until = time.monotonic() + 5
            log = output / 'control/calibration/heavyTarget.log'
            checkpoint = log.with_suffix('.target.json')
            while time.monotonic() < until:
                try:
                    read_json(log)
                    read_json(checkpoint)
                    break
                except (OSError, ValueError):
                    time.sleep(.01)
            # A second direct child has the same helper/target/cwd but a different
            # full receipt path; matching the executable alone must not signal it.
            distractor = [*stage['command']]
            distractor[2] = str(output / 'unrelated.target.json')
            with (output / 'unrelated.log').open('wb') as stream:
                other = original(distractor, cwd=stage['cwd'], stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
            children.append(other)
            unrelated.update(pid=other.pid, command=distractor)
            (output / 'unrelated-owned.json').write_text(json.dumps(unrelated))
            before = process(stage['pid'])
            assert before['ppid'] == child.pid
            assert before['command'] == stage['command']
            assert before['pgid'] == before['sid'] == before['pid']
            assert 'pid' not in read_json(log.parent / 'receipt.json')['stages'][-1]
            os.kill(child.pid, signal.SIGSTOP)
            until = time.monotonic() + 1
            while time.monotonic() < until:
                state = Path(f'/proc/{child.pid}/stat').read_text().rsplit(')', 1)[1].split()[0]
                if state == 'T':
                    break
                time.sleep(.01)
            assert state == 'T', 'controller did not stop in the PID-less window'
            before['controllerState'] = state
            (output / 'before-controller-stop.json').write_text(json.dumps(before))
        return child

    original_killpg = os.killpg

    def killpg(group, signum):
        entry = dict(group=group, signal=signum)
        if gap and signum == signal.SIGKILL:
            entry['leader'] = process(group)
            entry['ownerPid'] = os.getpid()
            entry['directChildren'] = Path(f'/proc/self/task/{os.getpid()}/children').read_text().split()
        signals.append(entry)
        return original_killpg(group, signum)

    original_run = subprocess.run

    def short_legacy_timeout(*args, **kwargs):
        # Exercise the old subprocess.run path on RED; unused by the fixed CLI.
        kwargs['timeout'] = 2
        return original_run(*args, **kwargs)

    def capture():
        result = {}
        try:
            with patch.object(subprocess, 'Popen', popen), \
                    patch.object(os, 'killpg', killpg), \
                    patch.object(subprocess, 'run', short_legacy_timeout):
                if mode == 'term-ignore':
                    control.cancellation(helper, output / 'control')
                else:
                    if mode in ('deadline', 'hard-timeout', 'checkpoint-gap'):
                        control.CONTROLLER_HARD_SECONDS = 1.5 if gap else 3
                    result['exit'] = control.main(['--supervisor', str(helper),
                                                  '--output', str(output / 'control')])
        except BaseException as error:
            result['error'] = type(error).__name__
        else:
            result['error'] = None
        # Snapshot BEFORE the independent owner finally cleans anything.
        result['children'] = [child.pid for child in children]
        if mode == 'term-ignore':
            result['control'] = read_json(output / 'control/results.json')
            pids = result['control']['pids']
        elif mode == 'inventory':
            result['proof'] = read_json(output / 'control/proof.json')
            pids = result['children'][:]
            for receipt in (output / 'control').rglob('receipt.json'):
                pids.extend(row['pid'] for row in read_json(receipt)['stages'] if 'pid' in row)
            for receipt in (output / 'control').rglob('*.target.json'):
                pids.append(read_json(receipt)['pid'])
            for log in (output / 'control').rglob('tree.log'):
                tree = read_json(log)
                pids.extend([tree['pid'], tree['child']])
        else:
            result['proof'] = read_json(output / 'control/proof.json')
            result['receipt'] = read_json(output / 'control/calibration/receipt.json')
            result['checkpoint'] = read_json(output / 'control/calibration/heavyTarget.target.json')
            result['tree'] = read_json(output / 'control/calibration/heavyTarget.log')
            group = read_json(marker)['pid'] if gap else result['receipt']['stages'][-1]['pid']
            pids = [group, result['tree']['pid'], result['tree']['child'], children[0].pid]
            if gap:
                result['beforeStop'] = read_json(output / 'before-controller-stop.json')
                result['signals'] = signals
                result['unrelatedAtReturn'] = process(unrelated['pid'])
                result['unrelatedExpected'] = unrelated
        result['pids'] = sorted(set(pids))
        result['liveAtReturn'] = [pid for pid in result['pids'] if Path(f'/proc/{pid}').exists()]
        (output / 'snapshot.json').write_text(json.dumps(result))

    try:
        capture()
    finally:
        # Snapshot first; local Popen ownership still gets independent cleanup.
        # The outer isolated subreaper is the hard failsafe for the whole tree.
        for child in children:
            if child.poll() is None:
                if os.getpgid(child.pid) == child.pid:
                    os.killpg(child.pid, signal.SIGKILL)
                else:
                    child.kill()
            child.wait(timeout=5)


def native_owner(helper, output, mode):
    """Hard failsafe owns/reaps exact fixture groups even against broken source."""
    import ctypes
    libc = ctypes.CDLL(None)
    assert libc.prctl(36, 1, 0, 0, 0) == 0  # Isolated owner only.
    command = [sys.executable, '-B', '-m', 'tools.tests.test_verify_productdb_observer',
               '--native-controller', str(helper), str(output), mode]
    controller = subprocess.Popen(command, cwd=ROOT, start_new_session=True)
    groups, pids, reaped = set(), {controller.pid}, []
    try:
        controller.wait(timeout=20)
    finally:
        # Only exact MeasuredRun receipt checkpoints from this new private tree.
        for receipt in output.rglob('receipt.json'):
            for stage in read_json(receipt)['stages']:
                if type(stage.get('pid')) is int and stage['command'][0] == str(helper):
                    groups.add(stage['pid'])
                    pids.add(stage['pid'])
        # The gap fixture's separate owner markers are NOT runtime inputs.
        # They allow the original-source RED to fail without abandoning processes.
        for marker in (output / 'gap-owned.json', output / 'unrelated-owned.json'):
            if marker.exists():
                owned = read_json(marker)
                group = owned['pid']
                assert type(group) is int and group > 1 and owned['command'][0] == str(helper)
                if Path(f'/proc/{group}').exists():
                    assert os.getpgid(group) == os.getsid(group) == group
                groups.add(group)
                pids.add(group)
        for receipt in output.rglob('*.target.json'):
            pids.add(read_json(receipt)['pid'])
        for log in output.rglob('*.log'):
            for line in log.read_text().splitlines():
                try:
                    value = json.loads(line)
                except ValueError:
                    continue
                if isinstance(value, dict):
                    pids.update(value[k] for k in ('pid', 'child') if type(value.get(k)) is int)
        for group in groups | {controller.pid}:
            try:
                os.killpg(group, signal.SIGKILL)
            except ProcessLookupError:
                pass
        controller.wait(timeout=5)
        for group in groups:
            until = time.monotonic() + 5
            while time.monotonic() < until:
                try:
                    pid, status = os.waitpid(-group, os.WNOHANG)
                except ChildProcessError:
                    break
                if pid:
                    reaped.append({'pid': pid, 'waitStatus': status})
                else:
                    time.sleep(.01)
        cleanup = {'controllerPid': controller.pid, 'controllerExit': controller.returncode,
                   'pids': sorted(pids), 'reaped': reaped,
                   'survivors': [pid for pid in pids if Path(f'/proc/{pid}').exists()]}
        (output / 'owner-cleanup.json').write_text(json.dumps(cleanup))
        assert not cleanup['survivors'], cleanup


@unittest.skipUnless(sys.platform == 'linux' and NATIVE_HELPER,
                     'opt-in existing native helper required; no build')
class NativeObserverControlTests(unittest.TestCase):
    def fixture(self, mode):
        # Opt-in attempts retain raw evidence in a new private directory.
        base = os.environ.get('KNXBENCH_OBSERVER_TEST_EVIDENCE')
        directory = Path(tempfile.mkdtemp(prefix=mode + '-', dir=base)).resolve()
        assert NATIVE_HELPER is not None
        result = subprocess.run([sys.executable, '-B', '-m',
                                 'tools.tests.test_verify_productdb_observer', '--native-owner',
                                 str(Path(NATIVE_HELPER).resolve()), str(directory), mode],
                                cwd=ROOT, capture_output=True, timeout=35)
        (directory / 'owner.stdout').write_bytes(result.stdout)
        (directory / 'owner.stderr').write_bytes(result.stderr)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        cleanup = read_json(directory / 'owner-cleanup.json')
        self.assertEqual(cleanup['survivors'], [])
        return directory, read_json(directory / 'snapshot.json')

    def test_cli_inventory_requires_actual_echild_and_failure_controls(self):
        directory, snapshot = self.fixture('inventory')
        self.assertEqual(snapshot['exit'], 0)
        proof = snapshot['proof']
        self.assertEqual([row['name'] for row in proof['controllers']],
                         ['calibration', 'cancellation', 'term_ignore', 'timeout', 'wait_failure'])
        self.assertTrue(proof['success'])
        self.assertEqual(proof['sourceStart'], proof['sourceEnd'])
        self.assertEqual(proof['helperStart'], proof['helperEnd'])
        self.assertEqual(snapshot['liveAtReturn'], [])
        failure = read_json(directory / 'control/wait_failure/results.json')
        emitted = json.loads((directory / 'control/wait_failure/echild.log').read_text().splitlines()[0])
        self.assertEqual(failure['wrapper']['exit'], 125)
        self.assertEqual(failure['checkpoint'], {'format': 1, 'complete': False, 'pid': emitted['pid']})
        self.assertNotEqual(emitted['pid'], failure['wrapper']['pid'])
        self.assertIsNotNone(failure['target']['observerError'])
        for key in ('pid', 'exit', 'waitStatus', 'signal', 'peakRssKiB', 'cpuSeconds', 'wallSeconds'):
            self.assertIsNone(failure['target'][key])
        self.assertEqual(failure['pids'], failure['absentPids'])
        for name in ('term_ignore', 'timeout'):
            path = directory / 'control' / name
            result = read_json(path / 'results.json')
            receipt = read_json(path / 'receipt.json')
            self.assertEqual(result['pids'], result['absentPids'])
            self.assertIs(receipt['success'], False)
            self.assertIs(receipt['complete'], False)
            self.assertIsNone(receipt['stages'][0]['exit'])
            self.assertFalse(read_json(path / 'tree.target.json')['complete'])
        self.assertTrue(read_json(directory / 'control/timeout/results.json')['timedOut'])

    def assert_timeout(self, mode):
        directory, snapshot = self.fixture(mode)
        self.assertIsNone(snapshot['error'])
        self.assertEqual(snapshot['exit'], 1)
        self.assertIs(snapshot['proof']['success'], False)
        self.assertIs(snapshot['receipt']['complete'], False)
        self.assertIs(snapshot['receipt']['success'], False)
        self.assertEqual(snapshot['checkpoint'],
                         {'format': 1, 'complete': False, 'pid': snapshot['tree']['pid']})
        self.assertIsNone(snapshot['receipt']['stages'][-1]['exit'])
        self.assertEqual(snapshot['liveAtReturn'], [], snapshot)
        self.assertTrue(all(not Path(f'/proc/{pid}').exists() for pid in snapshot['pids']))
        if mode == 'deadline':
            self.assertIn('TimeoutError', (directory / 'control/calibration-controller.log').read_text())
            self.assertFalse(snapshot['proof']['controllers'][0]['hardTimeout'])
        else:
            self.assertTrue(snapshot['proof']['controllers'][0]['hardTimeout'])
            self.assertEqual({row['pid'] for row in snapshot['proof']['controllers'][0]['reaped']},
                             {snapshot['receipt']['stages'][-1]['pid'], snapshot['tree']['pid'],
                              snapshot['tree']['child']})

    def test_actual_hanging_target_deadline_runs_measured_cleanup(self):
        self.assert_timeout('deadline')

    def test_outer_hard_timeout_owns_and_reaps_checkpointed_group(self):
        self.assert_timeout('hard-timeout')

    def test_outer_hard_timeout_recovers_spawn_before_pid_checkpoint(self):
        directory, snapshot = self.fixture('checkpoint-gap')
        self.assertIsNone(snapshot['error'], snapshot)
        self.assertEqual(snapshot['exit'], 1)
        self.assertIs(snapshot['proof']['success'], False)
        self.assertNotIn('pid', snapshot['receipt']['stages'][-1])
        self.assertIsNone(snapshot['receipt']['stages'][-1]['exit'])
        self.assertEqual(snapshot['checkpoint'],
                         {'format': 1, 'complete': False, 'pid': snapshot['tree']['pid']})
        # This assertion precedes and cannot be satisfied by the owner's finally.
        self.assertEqual(snapshot['liveAtReturn'], [], 'spawned group survived maintained fallback')
        before = snapshot['beforeStop']
        self.assertEqual(before['controllerState'], 'T')
        kills = [row for row in snapshot['signals'] if row['signal'] == signal.SIGKILL]
        self.assertEqual(len(kills), 1)
        kill = kills[0]
        leader = kill['leader']
        self.assertEqual(kill['group'], before['pid'])
        self.assertEqual(leader['startTicks'], before['startTicks'])
        self.assertEqual(leader['command'], snapshot['receipt']['stages'][-1]['command'])
        self.assertEqual(leader['cwd'], snapshot['receipt']['stages'][-1]['cwd'])
        self.assertEqual(leader['pgid'], leader['pid'])
        self.assertEqual(leader['sid'], leader['pid'])
        self.assertEqual(leader['ppid'], kill['ownerPid'])  # Adopted after controller kill/wait.
        self.assertIn(str(leader['pid']), kill['directChildren'])
        self.assertNotEqual(before['ppid'], leader['ppid'])
        self.assertEqual(snapshot['unrelatedAtReturn']['command'], snapshot['unrelatedExpected']['command'])
        self.assertEqual({row['group'] for row in snapshot['signals']}, {leader['pid']})
        self.assertTrue(snapshot['proof']['controllers'][0]['hardTimeout'])
        self.assertEqual({row['pid'] for row in snapshot['proof']['controllers'][0]['reaped']},
                         {before['pid'], snapshot['tree']['pid'], snapshot['tree']['child']})
        self.assertEqual(read_json(directory / 'owner-cleanup.json')['survivors'], [])

    def test_actual_term_ignore_escalation_is_one_shot_and_reaps_tree(self):
        _, snapshot = self.fixture('term-ignore')
        self.assertIsNone(snapshot['error'], snapshot)
        self.assertEqual(snapshot['liveAtReturn'], [])
        proof = snapshot['control']
        self.assertEqual(proof['pids'], proof['absentPids'])
        self.assertEqual({r['pid'] for r in proof['reaped']},
                         {proof['targetTree']['pid'], proof['targetTree']['child']})
        self.assertEqual({r['waitStatus'] for r in proof['reaped']}, {signal.SIGKILL})


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] in ('--native-owner', '--native-controller'):
        action, helper, output, mode = sys.argv[1:]
        (native_owner if action == '--native-owner' else native_controller)(
            Path(helper), Path(output), mode)
    else:
        unittest.main()
