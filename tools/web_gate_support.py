"""Measured web-gate evidence: exact browser inventory and unambiguous outcomes."""
from __future__ import annotations

import ctypes
import hashlib
import json
import os
import re
import secrets
from pathlib import Path
import signal
import subprocess
import sys
import time
from typing import Callable, NamedTuple


PR_SET_CHILD_SUBREAPER = 36
TERM_GRACE_SECONDS = 1.0
KILL_WAIT_SECONDS = 4.0
SPAWN_SETTLE_SECONDS = 5.0
SUPERVISOR_CLEANUP_SECONDS = TERM_GRACE_SECONDS + KILL_WAIT_SECONDS + 1.0
STAGE_OWNERS_ENV = 'MEASURED_RUN_STAGE_OWNERS'
HANDLED_SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)


class ProcessIdentity(NamedTuple):
    """One Linux process identity that cannot match a recycled numeric PID."""

    pid: int
    start_ticks: int


class _ProcessInfo(NamedTuple):
    identity: ProcessIdentity
    parent_pid: int
    state: str


def enable_child_subreaper() -> None:
    """Require the Linux ownership primitives used for leak-free gate stages."""
    if sys.platform != 'linux' or not hasattr(os, 'pidfd_open') or not hasattr(signal, 'pidfd_send_signal'):
        raise RuntimeError('Linux child subreaper cleanup with pidfd support is required')
    descriptor = None
    try:
        descriptor = os.pidfd_open(os.getpid())
        signal.pidfd_send_signal(descriptor, 0)
    except OSError as error:
        raise RuntimeError(f'Linux pidfd cleanup is unavailable: {error.strerror}') from error
    finally:
        if descriptor is not None:
            os.close(descriptor)
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        prctl = libc.prctl
    except (AttributeError, OSError) as error:
        raise RuntimeError('Linux child subreaper cleanup is unavailable') from error
    prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
    prctl.restype = ctypes.c_int
    if prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
        error = ctypes.get_errno()
        raise RuntimeError(f'Linux child subreaper setup failed: {os.strerror(error)}')


def _process_info(pid: int) -> _ProcessInfo | None:
    try:
        fields = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()
        return _ProcessInfo(ProcessIdentity(pid, int(fields[19])), int(fields[1]), fields[0])
    except (FileNotFoundError, ProcessLookupError):
        return None


def _process_snapshot() -> dict[int, _ProcessInfo]:
    result = {}
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        info = _process_info(int(entry.name))
        if info is not None:
            result[info.identity.pid] = info
    return result


def _direct_children(snapshot: dict[int, _ProcessInfo] | None = None) -> list[_ProcessInfo]:
    processes = snapshot or _process_snapshot()
    return [info for info in processes.values() if info.parent_pid == os.getpid()]


def _has_stage_owner(pid: int, marker: str) -> bool:
    try:
        entries = (Path('/proc') / str(pid) / 'environ').read_bytes().split(b'\0')
    except (FileNotFoundError, PermissionError, ProcessLookupError):
        return False
    prefix = os.fsencode(STAGE_OWNERS_ENV + '=')
    encoded = marker.encode()
    return any(encoded in entry[len(prefix):].split(b':')
               for entry in entries if entry.startswith(prefix))


def _signal_process(identity: ProcessIdentity, signum: signal.Signals) -> None:
    """Signal exactly the start-time-validated process retained by a pidfd."""
    before = _process_info(identity.pid)
    if before is None or before.identity != identity or before.state == 'Z':
        return
    descriptor = None
    try:
        descriptor = os.pidfd_open(identity.pid)
        after = _process_info(identity.pid)
        if after is not None and after.identity == identity:
            signal.pidfd_send_signal(descriptor, signum)
    except ProcessLookupError:
        pass
    finally:
        if descriptor is not None:
            os.close(descriptor)


def _spawn_owned(processes: _StageProcessTree, command: list[str], **kwargs) -> subprocess.Popen:
    """Defer handled signals until the spawned process identity is retained."""
    previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, HANDLED_SIGNALS)
    try:
        restored = ','.join(str(int(signum)) for signum in sorted(previous_mask))
        trampoline = [sys.executable, str(Path(__file__).resolve()),
                      '--restore-signal-mask', restored, '--', *command]
        child = subprocess.Popen(trampoline, **kwargs)
        processes.attach(child.pid)
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)
    return child


