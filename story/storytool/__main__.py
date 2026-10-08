"""Command line for preparing, previewing and gating KNXBench project-evolution candidates."""

from __future__ import annotations

import argparse
import functools
import http.server
import ipaddress
import json
import sys
from pathlib import Path

from . import candidate, release, render
from .schema import ValidationError, validate

STORY_DIR = Path(__file__).resolve().parent.parent
DEFAULT_CONTENT = STORY_DIR / "content" / "edition.json"
DEFAULT_CANDIDATES = STORY_DIR / "candidates"
DEFAULT_DIST = STORY_DIR / "dist"
DEFAULT_PREVIEWS = STORY_DIR / "previews"

EXIT_OK, EXIT_INVALID, EXIT_REFUSED = 0, 1, 3


def _cmd_validate(args: argparse.Namespace) -> int:
    content = json.loads(Path(args.content).read_text(encoding="utf-8"))
    validate(content)
    print(f"valid: {len(content['events'])} events, {len(content['relations'])} relations, "
          f"{len(content['chapters'])} chapters")
    return EXIT_OK


def _cmd_prepare(args: argparse.Namespace) -> int:
    result = candidate.prepare(
        Path(args.content), Path(args.candidates),
        provenance_path=Path(args.private_provenance) if args.private_provenance else None,
        base_id=args.base,
    )
    print(f"{result.status}: candidate {result.candidate_id} at {result.directory}")
    print(f"story_sha256 {result.story_sha256}")
    print(f"privacy warnings for review: {len(result.warnings)}")
    print("not approved for publication; review REVIEW.md and CHANGES.md in the candidate directory")
    return EXIT_OK


def _cmd_build(args: argparse.Namespace) -> int:
    source = Path(args.candidates) / args.candidate
    if args.approval and args.preview:
        print("refused: --approval renders the published variant, never a committed preview", file=sys.stderr)
        return EXIT_REFUSED
    if args.preview:
        target = render.build(source, DEFAULT_PREVIEWS, f"{args.candidate}.html")
        print(f"wrote versioned preview {target} (commit it together with its candidate)")
        return EXIT_OK
    if args.approval:
        try:
            target = render.build(source, Path(args.out) if args.out else DEFAULT_DIST / args.candidate,
                                  approval=Path(args.approval))
        except render.PublicationRefused as refusal:
            print(f"refused: {refusal}", file=sys.stderr)
            return EXIT_REFUSED
        print(f"built published edition {target} (approval matched; deploying it is a separate step)")
        return EXIT_OK
    target = render.build(source, Path(args.out) if args.out else DEFAULT_DIST / args.candidate)
    print(f"built private preview {target}")
    return EXIT_OK


def _cmd_serve(args: argparse.Namespace) -> int:
    if not ipaddress.ip_address(args.host).is_loopback:
        print("refused: the preview is private and may only listen on a loopback address", file=sys.stderr)
        return EXIT_REFUSED
    directory = Path(args.dist) / args.candidate
    if not (directory / "index.html").is_file():
        print(f"no built preview in {directory}; run 'build' first", file=sys.stderr)
        return EXIT_INVALID
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(directory))
    with http.server.ThreadingHTTPServer((args.host, args.port), handler) as server:
        host, port = server.server_address[:2]
        print(f"serving private preview on http://{host}:{port}/ (Ctrl+C to stop)", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
    return EXIT_OK


def _cmd_release_check(args: argparse.Namespace) -> int:
    gate = release.check_approval(Path(args.candidates) / args.candidate,
                                  Path(args.approval) if args.approval else None)
    if gate.eligible:
        print("approval matches this exact candidate. Publication itself is still a separate, manual step.")
        return EXIT_OK
    for reason in gate.reasons:
        print(f"not eligible: {reason}", file=sys.stderr)
    return EXIT_REFUSED


def _cmd_publish(_args: argparse.Namespace) -> int:
    print(f"refused: {release.PUBLISH_REFUSAL}", file=sys.stderr)
    return EXIT_REFUSED


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="storytool", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("validate", help="check the candidate content against the schema")
    p.add_argument("--content", default=str(DEFAULT_CONTENT))
    p.set_defaults(func=_cmd_validate)

    p = sub.add_parser("prepare", help="prepare an immutable private candidate (never publishes)")
    p.add_argument("--content", default=str(DEFAULT_CONTENT))
    p.add_argument("--candidates", default=str(DEFAULT_CANDIDATES))
    p.add_argument("--private-provenance", help="private provenance file, read for traceability checks only")
    p.add_argument("--base", help="candidate id to diff against (default: the latest earlier candidate)")
    p.set_defaults(func=_cmd_prepare)

    p = sub.add_parser("build", help="render a candidate into a self-contained offline preview")
    p.add_argument("candidate")
    p.add_argument("--candidates", default=str(DEFAULT_CANDIDATES))
    output = p.add_mutually_exclusive_group()
    output.add_argument("--out", help="output directory (default: story/dist/<candidate>)")
    output.add_argument("--preview", action="store_true",
                        help="write the versioned preview story/previews/<candidate>.html")
    p.add_argument("--approval", help="render the published variant; refused unless this "
                   "approval record matches the exact candidate (not with --preview)")
    p.set_defaults(func=_cmd_build)

    p = sub.add_parser("serve", help="serve a built preview on a loopback address only")
    p.add_argument("candidate")
    p.add_argument("--dist", default=str(DEFAULT_DIST))
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=8765)
    p.set_defaults(func=_cmd_serve)

    p = sub.add_parser("release-check", help="check whether an approval record matches a candidate exactly")
    p.add_argument("candidate")
    p.add_argument("--candidates", default=str(DEFAULT_CANDIDATES))
    p.add_argument("--approval", help="approval record (JSON); missing means not approved")
    p.set_defaults(func=_cmd_release_check)

    p = sub.add_parser("publish", help="always refuses: publication is outside this tool")
    p.add_argument("candidate", nargs="?")
    p.set_defaults(func=_cmd_publish)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.func(args)
    except ValidationError as error:
        for problem in error.problems:
            print(f"invalid: {problem}", file=sys.stderr)
        return EXIT_INVALID
    except (candidate.PrepareError, release.IntegrityError) as error:
        print(f"refused: {error}", file=sys.stderr)
        return EXIT_REFUSED


if __name__ == "__main__":
    sys.exit(main())
