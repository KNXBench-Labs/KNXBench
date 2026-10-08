#!/usr/bin/env python3
"""Deterministic, offline marketing site: private preview, or a release build behind the story approval."""
from __future__ import annotations

import argparse
import hashlib
from html import escape
import json
from pathlib import Path
import re
import shutil
import string
import subprocess
import sys
import tempfile

WEBSITE = Path(__file__).resolve().parent
REPO = WEBSITE.parent
SCHEMA = "knxbench-website-build/1"
MODES = ("private-preview", "release")
DOMAIN = "knxbench.com"
CONTRIBUTION_GUIDE = "https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/README.md"
GITHUB_PRIVACY = "https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement"


class BuildError(ValueError):
    """A refused build that must not damage existing output."""


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path):
    if path.is_symlink():
        raise BuildError(f"source symlink refused: {path.name}")
    def unique_fields(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise BuildError(f"duplicate JSON field: {key}")
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_fields)


def check_output(target: Path) -> None:
    if target.is_symlink():
        raise BuildError("output symlink refused")
    resolved = target.resolve()
    if WEBSITE.is_relative_to(resolved):
        raise BuildError("output must not replace the source or a parent directory")
    if resolved.is_relative_to(REPO) and not resolved.is_relative_to(WEBSITE / "dist"):
        raise BuildError("within the repository, output must stay under website/dist")
    if not target.exists():
        return
    if not target.is_dir():
        raise BuildError("output is not a directory")
    paths = list(target.rglob("*"))
    if any(path.is_symlink() for path in paths):
        raise BuildError("symlink inside existing output refused")
    files = {str(path.relative_to(target)) for path in paths if path.is_file()}
    if not files:
        return
    manifest = target / "build-manifest.json"
    if not manifest.is_file():
        raise BuildError("output contains foreign files; nothing was deleted")
    record = read_json(manifest)
    if record.get("schema") != SCHEMA or record.get("mode") not in MODES:
        raise BuildError("output is not owned by this builder")
    if files != set(record.get("files", {})) | {"build-manifest.json"}:
        raise BuildError("output contains unlisted files; nothing was deleted")
    for name in record["files"]:
        path = Path(name)
        if path.is_absolute() or ".." in path.parts:
            raise BuildError("unsafe path in existing output manifest")
        if digest(target / path) != record["files"][name]:
            raise BuildError("output contains edited files; nothing was deleted")
    expected_dirs = {str(parent) for name in files for parent in Path(name).parents if str(parent) != "."}
    actual_dirs = {str(path.relative_to(target)) for path in paths if path.is_dir()}
    if actual_dirs != expected_dirs:
        raise BuildError("output contains unlisted directories; nothing was deleted")


def info_page(lang: str, kind: str, text: dict[str, str], release: bool = False) -> str:
    e = lambda value: escape(value, quote=True)
    title = text["privacy_title_release"] if release and kind == "privacy" else text[kind + "_title"]
    contact_link = (f'<p><a href="mailto:{e(text["contact_email"])}">{e(text["contact_email"])}</a></p>'
                    if kind == "contact" else "")
    provider = (f'<p><strong>{e(text["contact_provider_label"])}</strong></p>'
                f'<address class="provider-address">{e(text["contact_name"])}<br>'
                f'{e(text["contact_street"])}<br>{e(text["contact_city"])}</address>'
                if kind == "contact" else "")
    if kind != "privacy":
        pending = ""
    elif release:
        pending = (f'<p>{e(text["privacy_hosting"])}</p>'
                   f'<p><a href="{GITHUB_PRIVACY}" rel="noopener">{e(text["privacy_policy_label"])}</a></p>')
    else:
        pending = f'<p class="launch-note">{e(text["privacy_pending"])}</p>'
    robots = "" if release else '<meta name="robots" content="noindex, nofollow">'
    banner = "" if release else f'<div class="preview-note">{e(text["preview_note"])}</div>'
    return f'''<!doctype html>
<html lang="{lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
{robots}<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'self'; font-src 'self'; base-uri 'none'; form-action 'none'">
<link rel="stylesheet" href="../../assets/site.css"><title>{e(title)} — KNXBench</title></head>
<body>{banner}<main class="wrap info-page">
<a class="text-link" href="../">{e(text['back_home'])}</a><h1>{e(title)}</h1>
{provider}{contact_link}<p>{e(text[kind + '_body'])}</p>{pending}
<a href="{CONTRIBUTION_GUIDE}" rel="noopener">{e(text['contribute_cta'])}</a>
</main></body></html>\n'''


