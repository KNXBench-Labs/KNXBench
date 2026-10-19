"""Keep test stages away from the developer's real KNXBench data and config.

The apps resolve their user files as `$XDG_DATA_HOME/knx/` (product
database) and `$XDG_CONFIG_HOME/knx/` (remembered legacy password), falling
back to `~/.local/share` and `~/.config`; an empty variable counts as unset.
A test that spawns a `knx`, server or desktop binary without its own homes
opens those real files and may migrate them to the branch's schema (it did,
twice, on 19 October 2026). Gates therefore run test stages with empty
scratch homes and compare a snapshot of the real directories before and
after the run.

A snapshot never raises. What it cannot cover (a directory it may not list,
a symlink to an ancestor of the knx directory or of `$HOME`, more entries
than `MAX_ENTRIES`) is recorded as a coverage problem, and a problem fails
the receipt just like a change: an unchecked directory is not a clean one.
"""
from __future__ import annotations

from dataclasses import dataclass, field
import os
from pathlib import Path

USER_HOMES = (('XDG_DATA_HOME', '.local/share'), ('XDG_CONFIG_HOME', '.config'))
# The knx directories hold a product database with its WAL/SHM, a remembered
# password and a few settings; thousands of entries mean a link led elsewhere.
MAX_ENTRIES = 5_000


@dataclass
class Snapshot:
    entries: dict[str, list] = field(default_factory=dict)
    problems: list[str] = field(default_factory=list)


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


def _reason(error: OSError) -> str:
    return error.strerror or type(error).__name__


def _entry(path: Path) -> list:
    """The entry itself and, for a symlink, its text and what it resolves to."""
    entry: list = _identity(path.lstat())
    if path.is_symlink():
        entry.append(os.readlink(path))
        try:
            entry.extend(_identity(path.stat()))
        except FileNotFoundError:
            entry.append(None)  # dangling
    return entry


def user_data_snapshot(dirs: list[Path], home: str | None = None) -> Snapshot:
    """Every entry below `dirs`. Symlinked directories are followed once unless
    they resolve to an ancestor of the knx directory or of `home`. Never raises."""
    snapshot = Snapshot()
    seen: set[tuple[int, int]] = set()
    home_path = Path(home).resolve() if home else None
    for top in dirs:
        try:
            if not top.exists() and not top.is_symlink():
                continue  # nothing there yet: a file created later shows as a change
            guards = [top.resolve().parent, *([home_path] if home_path else [])]
        except OSError as error:
            snapshot.problems.append(f'{top}: cannot be checked ({_reason(error)})')
            continue
        pending = [top]
        while pending:
            directory = pending.pop()
            try:
                status = directory.stat()
                if (status.st_dev, status.st_ino) in seen:
                    continue
                seen.add((status.st_dev, status.st_ino))
                children = sorted(os.scandir(directory), key=lambda child: child.name)
            except FileNotFoundError:
                continue  # removed while scanning: the other snapshot shows the difference
            except OSError as error:
                snapshot.problems.append(f'{directory}: cannot be listed ({_reason(error)})')
                continue
            for child in children:
                path = Path(child.path)
                try:
                    entry = _entry(path)
                    if path.is_symlink():
                        snapshot.entries[str(path)] = entry  # retargeting a link is a change
                    if path.is_dir():
                        real = path.resolve()
                        if path.is_symlink() and any(guard == real or guard.is_relative_to(real)
                                                     for guard in guards):
                            snapshot.problems.append(
                                f'{path}: links to {real}, an ancestor of the knx directory or of HOME; '
                                f'not followed, so the knx data cannot be fully checked')
                        else:
                            pending.append(path)
                        continue
                except FileNotFoundError:
                    continue  # removed while scanning
                except OSError as error:
                    snapshot.problems.append(f'{path}: cannot be checked ({_reason(error)})')
                    continue
                snapshot.entries[str(path)] = entry
                if len(snapshot.entries) > MAX_ENTRIES:
                    snapshot.problems.append(f'more than {MAX_ENTRIES} entries below {top}; a link probably '
                                             f'leads outside the knx data, which cannot be fully checked')
                    return snapshot
    return snapshot


def touched_user_data(before: dict[str, list], after: dict[str, list]) -> list[str]:
    """Paths created, changed or removed between two snapshots' entries."""
    return sorted(path for path in before.keys() | after.keys() if before.get(path) != after.get(path))


def record_user_data(receipt: dict, before: Snapshot, after: Snapshot) -> None:
    """Fail the receipt when the real knx directories changed or could not be checked."""
    touched = touched_user_data(before.entries, after.entries)
    problems = list(dict.fromkeys(before.problems + after.problems))
    receipt['userDataTouched'] = touched
    receipt['userDataUnchecked'] = problems
    reasons = []
    if touched:
        reasons.append(f'the real knx data or config directories changed during the gate: '
                       f'{", ".join(touched)} (a stage reached them, or an app ran meanwhile)')
    if problems:
        reasons.append(f'the real knx data or config directories could not be fully checked: '
                       f'{"; ".join(problems)}')
    if reasons:
        receipt['planCompleted'] = False
        receipt['error'] = '; '.join(filter(None, (receipt.get('error'), *reasons)))
