#!/usr/bin/env python3
"""Refuse local machine details (local paths, host, kernel, CPU, distribution) in tracked files and commit messages.

The repository is public. A path such as a developer's mount point or home
directory, the host name, the exact kernel release, the CPU model, the
distribution or a real installation's network address tells readers about one
machine and nothing about KNXBench. Write a placeholder instead
(`<sources>`, `<profile>`, `~`, `<host>`, "8-core CPU", "the dev machine", a
documentation address such as `203.0.113.250`).

Two kinds of rule apply:

* generic patterns that need no knowledge of any machine (local mount points,
  also in the dash-joined form tools use as directory keys, home directories
  other than obvious placeholders, agent profile paths, Arch kernel releases);
* the identifiers of the machine running the check, read at runtime so this
  file never has to contain them: host name, kernel release, CPU model,
  distribution ID unless it is a common one, `$HOME` and the directory the
  checkout lives in (both also dash-joined).

Further tokens that no machine can report (an old mount point, the address of
a real KNX gateway) come from `KNXBENCH_LOCAL_INFO_DENY` (colon-separated) and
from the clone-local, untracked file `.git/info/local-info-deny` (one token per
line, `#` comments), so they are checked without being committed anywhere.

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
    ("local mount path", re.compile(r"(?<![\w-])-(?:mnt|media)-[A-Za-z0-9][A-Za-z0-9_]*-[A-Za-z0-9]")),
    ("kernel release", re.compile(r"(?<![\w.])\d+\.\d+\.\d+-arch\d+-\d+(?![\w.])")),
)
# Distribution IDs that describe a large public user base, not one machine.
COMMON_DISTRIBUTIONS = {"arch", "debian", "ubuntu", "fedora", "alpine", "centos", "rhel", "rocky",
                        "almalinux", "opensuse", "opensuse-leap", "opensuse-tumbleweed", "sles",
                        "nixos", "linuxmint", "gentoo", "void", "amzn", "ol", "raspbian"}
CPU_VENDORS = {"AMD", "Intel(R)", "Intel"}
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


def cpu_tokens(cpuinfo: str) -> list[str]:
    """The CPU model as written in `/proc/cpuinfo`, without vendor and the bare model number."""
    match = re.search(r"^model name\s*:\s*(.+)$", cpuinfo, re.M)
    if not match:
        return []
    model = re.sub(r"\s+@.*$", "", match.group(1)).strip()
    model = re.sub(r"\s+(?:\d+-Core\s+)?(?:Processor|CPU)$", "", model).strip()
    words = model.split()
    tokens = [model]
    if len(words) > 1 and words[0] in CPU_VENDORS:
        tokens.append(" ".join(words[1:]))
    # A model number such as `1234X` or `i9-0000`; a bare number is too common a word.
    if words and re.search(r"\d", words[-1]) and re.search(r"[A-Za-z]", words[-1]):
        tokens.append(words[-1])
    return tokens


def distribution_tokens(os_release: str) -> list[str]:
    values = dict(re.findall(r'^(ID|NAME)="?([^"\n]*)"?$', os_release, re.M))
    if values.get("ID", "").lower() in COMMON_DISTRIBUTIONS:
        return []
    return [value for value in (values.get("ID", ""), values.get("NAME", "")) if value]


def deny_file_tokens(root: Path) -> list[str]:
    try:
        common = subprocess.run(["git", "rev-parse", "--git-common-dir"], cwd=root, check=True,
                                capture_output=True, text=True).stdout.strip()
        text = (root / common / "info" / "local-info-deny").read_text()
    except (OSError, subprocess.CalledProcessError):
        return []
    return [line.strip() for line in text.splitlines() if line.strip() and not line.lstrip().startswith("#")]


def read_optional(path: str) -> str:
    try:
        return Path(path).read_text()
    except OSError:
        return ""


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
            tokens.append((kind, directory.replace("/", "-")))
    tokens += [("CPU model", token) for token in cpu_tokens(read_optional("/proc/cpuinfo"))]
    tokens += [("distribution", token) for token in distribution_tokens(read_optional("/etc/os-release"))]
    extras = os.environ.get("KNXBENCH_LOCAL_INFO_DENY", "").split(":") + deny_file_tokens(root)
    tokens += [("denied token", extra.strip()) for extra in extras if len(extra.strip()) >= MIN_TOKEN]
    seen, rules = set(), []
    for kind, token in tokens:
        if token not in seen:
            seen.add(token)
            flags = re.I if kind == "distribution" else 0
            rules.append((kind, re.compile(r"(?<![\w-])" + re.escape(token) + r"(?![\w])", flags)))
    return rules


def scan_text(where: str, text: str, rules) -> list[Finding]:
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        if ALLOW_MARKER in line:
            continue
        kinds = set()  # one finding per line and kind, however many tokens overlap
        for kind, pattern in rules:
            match = pattern.search(line)
            if match and kind not in kinds:
                kinds.add(kind)
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
