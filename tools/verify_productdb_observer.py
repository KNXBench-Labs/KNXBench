#!/usr/bin/env python3
"""Opt-in Linux synthetic controls, NOT a production study or RSS baseline.

Run with --supervisor ABS_NATIVE_HELPER --output ABS_NEW_PRIVATE_DIRECTORY.
No build, installation, migration, corpus or bus access. Raw proofs stay private.
"""
from __future__ import annotations

import argparse
import ctypes
from contextlib import contextmanager
import gc
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
if __package__ in (None, ''):
    sys.path.insert(0, str(ROOT))
from tools.productdb_process_observer import observed_step
from tools.web_gate_support import MeasuredRun

CONTROLLER_SECONDS = 30
CONTROLLER_HARD_SECONDS = 60  # Includes MeasuredRun's five-second TERM escalation.
CONTROLLERS = ('calibration', 'cancellation', 'term_ignore', 'timeout', 'wait_failure')

PYTHON = str(Path(sys.executable).resolve())
TARGET = 'import os,json,signal; print(json.dumps(dict(pid=os.getpid())),flush=True); '
TREE = '''import os,json,signal
child=os.fork()
if child==0:
    while True: signal.pause()
print(json.dumps(dict(pid=os.getpid(),child=child,pgid=os.getpgrp(),sid=os.getsid(0))),flush=True)
while True: signal.pause()
'''


def require(condition):
    if not condition:
        raise ValueError('observer control proof rejected')


def write(path, value):
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def identity(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return {'path': str(path), 'sha256': digest.hexdigest()}


def source_manifest():
    # Explicit executing modules, not a repository snapshot or build-provenance claim.
    paths = (Path(__file__).resolve(),
             Path(sys.modules[observed_step.__module__].__file__).resolve(),
             Path(sys.modules[MeasuredRun.__module__].__file__).resolve())
    return {path.name: identity(path) for path in paths}


def checked_paths(helper, output):
    for path in (helper, output):
        require(path.is_absolute() and path == path.resolve() and '..' not in path.parts)
    require(helper.is_file() and os.access(helper, os.X_OK))
    with helper.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF')
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir())
    for protected in (ROOT, helper.parent):
        require(not output.is_relative_to(protected) and not protected.is_relative_to(output))


def validate_calibration(values):
    tiny = [values[n]['target']['peakRssKiB']
            for n in ('fresh', 'held64MiB', 'freed64MiB', 'afterHeavyChild')]
    require(all(type(v) is int and 0 < v < 16 * 1024 for v in tiny))
    require(max(tiny) - min(tiny) < 8 * 1024)
    require(values['heavyTarget']['target']['peakRssKiB'] - tiny[0] > 48 * 1024)
    for name in ('held64MiB', 'freed64MiB'):
        require(values[name]['wrapper']['peakRssKiB'] - values['fresh']['wrapper']['peakRssKiB'] > 48 * 1024)


def calibration(helper, output):
    run = MeasuredRun(output)
    values = {}

    def observe(name, command, code=0):
        target = observed_step(run, helper, name, command, output, None)
        values[name] = {'target': target, 'wrapper': run.receipt['stages'][-1]}
        write(output / 'results.json', values)  # Retain even a rejected calibration.
        require(target['observerError'] is None and target['exit'] == code)
        require(target['pid'] != target['supervisorPid'])
        require(values[name]['wrapper']['exit'] == (code if code >= 0 else 128 - code))
        if command[0] == PYTHON:
            require(json.loads((output / (name + '.log')).read_text())['pid'] == target['pid'])

    for name in ('fresh', 'held64MiB', 'freed64MiB'):
        if name == 'held64MiB':
            memory = bytearray(64 * 1024 * 1024)
            for index in range(0, len(memory), 4096):
                memory[index] = 13
        if name == 'freed64MiB':
            del memory
            gc.collect()
        observe(name, ['/usr/bin/true'])
    observe('heavyTarget', [PYTHON, '-c', TARGET + 'memory=bytearray(64*1024*1024)'])
    observe('afterHeavyChild', ['/usr/bin/true'])
    for code in (7, 125, 255):
        observe('exit' + str(code), [PYTHON, '-c', TARGET + f'raise SystemExit({code})'], code)
    observe('signal', [PYTHON, '-c', TARGET + 'os.kill(os.getpid(),signal.SIGTERM)'], -signal.SIGTERM)
    validate_calibration(values)


