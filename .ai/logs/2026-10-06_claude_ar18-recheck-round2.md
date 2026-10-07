# 2026-10-06 — Claude — AR18 re-check, round 2

Fresh reviewer session, started from
`docs/review/AR18_RECHECK_BRIEF.md#round-2-n1n6`. Read-only on product code,
offline, no hardware, no subagents, three leases for every Cargo run.

## What was done

- **Candidate:** `863f72ad`. The product code equals `3ede4817` (the diff
  since then is docs-only, verified). AppImage copy SHA-256 `ca3101fb…`
  (matches).
- **Full gate on a fresh target directory**, offline: Rust 3359/0/178 in
  192 blocks, Vitest 2076/117, Chromium 142, fmt, clippy, xtask ×5, deny,
  npm audit 0, check-appimage. The corpus run gave 137/143 when only
  `OriginalData/` was linked (6 oracle tests need `project_dump.json`), and
  143/143 with both linked.
- **N4:** the whole workspace test suite with an empty fake `HOME` and no
  `XDG_DATA_HOME`: 3359 passed, and the fake home is still empty.
- **Adversarial archives:** about 60, from the fictional sample and a
  hand-written ZIP writer: Unicode Path, CP437/UTF-8, `0.xml`
  substitution, local/central name mismatch, aliased offsets, zip64,
  directory entries, EOCD counts, prefixes, ZipCrypto nested payloads.
  Each was run through the release CLI, the release server and the
  AppImage's own server.
- **N2 peaks:** the 12 MB nested bomb now peaks at 31 MB in the CLI (was
  8.2 GB), 37 MB in the server, and 261 MB in the AppImage (244 MB at
  start).
- **Corpus census (aggregates only):** 3 `.knxproj` (2 exit 0, 1 exit 2
  for report errors, none refused), the same 3 through `products ingest`
  (3/3), and 103 `.knxprod` (103/103). No corpus file changed.

## Result

`READY_WITH_CONDITIONS`. **N7 (IMPORTANT):** a record whose decoded name
ends in `/` is skipped by both identity checks and by the inventory, so a
Unicode Path field can make the real `0.xml` a "directory" and another
member `0.xml`. The forged project is imported with exit 0, and the real
bytes are dropped without a report line. The product reader
(`knx-productdb/src/package.rs`) already refuses the same trick.

MINOR findings: N8 (import of a missing path answers 500), N9 (local and
central headers not compared), N10 (stale statements in
`ALPHA_CANDIDATE.md:53` and `domain.rs:271`).

## Pitfalls for the next reviewer

- Link `project_dump.json` as well as `OriginalData/` before running the
  corpus tests in a worktree.
- To measure peak RSS with `wait4`, spawn the CLI from a process that does
  not itself hold the archive. A fork inherits the parent's RSS into
  `ru_maxrss`.
- Run the AppImage's Wayland recipe with `AppRun.wrapped` after sourcing
  `apprun-hooks/linuxdeploy-plugin-gtk.sh`. Plain `AppRun` forces X11.
