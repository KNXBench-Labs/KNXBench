"""Regress termination signals that land while browser_tmp spawns its child group."""
from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock

from tools import browser_tmp
from tools.tests.test_browser_tmp_launch import (
    close_owned_processes, open_owned_process, process_state, require_pidfd_cleanup,
    signal_owned_processes, wait_for_owned_processes)

ROOT = Path(__file__).resolve().parents[2]
FORWARDED = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)
FAKE_GROUP = 4242
# The driver runs browser_tmp.run_command with a chosen starting disposition and sends itself the
# signal synchronously at the spawn boundary: after the child exists, before Popen returns.
DRIVER = '''
import os
import select
import signal
import subprocess
import sys
from tools import browser_tmp

name, disposition, workdir, marker, receipt = sys.argv[1:6]
signum = getattr(signal, name)
signal.signal(signum, {"dfl": signal.SIG_DFL, "ign": signal.SIG_IGN,
                       "pydefault": signal.default_int_handler}[disposition])
real_popen = subprocess.Popen

def spawn(*args, **kwargs):
    ready_read, ready_write = os.pipe()
    kwargs["env"] = {**kwargs["env"], "KNXBENCH_SPAWN_MARKER": marker,
                     "KNXBENCH_SPAWN_RECEIPT": receipt, "KNXBENCH_SPAWN_READY_FD": str(ready_write)}
    kwargs["pass_fds"] = (ready_write,)
    child = real_popen(*args, **kwargs)
    os.close(ready_write)
    readable, _, _ = select.select([ready_read], [], [], 30)
    line = os.read(ready_read, 16) if readable else b""
    os.close(ready_read)
    print("child", child.pid, line.strip().decode(), flush=True)
    os.kill(os.getpid(), signum)
    return child

subprocess.Popen = spawn
status = browser_tmp.run_command([sys.executable, "-c", sys.argv[6]], __import__("pathlib").Path(workdir))
print("status", status, flush=True)
'''
# Installs its own handlers (an inherited SIG_IGN would survive exec), reports ready, then waits.
CHILD = '''
import os
from pathlib import Path
import signal
import sys
import time

receipt = Path(os.environ["KNXBENCH_SPAWN_RECEIPT"])

def received(signum, frame):
    receipt.write_text(str(signum))
    sys.exit(7)

for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
    signal.signal(signum, received)
os.write(int(os.environ["KNXBENCH_SPAWN_READY_FD"]), b"ready\\n")
deadline = time.monotonic() + 30
while time.monotonic() < deadline:
    time.sleep(0.05)
sys.exit(9)
'''


class FakeChild:
    """The part of Popen that run_command touches, with no process behind it."""

    pid = FAKE_GROUP

    def wait(self) -> int:
        return 0

    def poll(self) -> int | None:
        return 0


def check_pending_replay(module) -> list[str]:
    """Problems found when SIGTERM/INT/HUP arrive inside Popen, before a child identity exists."""
    problems = []
    for signum in FORWARDED:
        received: list[int] = []
        killed: list[tuple[int, int]] = []
        seen_while_spawning: list[tuple[int, int]] = []
        previous = signal.signal(signum, lambda number, frame: received.append(number))

        def spawn(*args, **kwargs):
            os.kill(os.getpid(), signum)
            seen_while_spawning.extend(killed)
            return FakeChild()

        try:
            with mock.patch.object(module.subprocess, 'Popen', spawn), \
                    mock.patch.object(module.os, 'killpg', lambda group, number: killed.append((group, number))):
                module.run_command(['unused'], Path('.'))
        finally:
            signal.signal(signum, previous)
        name = signal.Signals(signum).name
        if received:
            problems.append(f'{name} reached the caller handler instead of the wrapper')
        if seen_while_spawning:
            problems.append(f'{name} was sent to a group before the child identity existed')
        if killed != [(FAKE_GROUP, signum)]:
            problems.append(f'{name} forwarded as {killed}, expected [({FAKE_GROUP}, {signum})]')
    return problems


