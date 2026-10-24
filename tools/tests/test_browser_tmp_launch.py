"""Regress the recurring leaf-root refusal and unguarded standalone launches."""
from __future__ import annotations

import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import time
from typing import NamedTuple
import unittest
from unittest import mock

from tools import browser_tmp
from tools.tests.browser_tmp_fixtures import deep_below_scratch, short_workspace

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / 'apps/knx-web'
STATUS_FIELDS = ('State', 'PPid', 'NSpgid', 'NSsid', 'Threads', 'SigPnd', 'ShdPnd', 'SigBlk', 'SigIgn',
                 'SigCgt')
SIGTERM_CHILD = '''
import json
import os
from pathlib import Path
import signal
import sys
import time

directory = Path(os.environ["TMPDIR"])
receipt = Path(os.environ["KNXBENCH_SIGTERM_RECEIPT"])
release = Path(os.environ["KNXBENCH_SIGTERM_RELEASE"])
signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGTERM})
print(directory, flush=True)
if signal.sigtimedwait({signal.SIGTERM}, 60) is None:
    sys.exit(2)
if not directory.is_dir():
    sys.exit(3)
staging = receipt.with_suffix(".tmp")
staging.write_text(json.dumps({"pid": os.getpid(), "directory": str(directory)}))
os.replace(staging, receipt)
deadline = time.monotonic() + 60
while not release.exists():
    if time.monotonic() >= deadline:
        sys.exit(4)
    time.sleep(0.01)
'''
PREMATURE_CLEANUP_MUTANT = '''
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

base = Path(sys.argv[1])
mutated = Path(os.environ["KNXBENCH_SIGTERM_MUTATED"])
receipt = Path(os.environ["KNXBENCH_SIGTERM_RECEIPT"])
directory = Path(tempfile.mkdtemp(prefix="mutant-", dir=base))
env = {**os.environ, "TMPDIR": str(directory), "KNXBENCH_BROWSER_TMP_DIR": str(directory)}
signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGTERM})
child = subprocess.Popen(sys.argv[2:], env=env, start_new_session=True)
child_pidfd = os.pidfd_open(child.pid)

def forward(signum, frame):
    try:
        signal.pidfd_send_signal(child_pidfd, signum)
    except ProcessLookupError:
        pass
    deadline = time.monotonic() + 10
    while not receipt.exists():
        if time.monotonic() >= deadline:
            os.close(child_pidfd)
            os._exit(5)
        time.sleep(0.01)
    shutil.rmtree(directory)
    mutated.write_text(json.dumps({"child": child.pid, "directory": str(directory)}))
    os.close(child_pidfd)
    os._exit(0)

signal.signal(signal.SIGTERM, forward)
signal.pthread_sigmask(signal.SIG_UNBLOCK, {signal.SIGTERM})
status = child.wait()
os.close(child_pidfd)
sys.exit(status if status >= 0 else 128 - status)
'''


class OwnedProcessIdentity(NamedTuple):
    """A fixture process pinned to the kernel identity observed during validation."""

    pid: int
    pidfd: int


def process_state(pid: int) -> str:
    """Kernel view of one process: run state, wait channel and signal masks."""
    proc = Path('/proc') / str(pid)
    try:
        status = dict(line.split(':\t', 1) for line in (proc / 'status').read_text().splitlines()
                      if ':\t' in line)
        wchan = (proc / 'wchan').read_text()
    except OSError as error:
        return f'{pid}: gone ({error.strerror})'
    fields = ' '.join(f'{key}={status.get(key, "?").strip()}' for key in STATUS_FIELDS)
    return f'{pid}: {fields} wchan={wchan or "-"}'