class _StageProcessTree:
    """Own, terminate and reap one synchronous gate stage's complete process tree."""

    def __init__(self, *, exclusive: bool = False) -> None:
        self.marker = secrets.token_hex(16)
        self.owned: set[ProcessIdentity] = set()
        self.exclusive = exclusive

    def environment(self, env: dict[str, str] | None) -> dict[str, str]:
        stage_env = dict(os.environ if env is None else env)
        inherited = stage_env.get(STAGE_OWNERS_ENV)
        stage_env[STAGE_OWNERS_ENV] = f'{inherited}:{self.marker}' if inherited else self.marker
        return stage_env

    def attach(self, pid: int) -> None:
        info = _process_info(pid)
        if info is None:
            raise RuntimeError('gate stage exited before its process identity could be recorded')
        self.owned.add(info.identity)

    def _discover(self) -> dict[ProcessIdentity, _ProcessInfo]:
        snapshot = _process_snapshot()
        found = {identity: snapshot[identity.pid] for identity in self.owned
                 if identity.pid in snapshot and snapshot[identity.pid].identity == identity}
        for info in _direct_children(snapshot):
            if (self.exclusive or info.identity in self.owned
                    or _has_stage_owner(info.identity.pid, self.marker)):
                found[info.identity] = info
        changed = True
        while changed:
            changed = False
            parents = {info.identity.pid for info in found.values()}
            for info in snapshot.values():
                if info.parent_pid in parents and info.identity not in found:
                    found[info.identity] = info
                    changed = True
        self.owned.update(found)
        return found

    def _reap(self, child: subprocess.Popen | None) -> None:
        if child is not None:
            child.poll()
        for info in _direct_children():
            if (info.identity not in self.owned or
                    (child is not None and info.identity.pid == child.pid) or info.state != 'Z'):
                continue
            try:
                os.waitpid(info.identity.pid, os.WNOHANG)
            except ChildProcessError:
                pass

    def _wait(self, child: subprocess.Popen | None, deadline: float,
              signum: signal.Signals, await_owner: bool = False) -> bool:
        signalled: set[ProcessIdentity] = set()
        seen = bool(self.owned)
        while True:
            processes = self._discover()
            self._reap(child)
            processes = self._discover()
            if not processes:
                if seen or not await_owner or time.monotonic() >= deadline:
                    return True
                time.sleep(0.01)
                continue
            seen = True
            for identity, info in processes.items():
                if identity not in signalled and info.state != 'Z':
                    _signal_process(identity, signum)
                    signalled.add(identity)
            if time.monotonic() >= deadline:
                return False
            time.sleep(0.01)

    def terminate(self, child: subprocess.Popen | None, await_owner: bool = False,
                  term_grace: float = TERM_GRACE_SECONDS) -> None:
        """Defer cancellation through bounded TERM/KILL/reap, including normal exit."""
        previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, HANDLED_SIGNALS)
        try:
            grace = term_grace + (SPAWN_SETTLE_SECONDS if await_owner else 0.0)
            if self._wait(child, time.monotonic() + grace, signal.SIGTERM, await_owner):
                return
            if self._wait(child, time.monotonic() + KILL_WAIT_SECONDS, signal.SIGKILL):
                return
            survivors = ', '.join(f'{identity.pid}/{identity.start_ticks}'
                                  for identity in self._discover())
            raise RuntimeError(f'gate stage process cleanup incomplete: {survivors}')
        finally:
            # Deliver pending cancellation only after ownership is drained. In
            # particular, normal-completion cleanup must not lose its exclusive
            # subreaper to a forwarded signal before killing/reaping adoptees.
            signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)


def _supervise_stage(report_fd: int, command: list[str]) -> int:
    """Run one stage below an exclusive subreaper and report the target's wait4 result."""
    enable_child_subreaper()
    processes = _StageProcessTree(exclusive=True)
    child = None

    def interrupted(_signum: int, _frame: object) -> None:
        raise KeyboardInterrupt

    for signum in HANDLED_SIGNALS:
        signal.signal(signum, interrupted)
    try:
        try:
            child = _spawn_owned(processes, command, start_new_session=True)
            _, status, usage = os.wait4(child.pid, 0)
            child.returncode = os.waitstatus_to_exitcode(status)
            processes.terminate(child)
        except BaseException as error:
            if child is not None or isinstance(error, (KeyboardInterrupt, SystemExit)):
                processes.terminate(child, await_owner=child is None)
            raise
        result = {'format': 1, 'exit': child.returncode,
                  'cpuSeconds': usage.ru_utime + usage.ru_stime,
                  'peakRssKiB': usage.ru_maxrss}
        os.write(report_fd, json.dumps(result).encode())
        return 0
    finally:
        os.close(report_fd)


def _supervisor_command(report_fd: int, command: list[str]) -> list[str]:
    return [sys.executable, str(Path(__file__).resolve()), '--stage-supervisor',
            str(report_fd), '--', *command]


