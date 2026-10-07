# 2026-10-06 — Claude — AR18 re-check, round 3

Fresh reviewer session, started from
`docs/review/AR18_RECHECK_BRIEF.md#round-3-n7n10`. Read-only on product code,
offline, no hardware, no subagents, three leases for the gate run.

## What was done

- **Candidate:** `8a79791b`. The product code equals `754a66dd` (the diff
  since then is docs-only, verified). AppImage copy SHA-256 `37015eb6…`
  (matches), build stamp `754a66dd`.
- **Full gate on a fresh target directory**, offline: Rust 3367/0/178 in
  192 blocks, Vitest 2076/117, Chromium 142, fmt, clippy, xtask ×5, deny,
  npm audit 0, check-appimage, corpus subset with `OriginalData/` and
  `project_dump.json` linked.
- **Round-2 archives rerun** (round 2's ZIP writer and case scripts, from the
  evidence directory): every N7, N9-name, alias and directory case is
  refused; round-1 N1 cases stay refused; bombs peak at 10–31 MB, and the
  alias bombs that reached 1.6 GB are now refused.
- **New archives:** the ZIP writer was extended so the local header, the
  central record and the data descriptor each carry their own flags,
  method, CRC and sizes. About 25 new cases: data-descriptor variants of N9,
  size/CRC/method/flag/Unicode Path disagreements, directories with central
  size 0 but local bytes, deflated empty directories, orphaned local
  records. Run through the CLI, `products ingest`, the release server and
  the AppImage's own server.
- **Corpus:** the CLI census (3 `.knxproj` 2×0 + 1×2, 3/3 through
  `products ingest`, 103/103 `.knxprod`, no file changed) and a read-only
  structural census of the ZIP records (aggregates only).

## Result

`READY_WITH_CONDITIONS`. **N11 (IMPORTANT):** the container compares only the
local header's *name* with the central record. The `zip` reader takes all
sizes from the central record, so a record whose central sizes are 0 hides
the bytes in its local record. With a Unicode Path to a directory (or even
to another file name) the real `0.xml` disappears and a forged member is
read as `0.xml` — exit 0, `200` in the AppImage. The product reader refuses
it ("local and central CRC or sizes differ"). The corpus has 0 local/central
disagreements in 1,517 records, so the stricter rule refuses nothing real.

MINOR: N12 (a deflated empty directory, as Java tools write, is refused as
"carries data"; the product reader accepts it), N13 (`ALPHA_CANDIDATE.md:119`
still says 500).

## Pitfalls for the next reviewer

- `zip`'s `is_dir()` treats a trailing `\` as a directory too
  (`zip-8.6.0/src/spec.rs:1000`); a check that trims only `/` misses it.
- When checking "does the stricter rule break real files", run a structural
  census of the corpus (flags, method, CRC, sizes, descriptors, extras per
  record) instead of only importing: it answers the question for the owner
  before they change code. Real ETS nested payloads do carry Info-ZIP
  Unicode Path fields (both headers, consistent).
- The project's JSON has no `areas` key at the top; make the F1 "unsaved
  edit" with `PATCH /api/installations/0 {"name": …}` and confirm
  `is_modified` before testing refusals over it.