def diagnose_stuck_wrapper(
        wrapper: subprocess.Popen, directory: Path,
        wrapper_identity: OwnedProcessIdentity | None = None) -> str:
    """Describe a wrapper that did not finish after SIGTERM, then end it.

    Records the wrapper and every process carrying its temp directory (the
    child group), aborts them so PYTHONFAULTHANDLER prints their Python
    stacks, and returns all of it as the failure message.
    """
    group = fixture_processes(directory)
    try:
        diagnostic_processes = ([] if wrapper_identity is None else [wrapper_identity])
        diagnostic_processes.extend(
            process for process in group
            if wrapper_identity is None or process.pid != wrapper_identity.pid)
        states = [f'load={os.getloadavg()}', f'wrapper exit={wrapper.returncode}',
                  process_state(wrapper.pid),
                  *(process_state(process.pid) for process in group)]
        # One at a time, so the stacks do not interleave on the shared stderr pipe.
        for process in running_owned_processes(diagnostic_processes):
            signal_owned_processes([process], signal.SIGABRT)
            select.select([process.pidfd], [], [], 3)
        try:
            _, stderr = wrapper.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            remaining = running_owned_processes(diagnostic_processes)
            signal_owned_processes(remaining, signal.SIGKILL)
            _, stderr = wrapper.communicate(timeout=10)
        stacks = [line for line in stderr.splitlines()
                  if not line.lstrip().startswith('Binary file')]
        return ('wrapper pipes still open 10 s after SIGTERM\n' + '\n'.join(states) + '\n'
                + '\n'.join(stacks)[-6000:])
    finally:
        close_owned_processes(group)


def wait_for_path(path: Path, wrapper: subprocess.Popen, directory: Path,
                  description: str, timeout: float = 10) -> None:
    """Wait for a fixture receipt, using time only as a bounded failure backstop."""
    deadline = time.monotonic() + timeout
    while not path.exists():
        status = wrapper.poll()
        if status is not None:
            raise AssertionError(
                f'wrapper exited with {status} before {description}; '
                f'directory exists={directory.exists()}; {process_state(wrapper.pid)}')
        if time.monotonic() >= deadline:
            raise AssertionError(
                f'timed out waiting for {description}; directory exists={directory.exists()}; '
                f'{process_state(wrapper.pid)}')
        time.sleep(0.01)


def wait_for_wrapper_exit(wrapper: subprocess.Popen, directory: Path, timeout: float = 10) -> int:
    """Return the wrapper's actual status, bounded by a diagnostic timeout."""
    deadline = time.monotonic() + timeout
    while True:
        status = wrapper.poll()
        if status is not None:
            return status
        if time.monotonic() >= deadline:
            raise AssertionError(
                f'timed out waiting for synthetic wrapper exit; directory exists={directory.exists()}; '
                f'{process_state(wrapper.pid)}')
        time.sleep(0.01)


def assert_sigterm_child_held(test: unittest.TestCase, wrapper: subprocess.Popen,
                              directory: Path, child_pid: int) -> None:
    """Assert the state a correct wrapper must hold until the child is released."""
    problems = []
    if not directory.is_dir():
        problems.append('temporary directory removed before child release')
    status = wrapper.poll()
    if status is not None:
        problems.append(f'wrapper exited before child release (status {status})')
    if not (Path('/proc') / str(child_pid)).exists():
        problems.append(f'child {child_pid} disappeared before release')
    if problems:
        test.fail('; '.join(problems))


def pidfd_process_id(pidfd: int) -> int | None:
    """Return the live process ID retained by a pidfd, or None after its exit."""
    try:
        lines = (Path('/proc/self/fdinfo') / str(pidfd)).read_text().splitlines()
        value = next(line.split(':', 1)[1].strip() for line in lines if line.startswith('Pid:'))
        pid = int(value)
        return pid if pid >= 0 else None
    except (OSError, StopIteration, ValueError):
        return None


def open_owned_process(pid: int, marker: bytes) -> OwnedProcessIdentity | None:
    """Pin and revalidate one marker-owned process without trusting its numeric PID."""
    pidfd = None
    proc = Path('/proc') / str(pid)
    try:
        if marker not in (proc / 'environ').read_bytes().split(b'\0'):
            return None
        pidfd = os.pidfd_open(pid)
        if pidfd_process_id(pidfd) != pid:
            return None
        if marker not in (proc / 'environ').read_bytes().split(b'\0'):
            return None
        identity = OwnedProcessIdentity(pid=pid, pidfd=pidfd)
        pidfd = None
        return identity
    except OSError:
        return None
    finally:
        if pidfd is not None:
            os.close(pidfd)


