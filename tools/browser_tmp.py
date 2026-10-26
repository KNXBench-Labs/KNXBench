#!/usr/bin/env python3
"""Manage socket-budgeted browser temp directories without system-temp fallbacks."""
from __future__ import annotations

import argparse
from collections.abc import Iterator
from contextlib import contextmanager
import json
import os
from pathlib import Path
import shutil
import signal
import stat
import subprocess
import tempfile

# Linux AF_UNIX sun_path is 108 bytes including its terminating NUL.
MAX_SOCKET_BYTES = 107
CHROMIUM_SOCKET_SUFFIX = '/org.chromium.Chromium.XXXXXX/SingletonSocket'
# Python tempfile uses eight random characters; Node's six-character suffix fits too.
OWNED_TEMP_SUFFIX = '/wxxxxxxxx'


def fits_temp_root(root: Path) -> bool:
    return len(os.fsencode(str(root.resolve()) + OWNED_TEMP_SUFFIX + CHROMIUM_SOCKET_SUFFIX)) <= MAX_SOCKET_BYTES


def approved_scratch(path: Path) -> Path | None:
    """Canonical target of a `scratch` component, or None when it is not approved."""
    if path.is_symlink() and path.lstat().st_uid != os.getuid():
        return None
    try:
        target = path.resolve(strict=True)
    except OSError:
        return None
    if not target.is_dir():
        return None
    mode = target.stat()
    if mode.st_uid != os.getuid() or mode.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
        return None
    return target


def scratch_candidates(base: Path) -> list[Path]:
    """Owned, non-group/world-writable `scratch` ancestors, nearest first.

    The configured (lexical) path is walked before the canonical one, so a
    `scratch` component that is a user-owned symlink, e.g. a scratch moved to
    another disk, keeps the approval its name carries; its target is checked.
    """
    lexical = Path(os.path.abspath(base))
    canonical = base.resolve(strict=True)
    candidates: list[Path] = []
    for parent in [*lexical.parents, *canonical.parents]:
        if parent.name != 'scratch':
            continue
        target = approved_scratch(parent)
        if target is not None and target not in candidates:
            candidates.append(target)
    return candidates


def select_temp_root(base: Path, explicit: Path | None = None) -> Path:
    candidate = (explicit or base).resolve(strict=True)
    if not candidate.is_dir():
        raise ValueError('browser temporary root is not a directory')
    if fits_temp_root(candidate):
        return candidate
    if explicit is not None:
        raise ValueError('explicit browser temporary root exceeds the Chromium socket budget')
    for parent in scratch_candidates(base):
        if fits_temp_root(parent):
            return parent
    raise ValueError('Chromium needs a short approved temp root; set KNXBENCH_BROWSER_TMP_ROOT or --temp-root')


def validate_owned_directory(directory: Path) -> Path:
    directory = directory.resolve(strict=True)
    mode = directory.stat()
    if not directory.is_dir() or mode.st_uid != os.getuid() or mode.st_mode & 0o077:
        raise ValueError('parent-owned browser directory must be private to the current user')
    if len(os.fsencode(str(directory) + CHROMIUM_SOCKET_SUFFIX)) > MAX_SOCKET_BYTES:
        raise ValueError('parent-owned browser directory exceeds the Chromium socket budget')
    return directory


def allocate_browser_directory(base: Path, explicit: Path | None = None,
                               reuse_owned: bool = False) -> tuple[Path, bool]:
    """Return (directory, created_here); never delete a caller-owned directory.

    A private explicit root can already be a usable leaf even when another
    temporary component would exhaust the socket budget. Do not reject that
    leaf merely because it cannot serve as a parent for one more directory.
    """
    if reuse_owned:
        return validate_owned_directory(base), False
    if explicit is not None and not fits_temp_root(explicit):
        return validate_owned_directory(explicit), False
    parent = select_temp_root(base, explicit)
    directory = Path(tempfile.mkdtemp(prefix='w', dir=parent))
    try:
        return validate_owned_directory(directory), True
    except (OSError, ValueError):
        shutil.rmtree(directory)
        raise


@contextmanager
def browser_directory(base: Path, explicit: Path | None = None,
                      reuse_owned: bool = False) -> Iterator[Path]:
    directory, created = allocate_browser_directory(base, explicit, reuse_owned)
    try:
        yield directory
    finally:
        if created:
            shutil.rmtree(directory)


def run_command(command: list[str], directory: Path) -> int:
    """Wait for an owned child group before releasing its temporary directory."""
    env = {**os.environ, 'TMPDIR': str(directory), 'KNXBENCH_BROWSER_TMP_DIR': str(directory)}
    # Forwarders go in before the spawn: a signal landing between the child's creation and a
    # later installation would reach the caller's disposition and never the child group. One
    # that arrives before the group is known is kept and forwarded once it is.
    group: list[int] = []
    pending: list[int] = []
    previous = {}

    def forward(signum: int, frame: object) -> None:
        if not group:
            pending.append(signum)
            return
        try:
            os.killpg(group[0], signum)
        except ProcessLookupError:
            pass

    child = None
    try:
        for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            previous[signum] = signal.signal(signum, forward)
        child = subprocess.Popen(command, env=env, start_new_session=True)
        group.append(child.pid)
        for signum in pending:
            forward(signum, None)
        status = child.wait()
        return status if status >= 0 else 128 - status
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', type=Path, default=Path(tempfile.gettempdir()))
    parser.add_argument('--reuse-owned', action='store_true')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--allocate', action='store_true', help='JSON directory/ownership for a managing parent')
    mode.add_argument('--run', nargs=argparse.REMAINDER, help='run a command with managed browser temp/cleanup')
    args = parser.parse_args()
    explicit_value = os.environ.get('KNXBENCH_BROWSER_TMP_ROOT')
    explicit = Path(explicit_value) if explicit_value else None
    try:
        if args.run is not None:
            if not args.run:
                parser.error('--run requires a command')
            parent_owned = os.environ.get('KNXBENCH_BROWSER_TMP_DIR')
            base = Path(parent_owned) if parent_owned else args.base
            with browser_directory(base, explicit, args.reuse_owned or bool(parent_owned)) as directory:
                return run_command(args.run, directory)
        if args.allocate:
            directory, created = allocate_browser_directory(args.base, explicit, args.reuse_owned)
            print(json.dumps({'directory': str(directory), 'created': created}))
        elif args.reuse_owned:
            print(json.dumps(str(validate_owned_directory(args.base))))
        else:
            print(json.dumps(str(select_temp_root(args.base, explicit))))
    except (OSError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
