"""Keep stuck-wrapper diagnostics useful without generating intentional crash reports."""
from __future__ import annotations

from contextlib import ExitStack, nullcontext
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import unittest
from unittest import mock

from tools.tests import test_browser_tmp_launch as launch
from tools.tests.browser_tmp_fixtures import short_workspace


class BrowserTempDiagnostics(unittest.TestCase):
    def diagnose_with_mocks(self, *, communication_error: Exception | None = None):
        """Exercise the real helper while intercepting every destructive operation."""
        wrapper = mock.Mock(pid=10, returncode=None)
        wrapper.communicate.return_value = ('', 'fixture stderr\n')
        wrapper.communicate.side_effect = communication_error
        wrapper_identity = launch.OwnedProcessIdentity(10, 101)
        child_identity = launch.OwnedProcessIdentity(20, 102)
        group = [wrapper_identity, child_identity]
        events = []

        def record_signal(processes, signum):
            # Even the original broken helper must never abort a real process.
            events.append(('signal', list(processes), signum))

        def record_state(pid):
            events.append(('state', pid))
            return f'{pid}: State=S wchan=fixture_wait'

        with ExitStack() as stack:
            stack.enter_context(mock.patch.object(launch, 'fixture_processes', return_value=group))
            stack.enter_context(mock.patch.object(launch, 'process_state', side_effect=record_state))
            stack.enter_context(mock.patch.object(launch, 'running_owned_processes',
                                                  side_effect=lambda processes: processes))
            stack.enter_context(mock.patch.object(launch, 'signal_owned_processes',
                                                  side_effect=record_signal))
            stack.enter_context(mock.patch.object(launch.select, 'select', return_value=([], [], [])))
            waited = stack.enter_context(mock.patch.object(launch, 'wait_for_owned_processes'))
            closed = stack.enter_context(mock.patch.object(launch, 'close_owned_processes'))
            if communication_error is None:
                diagnosis = launch.diagnose_stuck_wrapper(wrapper, Path('fixture'), wrapper_identity)
            else:
                with self.assertRaises(type(communication_error)):
                    launch.diagnose_stuck_wrapper(wrapper, Path('fixture'), wrapper_identity)
                diagnosis = ''
        return diagnosis, events, wrapper, group, waited, closed

    def test_diagnostics_capture_state_and_stderr_then_kill_without_core_signal(self):
        diagnosis, events, wrapper, group, waited, closed = self.diagnose_with_mocks()
        signals = [event for event in events if event[0] == 'signal']
        self.assertEqual(signals, [('signal', group, signal.SIGKILL)],
                         'diagnostics must not deliberately crash fixture processes')
        first_signal = next(i for i, event in enumerate(events) if event[0] == 'signal')
        self.assertTrue(all(event[0] == 'state' for event in events[:first_signal]))
        self.assertIn('10: State=S wchan=fixture_wait', diagnosis)
        self.assertIn('20: State=S wchan=fixture_wait', diagnosis)
        self.assertIn('fixture stderr', diagnosis)
        self.assertIn('wrapper pipes still open 10 s after SIGTERM', diagnosis)
        wrapper.communicate.assert_called_once_with(timeout=10)
        waited.assert_called_once_with(group)
        closed.assert_called_once_with(group)

    def test_diagnostics_close_scanned_identities_even_when_pipe_read_times_out(self):
        _, events, _, group, _, closed = self.diagnose_with_mocks(
            communication_error=subprocess.TimeoutExpired('fixture', 10))
        self.assertEqual([event[2] for event in events if event[0] == 'signal'], [signal.SIGKILL])
        closed.assert_called_once_with(group)

    def probe_native_setup_failure(self, *, ready=True, pipe_timeout=False):
        """Fail the actual native test's setup with all process operations mocked."""
        wrapper = mock.Mock(pid=10, returncode=None)
        wrapper.stdout.readline.return_value = '20\n'
        wrapper.communicate.return_value = ('', '')
        identity = launch.OwnedProcessIdentity(10, 101)
        error = subprocess.TimeoutExpired('fixture', 10) if pipe_timeout else None
        wrapper.communicate.side_effect = error
        with ExitStack() as stack:
            stack.enter_context(mock.patch(__name__ + '.short_workspace',
                                           return_value=nullcontext(Path('fixture'))))
            stack.enter_context(mock.patch.object(launch, 'require_pidfd_cleanup'))
            stack.enter_context(mock.patch.object(subprocess, 'Popen', return_value=wrapper))
            stack.enter_context(mock.patch.object(select, 'select',
                                                  return_value=([wrapper.stdout] if ready else [], [], [])))
            opened = stack.enter_context(mock.patch.object(launch, 'open_owned_process',
                                                            side_effect=[identity, None]))
            stack.enter_context(mock.patch.object(launch, 'running_owned_processes',
                                                  side_effect=lambda processes: processes))
            signalled = stack.enter_context(mock.patch.object(launch, 'signal_owned_processes'))
            waited = stack.enter_context(mock.patch.object(launch, 'wait_for_owned_processes'))
            closed = stack.enter_context(mock.patch.object(launch, 'close_owned_processes'))
            case = BrowserTempDiagnostics('test_real_stuck_wrapper_and_child_end_without_crashing')
            expected = subprocess.TimeoutExpired if pipe_timeout else AssertionError
            with self.assertRaises(expected):
                case.test_real_stuck_wrapper_and_child_end_without_crashing()
        owned = [identity] if ready else []
        self.assertEqual(opened.call_count, 2 if ready else 0)
        signalled.assert_called_once_with(owned, signal.SIGKILL)
        wrapper.kill.assert_called_once_with()
        wrapper.communicate.assert_called_once_with(timeout=10)
        waited.assert_called_once_with(owned)
        closed.assert_called_once_with(owned)

    def test_native_fixture_cleans_up_after_readiness_failure(self):
        self.probe_native_setup_failure(ready=False)

    def test_native_fixture_cleans_up_after_child_identity_rejection(self):
        self.probe_native_setup_failure()

    def test_native_fixture_closes_handles_even_when_cleanup_pipe_read_fails(self):
        self.probe_native_setup_failure(pipe_timeout=True)

    def test_real_stuck_wrapper_and_child_end_without_crashing(self):
        launch.require_pidfd_cleanup(self)
        with short_workspace() as directory:
            marker = f'KNXBENCH_BROWSER_TMP_DIR={directory}'.encode()
            # Parent-death bounded as well as explicitly killed: setup failures
            # before the PID/identity handshake must not strand a child.
            child_code = 'import os;print(os.getpid(),flush=True)\n' + launch.ORPHAN_EXIT
            wrapper_code = ('import os,subprocess,sys;'
                            'print("diagnostic stderr witness",file=sys.stderr,flush=True);'
                            f'child=subprocess.Popen([sys.executable,"-c",{child_code!r},'
                            'str(os.getpid())]);'
                            'child.wait()')
            wrapper = subprocess.Popen(
                [sys.executable, '-c', wrapper_code],
                env={**os.environ, 'KNXBENCH_BROWSER_TMP_DIR': str(directory)},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            identities = []
            try:
                assert wrapper.stdout is not None
                self.assertTrue(select.select([wrapper.stdout], [], [], 10)[0],
                                'diagnostic child did not announce readiness')
                child_pid = int(wrapper.stdout.readline())
                for pid in (wrapper.pid, child_pid):
                    identity = launch.open_owned_process(pid, marker)
                    self.assertIsNotNone(identity, 'fixture identity could not be pinned')
                    identities.append(identity)
                real_send = signal.pidfd_send_signal
                sent = []

                def send_without_crash(pidfd, signum, *args):
                    self.assertEqual(signum, signal.SIGKILL,
                                     'diagnostic attempted a crash-generating signal')
                    sent.append(pidfd)
                    return real_send(pidfd, signum, *args)

                with mock.patch.object(signal, 'pidfd_send_signal', side_effect=send_without_crash):
                    diagnosis = launch.diagnose_stuck_wrapper(wrapper, directory, identities[0])
                self.assertEqual(len(sent), 2, 'wrapper and child must both be cleaned up')
                self.assertEqual(wrapper.returncode, -signal.SIGKILL)
                self.assertEqual(launch.running_owned_processes(identities), [])
                self.assertIn(f'{wrapper.pid}: State=', diagnosis)
                self.assertIn(f'{child_pid}: State=', diagnosis)
                self.assertIn('diagnostic stderr witness', diagnosis)
                os.fstat(identities[0].pidfd)  # Caller-owned identity remains open.
            finally:
                try:
                    launch.signal_owned_processes(launch.running_owned_processes(identities), signal.SIGKILL)
                    # Unreaped direct child; safe even when fixture setup failed.
                    wrapper.kill()
                    wrapper.communicate(timeout=10)
                finally:
                    try:
                        launch.wait_for_owned_processes(identities)
                    finally:
                        launch.close_owned_processes(identities)


if __name__ == '__main__':
    unittest.main()