def build(target: Path, release: bool = False, story_approval: Path | None = None) -> dict:
    check_output(target)
    media = read_json(WEBSITE / "media.json")
    if media.get("schema") != "knxbench-website-media/1" or not media.get("assets"):
        raise BuildError("missing explicit media inventory")
    source_hashes = {}
    sources = [WEBSITE / name for name in ("build.py", "page.html", "site.css", "site.js", "headlines.js", "story.json", "media.json")]
    sources += sorted((REPO / "story" / "storytool").glob("*.py"))
    sources += [REPO / "story" / "site" / name for name in ("style.css", "app.js")]
    for path in sources:
        if path.is_symlink():
            raise BuildError("source symlink refused")
        source_hashes[str(path.relative_to(REPO))] = digest(path)
    for name, record in media["assets"].items():
        if not re.fullmatch(r"[A-Za-z0-9_-]+\.(?:webp|woff2|mp4|vtt|txt)", name):
            raise BuildError("unsafe or unsupported asset name")
        source = WEBSITE / "assets" / name
        if source.is_symlink() or not source.is_file():
            raise BuildError(f"asset missing or symlink: {name}")
        if digest(source) != record.get("sha256"):
            raise BuildError(f"asset differs from reviewed media inventory: {name}")
        source_hashes[str(source.relative_to(REPO))] = record["sha256"]
    story = read_json(WEBSITE / "story.json")
    edition = story.get("edition", "")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}\.\d+", edition):
        raise BuildError("invalid pinned story edition")
    manifest = read_json(REPO / "story" / "candidates" / edition / "manifest.json")
    if manifest["story_sha256"] != story.get("storySha256"):
        raise BuildError("pinned story digest changed; review it explicitly")
    if story.get("publicationApproved") is not False:
        raise BuildError("website/story.json never claims approval; the story track's approval record decides")
    approval = story_approval or (REPO / "story" / "approvals" / f"{edition}.json")
    content = {lang: read_json(WEBSITE / "content" / f"{lang}.json") for lang in ("de", "en")}
    if set(content["de"]) != set(content["en"]):
        raise BuildError("translation keys differ")
    for lang, text in content.items():
        if not all(isinstance(v, str) and v for v in text.values()):
            raise BuildError("translations must be non-empty text")
        source_hashes[f"website/content/{lang}.json"] = digest(WEBSITE / "content" / f"{lang}.json")
    template = string.Template((WEBSITE / "page.html").read_text(encoding="utf-8"))
    target.parent.mkdir(parents=True, exist_ok=True)
    # Same-filesystem transactional staging, removed automatically on any failure.
    with tempfile.TemporaryDirectory(prefix=".website-build-", dir=target.parent) as temporary:
        stage = Path(temporary) / "site"
        (stage / "assets").mkdir(parents=True)
        for name in media["assets"]:
            shutil.copyfile(WEBSITE / "assets" / name, stage / "assets" / name)
        for name in ("site.css", "site.js", "headlines.js"):
            shutil.copyfile(WEBSITE / name, stage / "assets" / name)
        for lang, base, path in (("de", "../", "de/index.html"), ("en", "../", "en/index.html"), ("en", "", "index.html")):
            values = {key: escape(value, quote=True) for key, value in content[lang].items()}
            values.update(lang=lang, base=base, de_current='aria-current="page"' if lang == "de" else "",
                          en_current='aria-current="page"' if lang == "en" else "",
                          robots_meta="" if release else '  <meta name="robots" content="noindex, nofollow">\n',
                          preview_banner="" if release else f'  <div class="preview-note">{values["preview_note"]}</div>\n',
                          launch_block="" if release else f'      <p class="launch-note">{values["launch_note"]}</p>\n')
            page = stage / path
            page.parent.mkdir(parents=True, exist_ok=True)
            page.write_text(template.substitute(values), encoding="utf-8")
        for lang in ("de", "en"):
            for kind in ("privacy", "contact"):
                path = stage / lang / kind / "index.html"
                path.parent.mkdir(parents=True)
                path.write_text(info_page(lang, kind, content[lang], release), encoding="utf-8")
        (stage / ".nojekyll").write_text("", encoding="utf-8")
        (stage / "robots.txt").write_text("User-agent: *\nAllow: /\n" if release else "User-agent: *\nDisallow: /\n",
                                          encoding="utf-8")
        if release:
            (stage / "CNAME").write_text(DOMAIN + "\n", encoding="utf-8")
        (stage / "404.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Not found — KNXBench</title><h1>Page not found</h1><p><a href="/de/">Deutsch</a> · <a href="/en/">English</a></p></html>\n', encoding="utf-8")
        story_command = [sys.executable, "-m", "storytool", "build", edition, "--out", str(stage / "story")]
        if release:
            story_command += ["--approval", str(approval.resolve())]
        story_result = subprocess.run(story_command, cwd=REPO / "story", capture_output=True, text=True)
        if story_result.returncode:
            raise BuildError("existing story builder refused: " + story_result.stderr.strip())
        if not (stage / "story" / "index.html").is_file():
            raise BuildError("story builder produced no page")
        files = {str(path.relative_to(stage)): digest(path) for path in sorted(stage.rglob("*")) if path.is_file()}
        record = {"schema": SCHEMA, "mode": "release" if release else "private-preview", "storyEdition": edition,
                  "storySha256": story["storySha256"], "sourceHashes": source_hashes, "files": files}
        (stage / "build-manifest.json").write_text(json.dumps(record, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        # Recheck immediately before replacing only this builder's owned directory.
        check_output(target)
        if target.exists():
            backup = Path(temporary) / "previous"
            target.rename(backup)
            try:
                stage.rename(target)
            except OSError:
                backup.rename(target)
                raise
        else:
            stage.rename(target)
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=WEBSITE / "dist")
    parser.add_argument("--release", action="store_true",
                        help="public build for GitHub Pages; refused unless the story approval record matches")
    parser.add_argument("--story-approval", type=Path,
                        help="approval record to check (default: story/approvals/<edition>.json)")
    args = parser.parse_args()
    if args.story_approval and not args.release:
        print("refused: --story-approval only applies to --release", file=sys.stderr)
        return 2
    try:
        record = build(args.output.absolute(), args.release, args.story_approval)
    except (BuildError, OSError, ValueError, KeyError) as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    if args.release:
        print(f"release built: {len(record['files'])} inventoried files; story {record['storyEdition']} (approved); deploy is the workflow's job")
    else:
        print(f"private preview built: {len(record['files'])} inventoried files; story {record['storyEdition']}; no publication")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