def cancellation(helper, output, *, deadline_control=False):
    # Only this isolated controller adopts/reaps orphaned targets; never the caller.
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
    libc.prctl.restype = ctypes.c_int
    require(libc.prctl(36, 1, 0, 0, 0) == 0)  # PR_SET_CHILD_SUBREAPER
    run = MeasuredRun(output)
    recorded = {}
    deadline = time.monotonic() + 10

    def interrupt(_signum, _frame):
        if deadline_control and recorded:
            signal.setitimer(signal.ITIMER_REAL, 0)
            raise TimeoutError('intentional hanging target deadline')
        if time.monotonic() > deadline:
            signal.setitimer(signal.ITIMER_REAL, 0)
            raise TimeoutError('target tree readiness timeout')
        try:
            tree = json.loads((output / 'tree.log').read_text())
            checkpoint = json.loads((output / 'tree.target.json').read_text())
        except (OSError, ValueError):
            return
        # One shot: a second alarm must never interrupt MeasuredRun's five-second
        # TERM wait and KILL escalation while this first exception unwinds.
        signal.setitimer(signal.ITIMER_REAL, 0)
        wrapper = run.receipt['stages'][0]['pid']
        require(checkpoint == {'format': 1, 'complete': False, 'pid': tree['pid']})
        require(tree['pgid'] == tree['sid'] == wrapper)
        pids = [wrapper, tree['pid'], tree['child']]
        require(len(set(pids)) == 3)
        require(all(os.getpgid(pid) == wrapper and os.getsid(pid) == wrapper for pid in pids))
        recorded.update(wrapperPid=wrapper, targetTree=tree, checkpoint=checkpoint, pids=pids)
        write(output / 'before-interruption.json', recorded)
        if deadline_control:
            # The ready tree deliberately hangs until this one-shot deadline.
            signal.setitimer(signal.ITIMER_REAL, .25)
            return
        raise KeyboardInterrupt

    prior = signal.signal(signal.SIGALRM, interrupt)
    signal.setitimer(signal.ITIMER_REAL, 0.05, 0.05)
    interrupted = timed_out = False
    try:
        observed_step(run, helper, 'tree', [PYTHON, '-c', TREE], output, None)
    except KeyboardInterrupt:
        interrupted = True
    except TimeoutError:
        timed_out = True
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, prior)
        # MeasuredRun has killed the owned group before propagating interruption.
        reaped = []
        until = time.monotonic() + 5
        while recorded and time.monotonic() < until:
            try:
                pid, status = os.waitpid(-recorded['wrapperPid'], os.WNOHANG)
            except ChildProcessError:
                break
            if pid:
                reaped.append({'pid': pid, 'waitStatus': status})
            else:
                time.sleep(0.01)
        absent = [pid for pid in recorded.get('pids', []) if not Path(f'/proc/{pid}').exists()]
        recorded.update(interrupted=interrupted, timedOut=timed_out, reaped=reaped, absentPids=absent)
        write(output / 'results.json', recorded)
    require((timed_out if deadline_control else interrupted) and recorded['pids'] == absent)
    require({r['pid'] for r in reaped} == {recorded['targetTree']['pid'], recorded['targetTree']['child']})
    require(json.loads((output / 'tree.target.json').read_text()) == recorded['checkpoint'])
    require(run.receipt['stages'][0]['exit'] is None)  # Interrupted, not a successful measurement.