def marked_processes(marker: bytes) -> list[OwnedProcessIdentity]:
    """Pin every process that retains one exact fixture ownership marker."""
    processes = []
    try:
        for entry in Path('/proc').iterdir():
            if not entry.name.isdigit():
                continue
            process = open_owned_process(int(entry.name), marker)
            if process is not None:
                processes.append(process)
        return processes
    except BaseException:
        close_owned_processes(processes)
        raise


def fixture_processes(directory: Path) -> list[OwnedProcessIdentity]:
    """Pin live processes carrying this fixture's unique temp-directory marker."""
    return marked_processes(f'KNXBENCH_BROWSER_TMP_DIR={directory}'.encode())


def require_pidfd_cleanup(test: unittest.TestCase) -> None:
    """Skip before fixture launch when ownership-safe cleanup is unavailable."""
    if not hasattr(os, 'pidfd_open') or not hasattr(signal, 'pidfd_send_signal'):
        test.skipTest('Linux pidfds are required for ownership-safe fixture cleanup')
    try:
        pidfd = os.pidfd_open(os.getpid())
        try:
            if pidfd_process_id(pidfd) != os.getpid():
                test.skipTest('Linux pidfds cannot be revalidated through /proc fdinfo')
            signal.pidfd_send_signal(pidfd, 0)
        finally:
            os.close(pidfd)
    except OSError as error:
        test.skipTest(f'Linux pidfds are unavailable for ownership-safe fixture cleanup: {error}')


def handshake_processes(release: Path) -> list[OwnedProcessIdentity]:
    """Pin every process validated against this fixture's unique handshake."""
    if not hasattr(os, 'pidfd_open'):
        raise RuntimeError('pidfd support must be checked before launching a SIGTERM fixture')
    return marked_processes(f'KNXBENCH_SIGTERM_RELEASE={release}'.encode())


def signal_owned_processes(processes: list[OwnedProcessIdentity], signum: signal.Signals) -> None:
    """Signal only the kernel identities retained while fixture ownership was proven."""
    for process in processes:
        try:
            signal.pidfd_send_signal(process.pidfd, signum)
        except ProcessLookupError:
            pass


def close_owned_processes(processes: list[OwnedProcessIdentity]) -> None:
    """Close retained process identities after all possible signals."""
    for process in processes:
        os.close(process.pidfd)


def running_owned_processes(
        processes: list[OwnedProcessIdentity]) -> list[OwnedProcessIdentity]:
    """Return retained identities whose original processes have not exited."""
    if not processes:
        return []
    exited, _, _ = select.select([process.pidfd for process in processes], [], [], 0)
    return [process for process in processes if process.pidfd not in exited]


def wait_for_owned_processes(
        processes: list[OwnedProcessIdentity], timeout: float = 10) -> None:
    """Wait boundedly for retained fixture identities to report process exit."""
    remaining = running_owned_processes(processes)
    deadline = time.monotonic() + timeout
    while remaining:
        wait = deadline - time.monotonic()
        if wait <= 0:
            states = ', '.join(process_state(process.pid) for process in remaining)
            raise AssertionError(f'fixture-owned processes survived cleanup: {states}')
        exited, _, _ = select.select([process.pidfd for process in remaining], [], [], wait)
        exited = set(exited)
        remaining = [process for process in remaining if process.pidfd not in exited]


def cleanup_sigterm_fixture(
        wrapper: subprocess.Popen, release: Path,
        wrapper_identity: OwnedProcessIdentity | None = None) -> None:
    """Release or kill every process owned by one SIGTERM fixture.

    A supplied wrapper identity is consumed and closed by this function.
    """
    owned = ([] if wrapper_identity is None else [wrapper_identity])
    try:
        discovered = handshake_processes(release)
        for process in discovered:
            if wrapper_identity is not None and process.pid == wrapper_identity.pid:
                os.close(process.pidfd)
            else:
                owned.append(process)
        wrapper_processes = [process for process in owned if process.pid == wrapper.pid]
        release.touch(exist_ok=True)
        try:
            wrapper.communicate(timeout=3)
        except subprocess.TimeoutExpired:
            signal_owned_processes(running_owned_processes(wrapper_processes), signal.SIGTERM)
            try:
                wrapper.communicate(timeout=3)
            except subprocess.TimeoutExpired:
                signal_owned_processes(running_owned_processes(owned), signal.SIGKILL)
                wrapper.communicate(timeout=10)
        remaining = running_owned_processes(owned)
        signal_owned_processes(remaining, signal.SIGKILL)
        wait_for_owned_processes(remaining)
    finally:
        close_owned_processes(owned)


