"""Exercise late supervisor cancellation with a real, non-atomic process scan interleaving."""
from __future__ import annotations

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time
from unittest.mock import patch

from tools import web_gate_support as support


def wait_for(predicate, label: str) -> None:
    deadline = time.monotonic() + 8
    while not predicate():
        if time.monotonic() >= deadline:
            raise RuntimeError(f'timed out waiting for {label}')
        time.sleep(.005)


def exercise(root: Path, signum: int, repeated: bool) -> dict:
    stage_ready = root / 'stage.ready'
    spawn_release = root / 'spawn.release'
    descendant_ready = root / 'descendant.ready'
    exit_release = root / 'exit.release'
    term_received = root / 'term.received'
    sentinel_ready = root / 'sentinel.ready'
    sentinel_signal = root / 'sentinel.signal'
    descendant = f'''import os,signal
from pathlib import Path
signal.pthread_sigmask(signal.SIG_BLOCK, {{signal.SIGTERM}})
fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()
p=Path({str(descendant_ready)!r});t=p.with_suffix('.tmp')
t.write_text(f'{{os.getpid()}} {{fields[19]}}');t.replace(p)
while True:
    if signal.sigtimedwait({{signal.SIGTERM}}, .1) is not None:
        Path({str(term_received)!r}).touch()
'''
    stage = f'''import os,subprocess,sys,time
from pathlib import Path
fields=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()
p=Path({str(stage_ready)!r});t=p.with_suffix('.tmp')
t.write_text(f'{{os.getpid()}} {{fields[19]}}');t.replace(p)
while not Path({str(spawn_release)!r}).exists(): time.sleep(.005)
subprocess.Popen([sys.executable,'-c',{descendant!r}],env={{}},start_new_session=True)
while not Path({str(exit_release)!r}).exists(): time.sleep(.005)
os._exit(0)
'''
    sentinel_program = f'''import signal
from pathlib import Path
signals={{signal.SIGTERM,signal.SIGINT,signal.SIGHUP}}
signal.pthread_sigmask(signal.SIG_BLOCK,signals)
Path({str(sentinel_ready)!r}).touch()
while True:
    if signal.sigtimedwait(signals,.1) is not None:
        Path({str(sentinel_signal)!r}).touch()
'''
    run = support.MeasuredRun(root / 'attempt')
    sentinel = subprocess.Popen([sys.executable, '-c', sentinel_program], start_new_session=True)
    sentinel_identity = support._process_info(sentinel.pid).identity
    original_snapshot = support._process_snapshot
    original_signal = support._signal_process
    snapshots = 0
    observations = []
    forwarded = []
    thread_errors = []
    stopping = threading.Event()

    def scheduled_snapshot():
        nonlocal snapshots
        snapshots += 1
        if snapshots != 3:
            return original_snapshot()
        # The first _wait iteration scans before forwarding TERM. Read the new
        # descendant first, then the other processes after its parent is reaped:
        # this is a permitted interleaving of the real non-atomic /proc scan.
        spawn_release.touch()
        wait_for(descendant_ready.exists, 'descendant identity')
        identity = support.ProcessIdentity(*map(int, descendant_ready.read_text().split()))
        before = support._process_info(identity.pid)
        stage_pid = int(stage_ready.read_text().split()[0])
        assert before.identity == identity and before.parent_pid == stage_pid
        exit_release.touch()
        wait_for(lambda: support._process_info(stage_pid) is None, 'normal target reap')
        wait_for(term_received.exists, 'supervisor normal-completion cleanup')
        snapshot = original_snapshot()
        assert stage_pid not in snapshot
        snapshot[identity.pid] = before
        observations.append({'oldParent': stage_pid,
                             'liveParent': support._process_info(identity.pid).parent_pid})
        return snapshot

    def signal_and_yield(identity, forwarded_signal):
        original_signal(identity, forwarded_signal)
        if identity.pid == run.receipt['stages'][0]['pid']:
            forwarded.append(int(forwarded_signal))
            if repeated:
                # Exercise additional real outer cancellation during its cleanup.
                os.kill(os.getpid(), signum)
                os.kill(os.getpid(), signum)
            # Yield the outer runner until the supervisor exits. Otherwise its
            # next scan might rescue the child and hide interrupted ownership.
            wait_for(lambda: (info := support._process_info(identity.pid)) is None
                     or info.identity != identity or info.state == 'Z', 'supervisor exit')

    def interrupt():
        try:
            wait_for(lambda: stage_ready.exists() or stopping.is_set(), 'target ready')
            if not stopping.is_set():
                os.kill(os.getpid(), signum)
        except BaseException as error:
            thread_errors.append(repr(error))

    def interrupted(_signum, _frame):
        raise KeyboardInterrupt

    previous = {sig: signal.signal(sig, interrupted) for sig in support.HANDLED_SIGNALS}
    worker = threading.Thread(target=interrupt)
    error = None
    identities = [sentinel_identity]
    try:
        wait_for(sentinel_ready.exists, 'unrelated signal waiter')
        worker.start()
        with (patch.object(support, '_process_snapshot', scheduled_snapshot),
              patch.object(support, '_signal_process', signal_and_yield)):
            try:
                run.step('race', [sys.executable, '-c', stage], Path.cwd())
            except BaseException as caught:
                error = type(caught).__name__
        worker.join(timeout=10)
        assert not worker.is_alive() and not thread_errors, thread_errors
        identity = support.ProcessIdentity(*map(int, descendant_ready.read_text().split()))
        identities.append(identity)
        info = support._process_info(identity.pid)
        supervisor_pid = run.receipt['stages'][0]['pid']
        supervisor = support._process_info(supervisor_pid)
        if supervisor is not None:
            identities.append(supervisor.identity)
        return {'error': error, 'observations': observations, 'forwarded': forwarded,
                'descendantPresent': info is not None and info.identity == identity,
                'descendantState': info.state if info is not None else None,
                'supervisorPresent': supervisor is not None,
                'unrelatedAlive': sentinel.poll() is None,
                'unrelatedSignalled': sentinel_signal.exists(),
                'receipt': json.loads((root / 'attempt/receipt.json').read_text())}
    finally:
        stopping.set()
        if worker.ident is not None:
            worker.join(timeout=10)
        for sig, handler in previous.items():
            signal.signal(sig, handler)
        # Only fixture identities, with the same pidfd/start-time protections as
        # production; reap a leaked adopted child even when the RED oracle fails.
        for marker in (stage_ready, descendant_ready):
            if marker.exists():
                identities.append(support.ProcessIdentity(*map(int, marker.read_text().split())))
        if run.receipt['stages'] and (pid := run.receipt['stages'][0].get('pid')):
            if (info := support._process_info(pid)) is not None:
                identities.append(info.identity)
        cleanup = support._StageProcessTree()
        cleanup.owned.update(identities)
        cleanup.terminate(None, term_grace=.1)
        sentinel.wait(timeout=5)


if __name__ == '__main__':
    print(json.dumps(exercise(Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3] == 'repeat')))