class SpawnSignalContract(unittest.TestCase):
    def recorders(self):
        """Distinct caller handlers for the three signals, so a lost forward is observable."""
        previous = {number: signal.signal(number, lambda *_: None) for number in FORWARDED}
        self.addCleanup(lambda: [signal.signal(number, old) for number, old in previous.items()])
        return {number: signal.getsignal(number) for number in FORWARDED}

    def test_signals_during_spawn_wait_for_the_child_identity_then_reach_the_group(self) -> None:
        self.assertEqual(check_pending_replay(browser_tmp), [])

    def test_contract_rejects_a_wrapper_that_drops_the_pending_replay(self) -> None:
        source = Path(browser_tmp.__file__).read_text()
        replay = '        for signum in pending:\n            forward(signum, None)\n'
        self.assertEqual(source.count(replay), 1, 'mutation target moved; update the calibration')
        mutant = types.ModuleType('browser_tmp_mutant')
        exec(compile(source.replace(replay, '        pass\n'), 'browser_tmp_mutant', 'exec'), mutant.__dict__)
        problems = check_pending_replay(mutant)
        self.assertEqual(len(problems), len(FORWARDED), problems)
        self.assertTrue(all('forwarded as []' in problem for problem in problems), problems)

    def test_handlers_return_to_the_caller_after_a_run_and_after_a_failed_spawn(self) -> None:
        installed = self.recorders()

        def refuse(*args, **kwargs):
            os.kill(os.getpid(), signal.SIGTERM)
            raise OSError('spawn refused')

        with mock.patch.object(browser_tmp.subprocess, 'Popen', lambda *a, **k: FakeChild()):
            browser_tmp.run_command(['unused'], Path('.'))
        self.assertEqual({number: signal.getsignal(number) for number in FORWARDED}, installed)
        with mock.patch.object(browser_tmp.subprocess, 'Popen', refuse), \
                mock.patch.object(browser_tmp.os, 'killpg') as killpg:
            with self.assertRaises(OSError):
                browser_tmp.run_command(['unused'], Path('.'))
        # No group exists for the refused spawn; the pending signal has nobody to reach.
        killpg.assert_not_called()
        self.assertEqual({number: signal.getsignal(number) for number in FORWARDED}, installed)

    def test_a_child_that_outlives_its_wait_is_killed_then_reaped(self) -> None:
        events = []

        class Stubborn(FakeChild):
            def wait(self) -> int:
                events.append('wait')
                return 0

            def poll(self) -> None:
                events.append('poll')
                return None

        with mock.patch.object(browser_tmp.subprocess, 'Popen', lambda *a, **k: Stubborn()), \
                mock.patch.object(browser_tmp.os, 'killpg',
                                  lambda group, number: events.append((group, number))):
            browser_tmp.run_command(['unused'], Path('.'))
        self.assertEqual(events, ['wait', 'poll', (FAKE_GROUP, signal.SIGKILL), 'wait'])

    def test_a_failed_wait_preserves_the_exception_after_kill_and_reap(self) -> None:
        original = RuntimeError('first wait refused')
        events = []

        class Interrupted(FakeChild):
            def wait(self) -> int:
                events.append('wait')
                if events == ['wait']:
                    raise original
                return 0

            def poll(self) -> None:
                events.append('poll')
                return None

        installed = self.recorders()
        with mock.patch.object(browser_tmp.subprocess, 'Popen', lambda *a, **k: Interrupted()), \
                mock.patch.object(browser_tmp.os, 'killpg',
                                  lambda group, number: events.append((group, number))):
            with self.assertRaises(RuntimeError) as raised:
                browser_tmp.run_command(['unused'], Path('.'))
        self.assertIs(raised.exception, original)
        self.assertEqual(events, ['wait', 'poll', (FAKE_GROUP, signal.SIGKILL), 'wait'])
        self.assertEqual({number: signal.getsignal(number) for number in FORWARDED}, installed)

    def test_distinct_pending_signals_are_replayed_in_observed_order(self) -> None:
        installed = self.recorders()
        killed = []

        def spawn(*args, **kwargs):
            for number in FORWARDED:
                os.kill(os.getpid(), number)
            self.assertEqual(killed, [], 'no group identity exists yet')
            return FakeChild()

        with mock.patch.object(browser_tmp.subprocess, 'Popen', spawn), \
                mock.patch.object(browser_tmp.os, 'killpg',
                                  lambda group, number: killed.append((group, number))):
            browser_tmp.run_command(['unused'], Path('.'))
        self.assertEqual(killed, [(FAKE_GROUP, number) for number in FORWARDED])
        self.assertEqual({number: signal.getsignal(number) for number in FORWARDED}, installed)

    def test_default_and_ignored_dispositions_are_restored(self) -> None:
        self.recorders()
        for disposition in (signal.SIG_DFL, signal.SIG_IGN):
            with self.subTest(disposition=disposition):
                for number in FORWARDED:
                    signal.signal(number, disposition)
                with mock.patch.object(browser_tmp.subprocess, 'Popen', lambda *a, **k: FakeChild()):
                    browser_tmp.run_command(['unused'], Path('.'))
                self.assertEqual({signal.getsignal(number) for number in FORWARDED}, {disposition})


