# 2026-10-13 — Claude: session log 16000, manufacturer-unknown grouping (#7, KL-036)

Session `bcee21b10872`, owner request "do all of that; set the log limit to about 16000".

## Delivered

- Branch `feat/session-log-16k` (base `decd0dcc`): `0d92374f` server/app, `c7708162`
  web, `a6f6cdb8` docs, `bb753341` review fix; merge `beca550a`, handover `0781c672`.
- `MAX_ENTRIES` 16000; eviction info → warning → error, oldest first within each;
  drop notice keeps the parsed leading sentence plus `(i info, w warning, e error; …)`.
- `knx_app::import_findings` (new module; the counting helpers moved here from
  `import_payload_evidence`): manufacturer unknowns grouped by (kind, name), counted
  once per group for fresh and reopened projects (`import_evidence::UnknownWitness`).
  Archive rule `knx_etsproj::opaque::is_manufacturer_entry`.
- Web: `logRows.ts` (keys/times/order once per snapshot), LogPanel pages of 500,
  3-line detail preview, `SESSION_LOG_CAPACITY` 16000, drop regex for any cap.
- Desktop: session-log export capacity 16000, 64 MiB limit for that export only.

## Evidence

- Measured with a release server: 16000-entry fill 10 ms, push with eviction 27 µs,
  `GET /api/log` 6 ms / 5.3 MB. Chromium A/B against an origin/main build, 3 runs:
  open 2415–2534 → 150–166 ms, search 159–256 → 28–39 ms, clear 1556–1567 → 42–44 ms.
  Playwright WebKit does not start on this host.
- Real imports with product DB: large private schema-23 project 3432 entries / 3415
  warnings → 328 / 311; two smaller private ETS6/ETS4 projects about 100 each.
- Gates: branch full (Rust 4313/0/198, Vitest 3310, corpus 145/145 in 32 targets,
  frozen); merge on `dcdfb0fb` full (Rust 4385/0/199, Vitest 3330 in 237 files,
  corpus 145/145, frozen); re-merge on `113d3cea` delta (Vitest 3334, build,
  Playwright 59/59, desktop clippy, xtask, docs; frozen); final merge on `a0abd553`
  (docs/tools-only delta): tools, docs, limitations, anchors/ledger/headers.
- Review finding F1 (refetch reset the page limit) fixed with a red-proven test. The
  first full gate caught two guard tests (import-graph inventory, CSS rule for the new
  button class); both fixed before the frozen run.
- Mutants red: age-only eviction, grouping off, origin/main LogPanel, old drop regex,
  refetch reset.

## Root checkout

- Was 501 commits behind with 33 dirty doc paths and a stale empty `.git/index.lock`
  (2026-10-12 13:59). Parked unpublished docs on local `wip/manufacturer-plugin-research-20261013`
  (14 never-committed files) and `wip/root-residue-20261013` (11 paths), archived with
  SHA256SUMS in `KNXBench.backups/2026-10-13-root-sync/`, fast-forwarded. Later
  fast-forward kept a codex root-local note byte for byte.

## Not done

- KL-181/KL-182 native window: WebKit paints nothing on Hyprland `HEADLESS-1`; the
  desktop driver sees X11 only. Not moved to the owner's visible monitor.
- alpha.12: not cut; five issues promise "next alpha" for Goal B work still open.
