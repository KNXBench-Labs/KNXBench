#!/usr/bin/env python3
"""Check local documentation targets, image descriptions and the manual's chapter chain."""

import argparse
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit


@dataclass(frozen=True)
class Link:
    target: str
    image: bool = False
    alt: str = ""


def without_code(text: str) -> str:
    """Ignore examples in fenced/inline code and HTML comments, preserving prose."""
    lines = []
    fence = None
    for line in text.splitlines():
        marker = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
        if fence:
            if marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence):
                fence = None
            lines.append("")
        elif marker:
            fence = marker[1]
            lines.append("")
        else:
            lines.append(line)
    result = re.sub(r"<!--[\s\S]*?-->", "", "\n".join(lines))
    return re.sub(r"(`+)([^`]*?)\1", "", result)


class HtmlLinks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if tag == "img" and "src" in values:
            self.links.append(Link(values["src"], True, values.get("alt", "")))
        elif tag == "a" and "href" in values:
            self.links.append(Link(values["href"]))


def links_in(text: str) -> list[Link]:
    """Read the inline, reference and HTML link syntax used by repository docs."""
    text = without_code(text)
    definitions = dict(re.findall(r"(?m)^\s*\[([^\]]+)\]:\s*<?([^\s>]+)>?", text))
    definitions = {key.casefold(): value for key, value in definitions.items()}
    links = []
    for match in re.finditer(r"(!?)\[([^\]]*)\]\(", text):
        start = match.end()
        depth = 1
        end = start
        while end < len(text) and depth:
            if text[end] == "(" and (end == 0 or text[end - 1] != "\\"):
                depth += 1
            elif text[end] == ")" and (end == 0 or text[end - 1] != "\\"):
                depth -= 1
            end += 1
        if depth:
            raise ValueError("Unclosed Markdown link destination")
        value = text[start:end - 1].strip()
        if not value:
            raise ValueError("Empty Markdown link destination")
        target = value[1:value.index(">")] if value.startswith("<") else value.split()[0]
        links.append(Link(target, bool(match[1]), match[2] if match[1] else ""))
    for match in re.finditer(r"(!?)\[([^\]]*)\]\[([^\]]*)\]", text):
        key = (match[3] or match[2]).casefold()
        if key not in definitions:
            raise ValueError(f"Undefined reference link: {key}")
        links.append(Link(definitions[key], bool(match[1]), match[2] if match[1] else ""))
    parser = HtmlLinks()
    parser.feed(text)
    return links + parser.links


def local_target(source: Path, target: str) -> Path | None:
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    return (source.parent / unquote(parsed.path)).resolve()


def chapter_chain(root: Path) -> tuple[list[Path], list[str]]:
    manual = root / "docs/manual"
    index = manual / "README.md"
    entries = re.findall(r"(?m)^\d+\. \[([^\]]+)\]\(([^)]+\.md)\)$", index.read_text())
    errors = []
    chapters = [manual / target for _, target in entries]
    expected = set(manual.rglob("*.md")) - {index}
    if set(chapters) != expected or len(chapters) != len(set(chapters)):
        errors.append("Manual index must list every chapter exactly once")
    if not chapters:
        errors.append("Manual index has no numbered chapters")
    for i, chapter in enumerate(chapters):
        if not chapter.is_file():
            errors.append(f"Missing chapter: {chapter.relative_to(root)}")
            continue
        lines = chapter.read_text().strip().splitlines()
        for boundary, word, target in [
            (lines[0], "Previous:", chapters[i - 1] if i else index),
            (lines[-1], "Next:", chapters[i + 1] if i + 1 < len(chapters) else index),
        ]:
            navigation = links_in(boundary.split(word, 1)[1]) if word in boundary else []
            if not navigation or local_target(chapter, navigation[0].target) != target.resolve():
                errors.append(f"{chapter.relative_to(root)}: incorrect {word} boundary")
    return chapters, errors


def check(root: Path) -> tuple[dict, list[str]]:
    root = root.resolve()
    sources = sorted(set(root.glob("*.md")) | set((root / "docs").rglob("*.md")))
    errors = []
    count = images = 0
    for source in sources:
        try:
            links = links_in(source.read_text())
        except ValueError as error:
            errors.append(f"{source.relative_to(root)}: {error}")
            continue
        for link in links:
            if link.image:
                images += 1
                if not link.alt.strip():
                    errors.append(f"{source.relative_to(root)}: image has no description: {link.target}")
            target = local_target(source, link.target)
            if target is not None:
                count += 1
                if not target.exists():
                    errors.append(f"{source.relative_to(root)}: missing target: {link.target}")
    chapters, navigation = chapter_chain(root)
    errors.extend(navigation)
    return {"markdown_files": len(sources), "local_targets": count,
            "images": images, "chapters": len(chapters)}, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    counts, errors = check(args.root)
    print("Documentation:", ", ".join(f"{key}={value}" for key, value in counts.items()))
    for error in errors:
        print("ERROR:", error)
    print(f"{'FAIL' if errors else 'PASS'}: {len(errors)} errors; external URLs are not checked.")
    print("Run xtask check-anchors separately for heading anchors.")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
