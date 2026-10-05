"""Automatic privacy scan of public candidate text; a review aid, never a safety proof."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

# Hard findings block preparation: these classes should never appear in public text.
HARD_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("local filesystem path", re.compile(r"(?:(?<![\w.])/(?:home|mnt|root|tmp|var|etc|Users)/|~/|[A-Za-z]:\\)")),
    ("email address", re.compile(r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b")),
    ("IPv4 address", re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")),
    ("UUID or session identifier", re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", re.I)),
    ("long hexadecimal token", re.compile(r"\b[0-9a-f]{41,}\b", re.I)),
    ("credential assignment", re.compile(r"(?i)\b(?:password|passwd|passphrase|api[_-]?key|secret|token)\s*[:=]\s*\S+")),
    ("private key block", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("provider API key", re.compile(r"\b(?:sk|ghp|gho|xox[bp])[-_][A-Za-z0-9]{16,}")),
)

# Soft findings are reported for manual review but do not block preparation.
SOFT_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("possible KNX individual address or version number", re.compile(r"(?<![\w.])\d{1,2}\.\d{1,2}\.\d{1,3}(?![\w.]*\d)")),
    ("possible KNX group address", re.compile(r"(?<![\w/])\d{1,2}/\d{1,2}/\d{1,3}(?![\w/])")),
    ("web address", re.compile(r"\bhttps?://\S+")),
    ("at-handle", re.compile(r"(?<![\w.])@[A-Za-z0-9_]{2,}")),
)

# Fields that never contain prose; scanning them would only produce noise.
_SKIP_KEYS = {"id", "from", "to", "strand", "kind", "status", "type", "date", "date_precision",
              "source", "accent", "speaker", "original_language", "transforms", "schema",
              "git_commit", "git_ref", "git_commit_time", "conversations_through",
              "number", "ref"}


@dataclass(frozen=True)
class Finding:
    severity: str
    category: str
    location: str
    excerpt: str

    def as_dict(self) -> dict:
        return {"severity": self.severity, "category": self.category,
                "location": self.location, "excerpt": self.excerpt}


def _walk(value: object, location: str):
    if isinstance(value, dict):
        for key in sorted(value):
            if key in _SKIP_KEYS:
                continue
            if key == "events" and all(isinstance(item, str) for item in value[key]):
                continue  # a chapter's list of event identifiers, not prose
            yield from _walk(value[key], f"{location}.{key}" if location else key)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            # Records with an id are named by it, so review notes survive reordering.
            key = item.get("id") if isinstance(item, dict) else None
            yield from _walk(item, f"{location}[{key if isinstance(key, str) and key else index}]")
    elif isinstance(value, str):
        yield location, value


def _context(text: str, start: int, end: int) -> str:
    left, right = max(0, start - 20), min(len(text), end + 20)
    return ("…" if left else "") + text[left:right] + ("…" if right < len(text) else "")


def scan(content: dict) -> list[Finding]:
    """Scans every prose string of the content and returns hard and soft findings."""
    findings: list[Finding] = []
    for location, text in _walk(content, ""):
        for severity, patterns in (("error", HARD_PATTERNS), ("warning", SOFT_PATTERNS)):
            for category, pattern in patterns:
                for match in pattern.finditer(text):
                    findings.append(Finding(severity, category, location,
                                            _context(text, match.start(), match.end())))
    return findings


def private_canaries(provenance: dict) -> set[str]:
    """Collects distinctive private locator strings that must never reach the public payload."""
    canaries: set[str] = set()

    def collect(value: object) -> None:
        if isinstance(value, dict):
            for item in value.values():
                collect(item)
        elif isinstance(value, list):
            for item in value:
                collect(item)
        elif isinstance(value, str):
            for token in re.findall(r"[^\s,;()]+", value):
                token = token.rstrip("*.:")
                looks_private = (
                    token.startswith(("/", "~"))
                    or re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{3,}", token) is not None
                    or re.fullmatch(r"\d{8}_\d{3,}", token) is not None
                )
                if looks_private and len(token) >= 8:
                    canaries.add(token)

    collect(provenance.get("sources", {}))
    collect(provenance.get("events", {}))
    return canaries


def load_provenance(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        provenance = json.load(handle)
    if not isinstance(provenance.get("events"), dict):
        raise ValueError(f"{path}: private provenance must map event ids under 'events'")
    return provenance


def check_provenance(content: dict, provenance: dict, payload_text: str) -> dict:
    """Checks private traceability without copying any private locator into the result."""
    public_ids = {event["id"] for event in content["events"]}
    private_ids = set(provenance["events"])
    leaked = sorted(c for c in private_canaries(provenance) if c in payload_text)
    return {
        "status": "checked",
        "events_without_private_provenance": sorted(public_ids - private_ids),
        "private_entries_without_public_event": len(private_ids - public_ids),
        "private_locators_found_in_payload": len(leaked),
    }