@contextmanager
def inherited_signal(helper, signum):
    """Scoped Popen fixture in an isolated controller, not a runtime fault API."""
    from unittest.mock import patch
    original = subprocess.Popen

    def popen(command, *args, **kwargs):
        require(command[0] == str(helper))
        kwargs['preexec_fn'] = lambda: signal.signal(signum, signal.SIG_IGN)
        return original(command, *args, **kwargs)

    with patch.object(subprocess, 'Popen', popen):
        yield


def term_ignore(helper, output):
    with inherited_signal(helper, signal.SIGTERM):
        cancellation(helper, output)


def timeout_control(helper, output):
    cancellation(helper, output, deadline_control=True)


def wait_failure(helper, output):
    run = MeasuredRun(output)
    with inherited_signal(helper, signal.SIGCHLD):
        target = observed_step(run, helper, 'echild', [PYTHON, '-c', TARGET + 'pass'], output, None)
    wrapper = run.receipt['stages'][0]
    checkpoint = json.loads((output / 'echild.target.json').read_text())
    emitted = json.loads((output / 'echild.log').read_text().splitlines()[0])
    pids = [wrapper['pid'], emitted['pid']]
    absent = [pid for pid in pids if not Path(f'/proc/{pid}').exists()]
    write(output / 'results.json', dict(target=target, wrapper=wrapper,
          checkpoint=checkpoint, pids=pids, absentPids=absent))
    require(wrapper['exit'] == 125 and pids[0] != pids[1] and pids == absent)
    require(checkpoint == {'format': 1, 'complete': False, 'pid': emitted['pid']})
    require(target['observerError'] is not None)
    require(all(target[key] is None for key in ('pid', 'exit', 'waitStatus', 'signal',
                                               'peakRssKiB', 'cpuSeconds', 'wallSeconds')))


@contextmanager
def adopting():
    # Scope adoption to these opt-in controllers; never change the test caller.
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0)
    require(libc.prctl(36, 1, 0, 0, 0) == 0)
    try:
        yield
    finally:
        require(libc.prctl(36, previous.value, 0, 0, 0) == 0)


def child_identity(pid):
    # /proc stat's comm can contain spaces/parentheses. Keep kernel start time as
    # well as parent/group/session so a recycled numeric PID is never ownership.
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    return tuple(int(fields[index]) for index in (1, 2, 3, 19))


def pending_group(stage, output, helper):
    """Recover an execed helper adopted after stopping/waiting its controller.

    The native target checkpoint is not needed: the unfinished stage already
    records the full helper argv (including its unique absolute receipt) and cwd.
    Only the kernel-listed direct children of this scoped subreaper are inspected.
    """
    command = stage['command']
    require(command[:4] == [str(helper), '--receipt',
                           str(output / (stage['name'] + '.target.json')), '--'])
    expected = os.fsencode('\0'.join(command) + '\0')
    matches = []
    children = Path(f'/proc/self/task/{os.getpid()}/children').read_text().split()
    for value in children:
        pid = int(value)
        try:
            before = child_identity(pid)
            if before[:3] != (os.getpid(), pid, pid):
                continue
            root = Path(f'/proc/{pid}')
            if ((root / 'cmdline').read_bytes() != expected or
                    os.readlink(root / 'cwd') != stage['cwd']):
                continue
            require(child_identity(pid) == before)
            matches.append((pid, before))
        except ProcessLookupError:
            continue
        except FileNotFoundError:
            continue
    # Do not guess from a native PID, a partial argv or an arbitrary PGID. No
    # match (including a not-yet-spawned stage) is a refused proof, not a skip.
    require(len(matches) == 1)
    return matches[0]