def _read_supervisor_result(report_fd: int) -> dict:
    chunks = []
    while chunk := os.read(report_fd, 4096):
        chunks.append(chunk)
    try:
        result = json.loads(b''.join(chunks))
        if (result.get('format') != 1 or type(result.get('exit')) is not int
                or not isinstance(result.get('cpuSeconds'), (int, float))
                or type(result.get('peakRssKiB')) is not int):
            raise ValueError
    except (AttributeError, TypeError, ValueError, json.JSONDecodeError) as error:
        raise RuntimeError('gate stage supervisor returned no valid wait result') from error
    return result


class MeasuredRun:
    """One immutable attempt directory, checkpointed after every real command."""

    def __init__(self, output: Path, expected: list[str] | None = None) -> None:
        enable_child_subreaper()
        output.mkdir(parents=True, exist_ok=False)
        self.output = output
        self.expected = expected
        self.receipt: dict = {'format': 1, 'success': False, 'complete': False,
                              'queueWaitSeconds': 0.0, 'stages': []}
        self.save()

    def save(self) -> None:
        temporary = self.output / 'receipt.tmp'
        temporary.write_text(json.dumps(self.receipt, indent=2) + '\n')
        temporary.replace(self.output / 'receipt.json')

    def step(self, name: str, command: list[str], cwd: Path,
             env: dict[str, str] | None = None,
             verify: Callable[[], object] | None = None) -> dict:
        if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*', name):
            raise ValueError('invalid stage name')
        if any(s['name'] == name for s in self.receipt['stages']):
            raise ValueError('duplicate stage name')
        start = time.monotonic()
        stage: dict = {'name': name, 'command': command, 'cwd': str(cwd), 'exit': None,
                       'startedAtUnixSeconds': time.time(), 'loadAtStart': os.getloadavg()}
        self.receipt['stages'].append(stage)
        self.save()
        processes = _StageProcessTree()
        child = None
        report_read, report_write = os.pipe()
        with (self.output / f'{name}.log').open('wb') as log:
            try:
                child = _spawn_owned(
                    processes, _supervisor_command(report_write, command), cwd=cwd,
                    env=processes.environment(env), stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(report_write,))
                os.close(report_write)
                report_write = -1
                processes.attach(child.pid)
                # The PID checkpoint is inside the guarded block: a signal arriving
                # right after it is written (a caller may act on the receipt at
                # once) must still take the child's whole tree down.
                stage['pid'] = child.pid
                self.save()
                _, status, _usage = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
                if child.returncode != 0:
                    raise RuntimeError(f'gate stage supervisor exited with {child.returncode}')
                result = _read_supervisor_result(report_read)
            except BaseException as error:
                if child is not None or isinstance(error, (KeyboardInterrupt, SystemExit)):
                    processes.terminate(child, await_owner=child is None,
                                        term_grace=SUPERVISOR_CLEANUP_SECONDS)
                raise
            finally:
                if report_write >= 0:
                    os.close(report_write)
                os.close(report_read)
            processes.terminate(child, term_grace=SUPERVISOR_CLEANUP_SECONDS)
        stage.update(exit=result['exit'], wallSeconds=time.monotonic() - start,
                     cpuSeconds=result['cpuSeconds'], peakRssKiB=result['peakRssKiB'],
                     proofValid=result['exit'] == 0)
        if result['exit'] == 0 and verify is not None:
            try:
                stage['evidence'] = verify()
            except (ValueError, KeyError, TypeError, OSError) as error:
                stage['proofValid'] = False
                stage['proofError'] = str(error)
        self.save()
        return stage

    def empty(self, name: str, reason: str) -> dict:
        """Record a planned lane whose compiled inventory holds no case.

        No command runs, so there is nothing to time; the stage carries the
        inventory-backed reason instead of pretending to have executed.
        """
        if any(s['name'] == name for s in self.receipt['stages']):
            raise ValueError('duplicate stage name')
        stage = {'name': name, 'command': None, 'exit': 0, 'wallSeconds': 0.0,
                 'proofValid': True, 'evidence': 0, 'empty': reason}
        self.receipt['stages'].append(stage)
        self.save()
        return stage

    def finish(self, before: str, after: str) -> bool:
        stages = self.receipt['stages']
        complete = bool(stages) and (self.expected is None or
                                    [s['name'] for s in stages] == self.expected)
        self.receipt.update(complete=complete, inputStart=before, inputEnd=after,
                            inputsFrozen=before == after,
                            success=complete and before == after and
                            all(s.get('proofValid') and s['exit'] == 0 for s in stages))
        self.save()
        return self.receipt['success']


