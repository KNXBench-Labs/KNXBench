"""Read-only Git evidence checks against an edition's pinned source tree."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

from .schema import ValidationError


def validate_references(content: dict, repository: Path) -> None:
    """Require cited commits in the cutoff ancestry and files in that exact tree.

    This deliberately does not rewrite stale hashes. Resolve historical commit
    maps editorially, then validate the proposed new edition. No network, checkout
    changes or inspection of author identities is needed.
    """
    def git(*args: str) -> str | None:
        try:
            result = subprocess.run(
                ["git", "-C", str(repository), *args], capture_output=True,
                text=True, timeout=30, check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            return None
        return result.stdout.strip() if result.returncode == 0 else None

    cutoff = content["edition"]["cutoff"]["git_commit"]
    if not re.fullmatch(r"[0-9a-f]{40}", cutoff):
        raise ValidationError(["repository: cutoff must be a full 40-character commit hash"])
    if git("rev-parse", "--verify", f"{cutoff}^{{commit}}") != cutoff:
        raise ValidationError(["repository: cutoff commit is unavailable"])

    problems = []
    actual_time = git("show", "-s", "--format=%cI", cutoff)
    if actual_time != content["edition"]["cutoff"]["git_commit_time"]:
        problems.append("repository: cutoff commit time does not match Git including timezone")
    checked_commits: dict[str, str | None] = {}
    checked_files: dict[str, str | None] = {}
    for event in content["events"]:
        for evidence in event["evidence"]:
            ref = evidence.get("ref")
            where = f"repository: {event['id']} evidence {ref!r}"
            if evidence["kind"] == "commit":
                if ref not in checked_commits:
                    resolved = git("rev-parse", "--verify", f"{ref}^{{commit}}")
                    checked_commits[ref] = (
                        "missing or ambiguous commit" if resolved is None else
                        "not an ancestor of cutoff" if git("merge-base", "--is-ancestor", resolved, cutoff) is None
                        else None
                    )
                if checked_commits[ref]:
                    problems.append(f"{where}: {checked_commits[ref]}")
            elif evidence["kind"] in {"doc", "adr"}:
                if ref not in checked_files:
                    checked_files[ref] = git("cat-file", "-t", f"{cutoff}:{ref}")
                if checked_files[ref] != "blob":
                    problems.append(f"{where}: not a file at cutoff")
    if problems:
        raise ValidationError(problems)