def cleanup_groups(output, helper):
    """Fallback for exact unfinished stages, including spawn-before-PID-save."""
    receipt = output / 'receipt.json'
    reaped = []
    if not receipt.exists():
        return reaped
    for stage in json.loads(receipt.read_text())['stages']:
        if stage['exit'] is not None:
            continue
        group = stage.get('pid')
        if group is None:
            group, owned_identity = pending_group(stage, output, helper)
            # The controller is stopped and waited, the helper is our unreaped
            # child, and no concurrent owner may replace/reap it in this scope.
            require(child_identity(group) == owned_identity)
        require(type(group) is int and group > 1 and stage['command'][0] == str(helper))
        try:
            require(os.getpgid(group) == os.getsid(group) == group)
        except ProcessLookupError:
            pass  # The leader can be gone with live/adopted members left.
        try:
            os.killpg(group, signal.SIGKILL)
        except ProcessLookupError:
            pass
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
        # A living group is not successful cleanup (including non-child members).
        try:
            os.killpg(group, 0)
        except ProcessLookupError:
            continue
        raise ValueError('observer group cleanup incomplete')
    return reaped


def controller_entry(name, helper, output):
    def expired(_signum, _frame):
        signal.setitimer(signal.ITIMER_REAL, 0)
        raise TimeoutError('observer controller deadline')

    with adopting():
        prior = signal.signal(signal.SIGALRM, expired)
        signal.setitimer(signal.ITIMER_REAL, CONTROLLER_SECONDS)
        try:
            {'calibration': calibration, 'cancellation': cancellation, 'term_ignore': term_ignore,
             'timeout': timeout_control, 'wait_failure': wait_failure}[name](helper, output)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, prior)
            cleanup_groups(output, helper)


def run_controller(name, helper, output):
    code = ('from pathlib import Path; import sys; '
            'from tools.verify_productdb_observer import controller_entry; '
            'controller_entry(sys.argv[1],Path(sys.argv[2]),Path(sys.argv[3]))')
    command = [PYTHON, '-c', code, name, str(helper), str(output / name)]
    result = {'name': name, 'command': command, 'hardTimeout': False}
    with adopting(), (output / (name + '-controller.log')).open('wb') as log:
        child = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        result['pid'] = child.pid
        try:
            child.wait(timeout=CONTROLLER_HARD_SECONDS)
        except subprocess.TimeoutExpired:
            result['hardTimeout'] = True
        finally:
            # The fallback owns the controller AND its separately sessioned group.
            # Read checkpoints only after stopping the controller from launching more.
            if child.poll() is None:
                child.kill()
            child.wait()
            result['reaped'] = cleanup_groups(output / name, helper)
        result['exit'] = child.returncode
    return result


def main(argv=None):
    class Parser(argparse.ArgumentParser):
        def error(self, _message):
            raise ValueError('invalid control arguments')

    parser = Parser(description=__doc__)
    parser.add_argument('--supervisor', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    proof = None
    try:
        args = parser.parse_args(argv)
        require(sys.platform == 'linux')
        checked_paths(args.supervisor, args.output)
        args.output.mkdir(mode=0o700, exist_ok=False)
        proof = {'format': 1, 'success': False, 'kind': 'synthetic-observer-controls',
                 'helperStart': identity(args.supervisor), 'sourceStart': source_manifest(),
                 'controllers': []}
        write(args.output / 'proof.json', proof)
        for name in CONTROLLERS:
            # Fresh controller: no full-file hashing before its samples.
            result = run_controller(name, args.supervisor, args.output)
            proof['controllers'].append(result)
            write(args.output / 'proof.json', proof)
            require(result['exit'] == 0 and not result['hardTimeout'])
        proof['success'] = True
    except (OSError, ValueError, subprocess.SubprocessError):
        pass
    finally:
        if proof is not None:
            try:
                proof['helperEnd'] = identity(args.supervisor)
                proof['sourceEnd'] = source_manifest()
                proof['success'] = (proof['success'] and proof['helperStart'] == proof['helperEnd']
                                    and proof['sourceStart'] == proof['sourceEnd'])
                write(args.output / 'proof.json', proof)
            except OSError:
                proof['success'] = False
    if not proof or not proof['success']:
        print('observer controls refused or failed', file=sys.stderr)
        return 1
    print('observer controls passed (synthetic controls only)')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
