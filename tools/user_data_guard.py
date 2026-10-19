"""Keep test stages away from the developer's real KNXBench data and config.

The apps resolve their user files as `$XDG_DATA_HOME/knx/` (product
database) and `$XDG_CONFIG_HOME/knx/` (remembered legacy password), falling
back to `~/.local/share` and `~/.config`; an empty variable counts as unset.
A test that spawns a `knx`, server or desktop binary without its own homes
opens those real files and may migrate them to the branch's schema (it did,
twice, on 19 October 2026). Gates therefore run test stages with empty
scratch homes and compare a fingerprint of the real directories before and
after the run.
"""
from __future__ import annotations

import os
from pathlib import Path

USER_HOMES = (('XDG_DATA_HOME', '.local/share'), ('XDG_CONFIG_HOME', '.config'))


def user_knx_dirs(environ: dict[str, str]) -> list[Path]:
    """The real `knx` data and config directories, resolved as the apps resolve them."""
    home = environ.get('HOME') or ''
    dirs = []
    for variable, fallback in USER_HOMES:
        base = environ.get(variable) or (str(Path(home) / fallback) if home else '')
        if base:
            dirs.append(Path(base) / 'knx')
    return dirs


def isolated_environment(env: dict[str, str], scratch: Path) -> dict[str, str]:
    """`env` with empty scratch data and config homes below `scratch`."""
    isolated = dict(env)
    for variable, _ in USER_HOMES:
        directory = scratch / variable.lower().replace('_', '-')
        directory.mkdir(parents=True, exist_ok=True)
        isolated[variable] = str(directory)
    return isolated


def _identity(status: os.stat_result) -> list[int]:
    # ctime moves on every write and cannot be set back from user space.
    return [status.st_size, status.st_mtime_ns, status.st_ctime_ns, status.st_ino]


def user_data_fingerprint(dirs: list[Path]) -> dict[str, list]:
    """Every entry below `dirs`: the entry itself (lstat) and, for a symlink, what
    it points at. Symlinked directories are followed once (cycle-safe)."""
    found: dict[str, list] = {}
    seen: set[tuple[int, int]] = set()
    for top in dirs:
        for directory, subdirs, files in os.walk(top, followlinks=True):
            status = os.stat(directory)
            if (status.st_dev, status.st_ino) in seen:
                subdirs[:] = []
                continue
            seen.add((status.st_dev, status.st_ino))
            for name in sorted(files + subdirs):
                path = Path(directory, name)
                entry = _identity(path.lstat())
                if path.is_symlink():
                    target = os.readlink(path)
                    try:
                        entry += [target, *_identity(path.stat())]
                    except OSError:
                        entry += [target, None]
                elif path.is_dir():
                    continue  # its files are listed when the walk enters it
                found[str(path)] = entry
    return found


def touched_user_data(before: dict[str, list], after: dict[str, list]) -> list[str]:
    """Paths created, changed or removed between two fingerprints."""
    return sorted(path for path in before.keys() | after.keys() if before.get(path) != after.get(path))


def record_user_data(receipt: dict, before: dict[str, list], after: dict[str, list]) -> None:
    """Fail the receipt when the real knx directories changed during the gate."""
    touched = touched_user_data(before, after)
    receipt['userDataTouched'] = touched
    if touched:
        receipt['planCompleted'] = False
        reached = (f'the real knx data or config directories changed during the gate: '
                   f'{", ".join(touched)} (a stage reached them, or an app ran meanwhile)')
        receipt['error'] = '; '.join(filter(None, (receipt.get('error'), reached)))
