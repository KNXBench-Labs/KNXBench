# Archived restart-snapshot tracks, 2026-11


<!-- archived 2026-11-02 from .ai/CURRENT_STATE.md -->
## Manual screenshots after Alpha 19 (PUBLISHED; observed 2026-11-01 20:30 CET)
- On main `a66697ba2`: capture specs + 59 stills/3 GIFs from the published alpha.19 image, Report problem guide,
  media manifest. Change-scoped package gate 11/11 green, inputs frozen.

## Project history MiB display fix (DELIVERED; observed 2026-11-01 20:55 CET)
- On main with this commit: history limits round to one decimal in the UI language (953.7 / 953,7 MiB). Dialog test
  RED on old code, GREEN on fix; contract now states ADR-0142 limits; limitation page adds a current-limits section
  outside its hash-pinned record. Change-scoped gate 11/11 green (821 browser cases). Ships with the next alpha.
- Worktrees, branches, scratch and the local alpha.19 image extract of both packages retired.

## Alpha 19 release (PUBLISHED; owner tag claude-release-alpha19; observed 2026-11-01 18:45 CET)
- Release freeze: released by claude-release-alpha19. `main` is open again; later work ships in the next alpha.
- Immutable `v0.1.0-alpha.19` names `0b827abee222c459ce068cbce83a941691a16805` (cutoff `858b0a24d`). Never move the tag.
- Published pre-release: AppImage, MCP, SHA256SUMS; Docker amd64+arm64, version/latest same index. Tag publishers
  41330363960/41330363961 succeeded. Candidate 1 `b73c467a7` rejected (29/30), candidate 2 gated 30/30.
- Public receipt `docs/evidence/release-alpha19-2026-11-01.json`; notes `docs/releases/alpha19.md`.
- Pinned matrix NOT run (owner-approved 28-file cross-section; legacy opt-in + A18-vs-A19 ingest differential instead).

## DIAG-03 source closeout (DELIVERED; observed 2026-11-01 CET)
- Main-first source `7443e2ea6` includes B1 admitted reduced log/monitor/browser sections, quit-over-wizard and canonical absent-source review; security ZIP9/TLS main retained. Final maintained 15/15 PASS/frozen/planCompleted, checked user-data touched/unchecked empty; software scope in `docs/status/2026-11-01-diag03-closeout.md`.
- Actual built-server/production-Web no-fixture EN1440/DE400 text/consent/native-ZIP equality, exact inventory/manifest hashes, source unchanged/SIGTERM0. Original REDs and rejected/cache-observer attempts retained privately; no hardware/native WebKit/Tauri save-dialog or release claim.
- Runtime/source complete; no further feature work reserved. Result-documentation-only successor and exact-owned archival/cleanup complete this package; shared-root coordination owns volatile local resource state. Prior private sealed scopes/foreign tracks remain held by their owners.
Alpha 19 prerequisite delivered: DIAG-03

## Dependency security fix (DELIVERED; observed 2026-11-01 CET)
- Dependabot alerts 14-22 + RustSec 2026-0382/0386: zip 8.6 -> 9.2.1, rustls 0.23.46, rustls-webpki 0.103.16, tokio 1.54.1.
  Feature `df503e2a4`, main-first merge `be12e246e` (tree equal to the gated head). `cargo deny check` advisories ok.
- Gate change-scoped 16/16 PASS, inputsFrozen: Rust 56 binaries 7947/0/241 ignored, corpus 155/155, tool-tests 472 OK.
- zip 9 drift for the release notes: unflagged UTF-8 names now decode as UTF-8 (corpus: 0 affected; KL-159),
  fallible `name()`, writer/reader fixture changes (DEVELOPER_PITFALLS "ZIP dependency"). Branch/worktree retired.