def validate_vitest(report: dict) -> int:
    """Zero exit is insufficient without a nonempty, fully passed unit report."""
    total = report.get('numTotalTests', 0)
    if (type(total) is not int or total <= 0 or report.get('numPassedTests') != total
            or report.get('numFailedTests') != 0 or report.get('numPendingTests') != 0
            or report.get('numTotalTestSuites', 0) <= 0 or report.get('success') is not True):
        raise ValueError('missing, failed or skipped Vitest proof')
    return total


def source_fingerprint(root: Path) -> str:
    """Hash current Git-selected files, not a moving remote-tracking reference."""
    listing = subprocess.run(['git', 'ls-files', '-z', '--cached', '--others',
                              '--exclude-standard'], cwd=root, check=True,
                             capture_output=True).stdout
    head = subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=root,
                          capture_output=True).stdout
    digest = hashlib.sha256(head)
    for name in sorted(set(listing.split(b'\0')) - {b''}):
        file = root / os.fsdecode(name)
        digest.update(name + b'\0')
        if file.is_symlink():
            digest.update(b'link\0' + os.fsencode(os.readlink(file)))
        elif file.is_file():
            digest.update(str(file.stat().st_mode).encode() + b'\0')
            with file.open('rb') as content:
                for block in iter(lambda: content.read(1024 * 1024), b''):
                    digest.update(block)
        else:
            digest.update(b'missing\0')
    return digest.hexdigest()


EXCLUSIVE_TAG = 'exclusive'


def split_lanes(cases: dict[str, dict]) -> tuple[dict[str, dict], dict[str, dict]]:
    """Cases tagged @exclusive run alone; every other case is functional (ADR-0144)."""
    exclusive = {key: case for key, case in cases.items() if EXCLUSIVE_TAG in case['tags']}
    functional = {key: case for key, case in cases.items() if key not in exclusive}
    return functional, exclusive


def browser_cases(report: dict) -> dict[str, dict]:
    """Read compiled Playwright JSON cases, rejecting empty or ambiguous reports."""
    if not isinstance(report, dict) or report.get('errors') or not isinstance(report.get('suites'), list):
        raise ValueError('browser report errors or missing suites')
    cases: dict[str, dict] = {}

    def visit(suites: list[dict]) -> None:
        for suite in suites:
            if (not isinstance(suite, dict) or not isinstance(suite.get('specs', []), list)
                    or not isinstance(suite.get('suites', []), list)):
                raise ValueError('invalid browser suite shape')
            for spec in suite.get('specs', []):
                if not isinstance(spec, dict) or not isinstance(spec.get('tests'), list):
                    raise ValueError('invalid browser spec shape')
                for test in spec['tests']:
                    if not isinstance(test, dict):
                        raise ValueError('invalid browser case shape')
                    key = f'{spec["id"]}:{test["projectId"]}'
                    if key in cases:
                        raise ValueError('duplicate browser case identity')
                    if test.get('expectedStatus') != 'passed':
                        raise ValueError('non-passing browser expectation is not admitted')
                    tags = spec.get('tags', [])
                    if not isinstance(tags, list) or not all(isinstance(t, str) for t in tags):
                        raise ValueError('invalid browser tag list')
                    cases[key] = {'file': spec['file'], 'tags': tags, **test}
            visit(suite.get('suites', []))

    visit(report['suites'])
    if not cases:
        raise ValueError('empty browser inventory')
    return cases


def validate_browser(report: dict, expected: dict[str, dict]) -> int:
    """Every discovered case must pass once, without skips or hidden retries."""
    actual = browser_cases(report)
    if actual.keys() != expected.keys():
        raise ValueError('browser inventory differs from discovery')
    for case in actual.values():
        results = case.get('results', [])
        if (case.get('status') != 'expected' or len(results) != 1
                or results[0].get('status') != 'passed'
                or results[0].get('retry') != 0):
            raise ValueError('browser case failed, skipped, retried or not executed')
    return len(actual)


if __name__ == '__main__':
    if len(sys.argv) >= 5 and sys.argv[1] == '--stage-supervisor' and sys.argv[3] == '--':
        try:
            raise SystemExit(_supervise_stage(int(sys.argv[2]), sys.argv[4:]))
        except KeyboardInterrupt:
            raise SystemExit(130) from None
    if len(sys.argv) >= 5 and sys.argv[1] == '--restore-signal-mask' and sys.argv[3] == '--':
        restored_mask = {int(signum) for signum in sys.argv[2].split(',') if signum}
        signal.pthread_sigmask(signal.SIG_SETMASK, restored_mask)
        os.execvpe(sys.argv[4], sys.argv[4:], os.environ)
    else:
        raise SystemExit('web_gate_support.py is an internal stage supervisor')