class BrowserTempLaunch(unittest.TestCase):
    def clean_environment(self) -> dict[str, str]:
        return {key: value for key, value in os.environ.items()
                if not key.startswith(('KNX', 'PLAYWRIGHT_'))}

    def workspace(self):
        return short_workspace()

    def leaf_root(self, scratch: Path) -> Path:
        # A root that fits Chromium itself, but cannot fit another /wXXXXXXXX.
        size = browser_tmp.MAX_SOCKET_BYTES - len(os.fsencode(browser_tmp.CHROMIUM_SOCKET_SUFFIX))
        leaf = scratch / ('x' * (size - len(os.fsencode(scratch)) - 1))
        leaf.mkdir(mode=0o700)
        self.assertFalse(browser_tmp.fits_temp_root(leaf))
        browser_tmp.validate_owned_directory(leaf)
        return leaf

    def test_direct_config_reuses_private_explicit_leaf_and_preserves_it(self) -> None:
        with self.workspace() as tmp:
            work = Path(tmp)
            leaf = self.leaf_root(work)
            marker = leaf / 'keep'
            marker.write_text('caller owns this')
            metadata = work / 'observed.json'
            config = work / 'probe.config.ts'
            config.write_text(
                f'import base from {json.dumps(str(APP / "playwright.config.ts"))};\n'
                'import {writeFileSync} from "node:fs";\n'
                f'writeFileSync({json.dumps(str(metadata))}, JSON.stringify(process.env.TMPDIR));\n'
                f'export default {{...base,testDir:{json.dumps(str(APP / "e2e"))}}};\n')
            env = {**self.clean_environment(), 'TMPDIR': str(leaf),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(leaf)}
            result = subprocess.run(
                ['node', 'node_modules/@playwright/test/cli.js', 'test',
                 f'--config={config}', '--list', '--reporter=json'],
                cwd=APP, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, (result.stdout + result.stderr)[-3000:])
            self.assertEqual(json.loads(metadata.read_text()), str(leaf))
            self.assertEqual(marker.read_text(), 'caller owns this')

    def test_python_context_reuses_leaf_without_deleting_caller_data(self) -> None:
        self.assertTrue(hasattr(browser_tmp, 'browser_directory'), 'Shared allocator is missing')
        with self.workspace() as tmp:
            leaf = self.leaf_root(Path(tmp))
            marker = leaf / 'keep'
            marker.write_text('retained')
            with browser_tmp.browser_directory(leaf, explicit=leaf) as directory:
                self.assertEqual(directory, leaf)
            self.assertEqual(marker.read_text(), 'retained')
            leaf.chmod(0o755)
            with self.assertRaisesRegex(ValueError, 'private'):
                with browser_tmp.browser_directory(leaf, explicit=leaf):
                    self.fail('must not reuse a non-private leaf')

    def test_standalone_preload_repairs_deep_tmp_before_loading_playwright(self) -> None:
        with deep_below_scratch() as (work, deep):
            marker = deep / 'keep'
            marker.write_text('retained')
            env = {**self.clean_environment(), 'TMPDIR': str(deep)}
            program = (
                "const fs=require('node:fs');"
                "const path=process.env.TMPDIR;"
                "if(Buffer.byteLength(path + '/org.chromium.Chromium.XXXXXX/SingletonSocket')>107)"
                "throw Error('unprotected Chromium temp path');"
                "console.log(JSON.stringify({path,mode:fs.statSync(path).mode&511}));")
            result = subprocess.run(
                ['node', '--require', str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=work, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            observed = json.loads(result.stdout)
            self.assertEqual(Path(observed['path']).parent, browser_tmp.select_temp_root(deep))
            self.assertEqual(observed['mode'], 0o700)
            self.assertFalse(Path(observed['path']).exists())
            self.assertEqual(marker.read_text(), 'retained')

    def test_cli_wraps_a_standalone_command_and_cleans_after_failure(self) -> None:
        with deep_below_scratch() as (work, deep):
            program = 'import os,sys;print(os.environ["TMPDIR"]);sys.exit(23)'
            result = subprocess.run(
                [sys.executable, str(ROOT / 'tools/browser_tmp.py'), '--base', str(deep),
                 '--run', sys.executable, '-c', program],
                env=self.clean_environment(), capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 23, result.stderr)
            directory = Path(result.stdout.strip())
            self.assertEqual(directory.parent, browser_tmp.select_temp_root(deep))
            self.assertFalse(directory.exists())
            self.assertTrue(deep.exists())

    def test_real_chromium_launches_at_the_validated_leaf_boundary(self) -> None:
        with deep_below_scratch() as (work, deep):
            leaf = self.leaf_root(work)
            env = {**self.clean_environment(), 'TMPDIR': str(leaf),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(leaf)}
            program = (
                "const fs=require('node:fs');const {chromium}=require('playwright');"
                "(async()=>{const profile=process.env.TMPDIR+'/p';"
                "const context=await chromium.launchPersistentContext(profile,"
                "{executablePath:'/usr/bin/chromium',headless:true});"
                "try{const page=await context.newPage();"
                "await page.goto('data:text/html,actual-browser-proof');"
                "if(await page.locator('body').textContent()!=='actual-browser-proof')throw Error('body');"
                "console.log(JSON.stringify({directory:process.env.TMPDIR,"
                "socket:fs.readlinkSync(profile+'/SingletonSocket')}));"
                "}finally{await context.close();}})().catch(e=>{console.error(e);process.exit(1)});")
            result = subprocess.run(
                ['unshare', '--user', '--map-root-user', '--net', 'node', '--require',
                 str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=APP, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr[-4000:])
            observed = json.loads(result.stdout)
            self.assertEqual(observed['directory'], str(leaf))
            self.assertLessEqual(len(os.fsencode(observed['socket'])), browser_tmp.MAX_SOCKET_BYTES)
            self.assertTrue(leaf.exists(), 'caller-owned leaf must survive browser shutdown')
            result = subprocess.run(
                ['unshare', '--user', '--map-root-user', '--net', 'node', '--require',
                 str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=APP, env={**self.clean_environment(), 'TMPDIR': str(deep)},
                capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr[-4000:])
            observed = json.loads(result.stdout)
            self.assertLessEqual(len(os.fsencode(observed['socket'])), browser_tmp.MAX_SOCKET_BYTES)
            self.assertFalse(Path(observed['directory']).exists(), 'owned browser temp leaked')
            self.assertTrue(deep.exists(), 'deep caller directory must survive')

    def test_nested_node_process_reuses_parent_directory_without_owning_cleanup(self) -> None:
        with deep_below_scratch() as (work, deep):
            guard = str(ROOT / 'tools/browser_tmp.cjs')
            child = "console.log(JSON.stringify(process.env.TMPDIR))"
            program = (
                "const {execFileSync}=require('node:child_process');const fs=require('node:fs');"
                "const directory=process.env.TMPDIR;"
                f"const nested=JSON.parse(execFileSync(process.execPath,['--require',{json.dumps(guard)},"
                f"'-e',{json.dumps(child)}],{{encoding:'utf8'}}));"
                "if(nested!==directory||!fs.existsSync(directory))throw Error('parent ownership lost');"
                "console.log(JSON.stringify(directory));")
            result = subprocess.run(['node', '--require', guard, '-e', program],
                                    cwd=work, env={**self.clean_environment(), 'TMPDIR': str(deep)},
                                    capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(Path(json.loads(result.stdout)).exists())
            self.assertTrue(deep.exists())

    def test_cli_sigterm_waits_for_child_shutdown_before_cleanup(self) -> None:
        require_pidfd_cleanup(self)
        with self.workspace() as tmp:
            work = Path(tmp)
            # The child blocks SIGTERM before it announces itself and then takes
            # it synchronously. A handler followed by time.sleep lost the signal
            # when it landed while print() returned: CPython 3.14 runs no pending
            # handler after a keyword call, so the sleep blocked for a minute.
            receipt = work / 'signal-received.json'
            release = work / 'release-child'
            # PYTHONFAULTHANDLER only acts on fatal signals: on a timeout the
            # diagnosis aborts both processes and gets their Python stacks.
            child = subprocess.Popen(
                [sys.executable, str(ROOT / 'tools/browser_tmp.py'), '--base', str(work),
                 '--run', sys.executable, '-c', SIGTERM_CHILD],
                env={**self.clean_environment(), 'PYTHONFAULTHANDLER': '1',
                     'KNXBENCH_SIGTERM_RECEIPT': str(receipt),
                     'KNXBENCH_SIGTERM_RELEASE': str(release)},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            directory = None
            child_pid = None
            wrapper_identity = None
            try:
                self.assertTrue(select.select([child.stdout], [], [], 10)[0], 'child did not start')
                directory = Path(child.stdout.readline().strip())
                self.assertTrue(directory.is_dir())
                self.assertNotIn(directory, receipt.parents)
                self.assertNotIn(directory, release.parents)
                marker = f'KNXBENCH_SIGTERM_RELEASE={release}'.encode()
                wrapper_identity = open_owned_process(child.pid, marker)
                if wrapper_identity is None:
                    self.fail('wrapper identity changed before SIGTERM')
                signal_owned_processes([wrapper_identity], signal.SIGTERM)
                wait_for_path(receipt, child, directory, 'child signal receipt')
                observed = json.loads(receipt.read_text())
                child_pid = observed['pid']
                self.assertEqual(observed['directory'], str(directory))
                assert_sigterm_child_held(self, child, directory, child_pid)
                release.touch()
                try:
                    stdout, stderr = child.communicate(timeout=10)
                except subprocess.TimeoutExpired:
                    self.fail(diagnose_stuck_wrapper(child, directory, wrapper_identity))
                self.assertEqual(child.returncode, 0, stderr)
                self.assertFalse(directory.exists())
                self.assertFalse((Path('/proc') / str(child_pid)).exists(),
                                 'wrapper returned without reaping its child')
                self.assertTrue(work.exists())
            finally:
                cleanup_sigterm_fixture(child, release, wrapper_identity)

    def test_sigterm_contract_rejects_missing_wait_and_premature_cleanup_mutant(self) -> None:
        require_pidfd_cleanup(self)
        with self.workspace() as tmp:
            work = Path(tmp)
            receipt = work / 'signal-received.json'
            release = work / 'release-child'
            mutated = work / 'mutated.json'
            env = {**self.clean_environment(), 'KNXBENCH_SIGTERM_RECEIPT': str(receipt),
                   'KNXBENCH_SIGTERM_RELEASE': str(release),
                   'KNXBENCH_SIGTERM_MUTATED': str(mutated)}
            wrapper = subprocess.Popen(
                [sys.executable, '-c', PREMATURE_CLEANUP_MUTANT, str(work),
                 sys.executable, '-c', SIGTERM_CHILD],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            directory = None
            wrapper_identity = None
            try:
                self.assertTrue(select.select([wrapper.stdout], [], [], 10)[0],
                                'synthetic child did not start')
                directory = Path(wrapper.stdout.readline().strip())
                self.assertTrue(directory.is_dir())
                marker = f'KNXBENCH_SIGTERM_RELEASE={release}'.encode()
                wrapper_identity = open_owned_process(wrapper.pid, marker)
                if wrapper_identity is None:
                    self.fail('synthetic wrapper identity changed before SIGTERM')
                signal_owned_processes([wrapper_identity], signal.SIGTERM)
                wait_for_path(receipt, wrapper, directory, 'synthetic child signal receipt')
                observed = json.loads(receipt.read_text())
                wait_for_path(mutated, wrapper, directory, 'synthetic premature cleanup')
                self.assertEqual(wait_for_wrapper_exit(wrapper, directory), 0)
                self.assertFalse(directory.exists(), 'synthetic mutant did not clean prematurely')
                with self.assertRaisesRegex(
                        AssertionError,
                        'temporary directory removed before child release.*wrapper exited before child release'):
                    assert_sigterm_child_held(self, wrapper, directory, observed['pid'])
            finally:
                cleanup_sigterm_fixture(wrapper, release, wrapper_identity)

    def test_sigterm_fixture_cleanup_rejects_mismatched_pidfd_identity(self) -> None:
        require_pidfd_cleanup(self)
        with self.workspace() as tmp:
            release = Path(tmp) / 'owned-release'
            env = {**self.clean_environment(), 'KNXBENCH_SIGTERM_RELEASE': str(release)}
            fixture = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(4)'], env=env)
            sentinel_marker = f'KNXBENCH_SIGTERM_SENTINEL={release}'.encode()
            real_pidfd_open = os.pidfd_open
            sentinel = None
            fixture_identity = None
            sentinel_identity = None
            scanned_pidfds = []
            try:
                sentinel = subprocess.Popen(
                    [sys.executable, '-c', 'import time; time.sleep(8)'],
                    env={**self.clean_environment(), 'KNXBENCH_SIGTERM_SENTINEL': str(release)})
                fixture_identity = open_owned_process(
                    fixture.pid, f'KNXBENCH_SIGTERM_RELEASE={release}'.encode())
                sentinel_identity = open_owned_process(sentinel.pid, sentinel_marker)
                if fixture_identity is None or sentinel_identity is None:
                    self.fail('cleanup regression identities changed before pidfd retention')

                def mismatched_pidfd_open(pid: int, flags: int = 0) -> int:
                    target = sentinel.pid if pid == fixture.pid else pid
                    pidfd = real_pidfd_open(target, flags)
                    scanned_pidfds.append(pidfd)
                    return pidfd

                with mock.patch.object(os, 'pidfd_open', side_effect=mismatched_pidfd_open):
                    cleanup_sigterm_fixture(fixture, release)
                self.assertEqual(running_owned_processes([sentinel_identity]), [sentinel_identity],
                                 'cleanup signalled an unrelated process through a mismatched pidfd')
                self.assertTrue(scanned_pidfds, 'cleanup did not inspect the mismatched identity')
                for pidfd in scanned_pidfds:
                    with self.assertRaises(OSError, msg='cleanup leaked a scanned pidfd'):
                        os.fstat(pidfd)
            finally:
                identities = [identity for identity in (fixture_identity, sentinel_identity)
                              if identity is not None]
                signal_owned_processes(running_owned_processes(identities), signal.SIGKILL)
                wait_for_owned_processes(identities)
                fixture.wait(timeout=10)
                if sentinel is not None:
                    sentinel.wait(timeout=10)
                close_owned_processes(identities)

    def test_sigterm_diagnostics_reject_mismatched_scanned_identity(self) -> None:
        require_pidfd_cleanup(self)
        with self.workspace() as tmp:
            directory = Path(tmp) / 'diagnostic-directory'
            marker = str(directory)
            program = (
                'import os,subprocess,sys,time;'
                'child=subprocess.Popen([sys.executable,"-c","import time;time.sleep(4)"],'
                'env=os.environ.copy());'
                'print(child.pid,flush=True);'
                'time.sleep(8)')
            env = {**self.clean_environment(), 'PYTHONFAULTHANDLER': '1',
                   'KNXBENCH_BROWSER_TMP_DIR': marker}
            wrapper = subprocess.Popen(
                [sys.executable, '-c', program], env=env,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            real_pidfd_open = os.pidfd_open
            sentinel = None
            wrapper_identity = None
            child_identity = None
            sentinel_identity = None
            scanned_pidfds = []
            try:
                self.assertTrue(select.select([wrapper.stdout], [], [], 10)[0],
                                'diagnostic fixture child did not start')
                child_pid = int(wrapper.stdout.readline())
                sentinel_marker = f'KNXBENCH_SIGTERM_SENTINEL={directory}'.encode()
                sentinel = subprocess.Popen(
                    [sys.executable, '-c', 'import time; time.sleep(8)'],
                    env={**self.clean_environment(), 'KNXBENCH_SIGTERM_SENTINEL': marker})
                fixture_marker = f'KNXBENCH_BROWSER_TMP_DIR={directory}'.encode()
                wrapper_identity = open_owned_process(wrapper.pid, fixture_marker)
                child_identity = open_owned_process(child_pid, fixture_marker)
                sentinel_identity = open_owned_process(sentinel.pid, sentinel_marker)
                if wrapper_identity is None or child_identity is None or sentinel_identity is None:
                    self.fail('diagnostic regression identities changed before pidfd retention')

                def mismatched_pidfd_open(pid: int, flags: int = 0) -> int:
                    target = sentinel.pid if pid == child_pid else pid
                    pidfd = real_pidfd_open(target, flags)
                    scanned_pidfds.append(pidfd)
                    return pidfd

                def reused_pid_kill(pid: int, signum: int) -> None:
                    identity = sentinel_identity if pid == child_pid else wrapper_identity
                    signal.pidfd_send_signal(identity.pidfd, signum)

                with mock.patch.object(os, 'pidfd_open', side_effect=mismatched_pidfd_open), \
                        mock.patch.object(os, 'kill', side_effect=reused_pid_kill):
                    diagnosis = diagnose_stuck_wrapper(wrapper, directory)
                self.assertIn('wrapper pipes still open 10 s after SIGTERM', diagnosis)
                self.assertEqual(running_owned_processes([sentinel_identity]), [sentinel_identity],
                                 'diagnostics signalled an unrelated process after PID reuse')
                self.assertGreaterEqual(len(scanned_pidfds), 2,
                                        'diagnostics did not inspect wrapper and child identities')
                for pidfd in scanned_pidfds:
                    with self.assertRaises(OSError, msg='diagnostics leaked a scanned pidfd'):
                        os.fstat(pidfd)
            finally:
                identities = [identity for identity in (
                    wrapper_identity, child_identity, sentinel_identity) if identity is not None]
                signal_owned_processes(running_owned_processes(identities), signal.SIGKILL)
                wait_for_owned_processes(identities)
                wrapper.wait(timeout=10)
                if sentinel is not None:
                    sentinel.wait(timeout=10)
                close_owned_processes(identities)

    def test_multibyte_and_whitespace_paths_use_byte_budget_and_json(self) -> None:
        with self.workspace() as tmp:
            work = Path(tmp)
            # Its character count fits while its UTF-8 byte count does not.
            remaining = 107 - len(os.fsencode(work)) - 1 - len(browser_tmp.CHROMIUM_SOCKET_SUFFIX)
            bad = work / ('ä' * (remaining // 2 + 1))
            bad.mkdir(mode=0o700)
            self.assertLessEqual(len(str(bad) + browser_tmp.CHROMIUM_SOCKET_SUFFIX), 107)
            self.assertGreater(len(os.fsencode(str(bad) + browser_tmp.CHROMIUM_SOCKET_SUFFIX)), 107)
            with self.assertRaisesRegex(ValueError, 'socket'):
                browser_tmp.allocate_browser_directory(bad, explicit=bad)
            spaced = work / '\n'
            spaced.mkdir(mode=0o700)
            env = {**self.clean_environment(), 'TMPDIR': str(spaced),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(spaced)}
            result = subprocess.run(
                ['node', '--require', str(ROOT / 'tools/browser_tmp.cjs'), '-e',
                 'console.log(JSON.stringify(process.env.TMPDIR))'],
                cwd=work, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            directory = Path(json.loads(result.stdout))
            if browser_tmp.fits_temp_root(spaced):
                self.assertEqual(directory.parent, spaced)
                self.assertFalse(directory.exists())
            else:
                self.assertEqual(directory, spaced)
            self.assertTrue(spaced.exists())

    def test_standalone_repository_launchers_load_guard_before_playwright(self) -> None:
        for relative in (
            'apps/knx-web/scripts/workbench-browser-proof.mjs',
            'design/verify-crt-interactions.mjs',
            'design/verify-crt-reference.mjs',
            'story/tests/browser/check_story.mjs',
        ):
            with self.subTest(path=relative):
                source = (ROOT / relative).read_text()
                self.assertTrue('browser_tmp.cjs' in source, 'standalone launch bypasses temp guard')
                self.assertLess(source.index('browser_tmp.cjs'), source.index('chromium.launch')
                                if 'chromium.launch' in source else source.index('require("playwright")'))


if __name__ == '__main__':
    unittest.main()