class SpawnSignalRealChild(unittest.TestCase):
    """A real wrapper and child group; the wrapper is a separate process so it may be killed."""

    def run_case(self, name: str, disposition: str) -> None:
        require_pidfd_cleanup(self)
        signum = getattr(signal, name)
        marker = f'KNXBENCH_SPAWN_MARKER={os.getpid()}-{time.monotonic_ns()}'
        env = {key: value for key, value in os.environ.items() if key != 'PYTHONPATH'}
        owned = None
        wrapper = None
        with tempfile.TemporaryDirectory() as work:
            receipt = Path(work) / 'receipt'
            out, err = Path(work) / 'stdout', Path(work) / 'stderr'
            try:
                with out.open('wb') as stdout, err.open('wb') as stderr:
                    argv = [sys.executable, '-c', DRIVER, name, disposition, work,
                            marker.split('=', 1)[1], str(receipt), CHILD]
                    wrapper = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
                deadline = time.monotonic() + 30
                child_pid = None
                while child_pid is None:
                    # Poll first: a read after the exit sees everything; only whole lines are records.
                    exited = wrapper.poll() is not None
                    for line in out.read_text().split('\n')[:-1]:
                        if line.startswith('child '):
                            child_pid = int(line.split()[1])
                    if child_pid is None:
                        self.assertFalse(exited, f'driver ended early: {err.read_text()}')
                        self.assertLess(time.monotonic(), deadline, 'driver never reported its child')
                        time.sleep(0.01)
                owned = open_owned_process(child_pid, marker.encode())
                try:
                    status = wrapper.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    status = None
                    wrapper.kill()
                    wrapper.wait(timeout=10)
                received = receipt.read_text() if receipt.exists() else None
                report = (f'{name} ({disposition}): wrapper exit={status}, child saw={received}, '
                          f'child state={process_state(child_pid)}, stdout={out.read_text()!r}')
                self.assertEqual((status, received), (0, str(int(signum))), report)
                self.assertIn('status 7', out.read_text(), report)
            finally:
                if wrapper is not None and wrapper.poll() is None:
                    wrapper.kill()
                    wrapper.wait(timeout=10)
                if owned is not None:
                    try:
                        signal_owned_processes([owned], signal.SIGKILL)
                        wait_for_owned_processes([owned])
                    finally:
                        close_owned_processes([owned])

    def test_default_disposition_wrapper_forwards_a_signal_from_the_spawn_boundary(self) -> None:
        for name in ('SIGTERM', 'SIGINT', 'SIGHUP'):
            with self.subTest(signal=name):
                self.run_case(name, 'dfl')

    def test_ignored_and_python_default_sigint_reach_the_child_too(self) -> None:
        for disposition in ('ign', 'pydefault'):
            with self.subTest(disposition=disposition):
                self.run_case('SIGINT', disposition)


class ScriptedWrapper:
    """Synthetic stand-in for the driver process: a scripted stdout file, no process behind it."""

    def __init__(self, out: Path, receipt: Path, signum: int, initial: str, steps) -> None:
        self.out, self.receipt, self.signum = out, receipt, signum
        self.steps = list(steps)
        self.returncode = None
        self.append(initial)

    def append(self, text: str) -> None:
        with self.out.open('ab') as stream:
            stream.write(text.encode())

    def poll(self):
        if self.steps:
            text, self.returncode = self.steps.pop(0)
            self.append(text)
        return self.returncode

    def wait(self, timeout=None) -> int:
        self.append('status 7\n')
        self.receipt.write_text(str(int(self.signum)))
        self.returncode = 0
        return 0

    def kill(self) -> None:
        self.returncode = -9


class SpawnRecordFraming(unittest.TestCase):
    """Synthetic fixture controls (not app evidence): run_case against a scripted driver stdout.

    Only run_case's own reader runs; the process helpers and Popen are stubbed, so no child exists.
    """

    def run_scripted(self, initial: str, steps) -> mock.MagicMock:
        def fake_popen(argv, cwd, env, stdout, stderr):
            return ScriptedWrapper(Path(stdout.name), Path(argv[-2]), signal.SIGHUP, initial, steps)

        module = sys.modules[__name__]
        with mock.patch.object(subprocess, 'Popen', fake_popen), \
                mock.patch.object(module, 'require_pidfd_cleanup'), \
                mock.patch.object(module, 'open_owned_process', return_value=object()) as opened, \
                mock.patch.object(module, 'process_state', return_value='S'), \
                mock.patch.object(module, 'signal_owned_processes'), \
                mock.patch.object(module, 'wait_for_owned_processes'), \
                mock.patch.object(module, 'close_owned_processes'):
            SpawnSignalRealChild().run_case('SIGHUP', 'dfl')
        return opened

    def test_an_unfinished_print_snapshot_waits_for_its_newline(self) -> None:
        opened = self.run_scripted('child ', [('', None), ('123 ready\n', None)])
        self.assertEqual(opened.call_args.args[0], 123)

    def test_a_partial_pid_fragment_waits_for_the_complete_record(self) -> None:
        opened = self.run_scripted('child 1', [('', None), ('23 ready\n', None)])
        self.assertEqual(opened.call_args.args[0], 123)

    def test_a_snapshot_cut_before_the_wrapper_exit_is_drained_not_blamed_on_the_exit(self) -> None:
        opened = self.run_scripted('child ', [('', None), ('123 ready\n', 0)])
        self.assertEqual(opened.call_args.args[0], 123)

    def test_a_complete_identity_row_without_a_pid_still_fails(self) -> None:
        with self.assertRaises(IndexError):
            self.run_scripted('child \n', [('', None)])

    def test_an_exited_wrapper_with_only_a_fragment_still_fails(self) -> None:
        with self.assertRaises(AssertionError) as raised:
            self.run_scripted('child ', [('', 0)])
        self.assertIn('driver ended early', str(raised.exception))


if __name__ == '__main__':
    unittest.main()
