#!/usr/bin/env python3
"""Refuse local machine details (absolute local paths, host name, kernel release) in tracked files and commit messages.

The repository is public. A path such as a developer's mount point or home
directory, the host name or the exact kernel release tells readers about one
machine and nothing about KNXBench. Write a placeholder instead
(`<sources>`, `<profile>`, `~`, `<host>`, "the dev machine").

Two kinds of rule apply:

* generic patterns that need no knowledge of any machine (local mount points,
  home directories other than obvious placeholders, agent profile paths);
* the identifiers of the machine running the check, read at runtime so this
  file never has to contain them: host name, kernel release, `$HOME` and the
  directory the checkout lives in. `KNXBENCH_LOCAL_INFO_DENY` adds further
  colon-separated tokens (for example an old mount point) without committing
  them anywhere.

A line that deliberately shows such a detail as an example (the guard's own
patterns and test fixtures) carries the marker `local-info: allow`; the marker
exempts that line only and is visible in review.

Usage:
  tools/check_local_info.py                     # all tracked text files
  tools/check_local_info.py --message-file F    # one commit message (commit-msg hook)
  tools/check_local_info.py --commits A..B      # messages of a commit range
"""

import argparse
from dataclasses import dataclass
import os
from pathlib import Path
import platform
import re
import socket
import subprocess

# Home directory names that are obviously invented for tests and examples.
PLACEHOLDER_HOMES = ("alice", "alicea", "bob", "someone", "user", "u", "private",
                     "customer", "knx", "system", "example")
# Mount names that are not a developer machine: synthetic test input and the
# path ETS itself writes inside its archives.
PLACEHOLDER_MOUNTS = ("private", "system")
BOUNDARY = r"(?<![\w.~-])"
END = r"(?![A-Za-z0-9])"

GENERIC_RULES = (
    ("local mount path", re.compile(
        BOUNDARY + r"/(?:mnt|media|run/media)/(?!(?:" + "|".join(PLACEHOLDER_MOUNTS) + r")" + END + r")"
        r"[A-Za-z0-9][A-Za-z0-9_-]*")),
    ("home directory", re.compile(
        BOUNDARY + r"/home/(?!(?:" + "|".join(PLACEHOLDER_HOMES) + r")" + END + r")[A-Za-z0-9][A-Za-z0-9._-]*")),
    ("agent profile path", re.compile(r"\.hermes/profiles/")),  # local-info: allow
)
# Directories that say nothing about a particular machine.
GENERIC_DIRS = {"/", "/root", "/home", "/tmp", "/srv", "/opt", "/mnt", "/media", "/workspace", "/build"}
MIN_TOKEN = 4
ALLOW_MARKER = "local-info: allow"


@dataclass(frozen=True)
class Finding:
    where: str
    line: int
    kind: str
    text: str

    def __str__(self) -> str:
        return f"{self.where}:{self.line}: {self.kind}: {self.text}"


def machine_rules(root: Path) -> list[tuple[str, re.Pattern]]:
    """Identifiers of the machine running the check, as literal patterns."""
    tokens: list[tuple[str, str]] = []
    host = socket.gethostname().split(".")[0]
    if len(host) >= MIN_TOKEN and host != "localhost" and not host.isdigit():
        tokens.append(("host name", host))
    release = platform.release()
    if len(release) >= MIN_TOKEN and "-" in release:
        tokens.append(("kernel release", release))
    for kind, directory in (("home directory", str(Path.home())),
                            ("checkout location", str(root.absolute().parent)),
                            ("checkout location", str(root.resolve().parent))):
        if directory not in GENERIC_DIRS and len(directory) >= MIN_TOKEN:
            tokens.append((kind, directory))
    for extra in os.environ.get("KNXBENCH_LOCAL_INFO_DENY", "").split(":"):
        if len(extra.strip()) >= MIN_TOKEN:
            tokens.append(("denied token", extra.strip()))
    seen, rules = set(), []
    for kind, token in tokens:
        if token not in seen:
            seen.add(token)
            rules.append((kind, re.compile(r"(?<![\w-])" + re.escape(token) + r"(?![\w])")))
    return rules


def scan_text(where: str, text: str, rules) -> list[Finding]:
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        if ALLOW_MARKER in line:
            continue
        for kind, pattern in rules:
            match = pattern.search(line)
            if match:
                findings.append(Finding(where, number, kind, match.group(0)))
    return findings


def tracked_files(root: Path) -> list[str]:
    listing = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True).stdout
    return [name for name in listing.decode().split("\0") if name]


def read_text(path: Path) -> str | None:
    if path.is_symlink() or not path.is_file():
        return None
    raw = path.read_bytes()
    if b"\0" in raw:
        return None
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return None


def check(root: Path, rules=None) -> list[Finding]:
    """All findings in the tracked text files of the checkout at `root`."""
    rules = list(GENERIC_RULES) + machine_rules(root) if rules is None else rules
    findings = []
    for name in tracked_files(root):
        text = read_text(root / name)
        if text is not None:
            findings.extend(scan_text(name, text, rules))
    return findings


def commit_messages(root: Path, revisions: str) -> list[tuple[str, str]]:
    log = subprocess.run(["git", "log", "-z", "--format=%h%n%B", revisions], cwd=root,
                         check=True, capture_output=True).stdout.decode()
    messages = []
    for entry in log.split("\0"):
        if entry.strip():
            sha, _, body = entry.partition("\n")
            messages.append((sha, body))
    return messages


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--message-file", type=Path, help="check one commit message instead of the tree")
    parser.add_argument("--commits", metavar="RANGE", help="check the messages of a commit range")
    args = parser.parse_args()
    rules = list(GENERIC_RULES) + machine_rules(args.root)
    if args.message_file:
        lines = [line for line in args.message_file.read_text().splitlines() if not line.startswith("#")]
        findings, scope = scan_text("commit message", "\n".join(lines), rules), "commit message"
    elif args.commits:
        messages = commit_messages(args.root, args.commits)
        findings = [f for sha, body in messages for f in scan_text(f"commit {sha}", body, rules)]
        scope = f"{len(messages)} commit messages"
    else:
        findings, scope = check(args.root, rules), "tracked files"
    for finding in findings:
        print("ERROR:", finding)
    print(f"{'FAIL' if findings else 'PASS'}: {len(findings)} local machine details in {scope}.")
    if findings:
        print("Use a placeholder such as <sources>, <profile>, ~ or <host> instead.")
    return int(bool(findings))


if __name__ == "__main__":
    raise SystemExit(main())
