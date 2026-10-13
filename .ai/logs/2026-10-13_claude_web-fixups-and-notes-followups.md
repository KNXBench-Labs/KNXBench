# Web fix-ups: DPST-9-1 layout test and notes follow-ups (claude-web-fixups)

Sessions 2026-10-12 evening to 2026-10-13 morning. Self-review only.

## Packages

1. **DPST-9-1** (Alpha 10 prerequisite): `device-editor-layout.e2e.ts` demanded the
   raw `DPST-9-1` inline; the DPT contract's name-first amendment says `Name
   (main.sub)` with the exact reference in the tooltip. The test was wrong, the
   product unchanged. Merge `e1088312`; gate: fixture Chromium 416/0.
2. **Notes follow-ups**: `knx_app::note_targets` + `GET /api/project/notes/targets`
   (communication objects as targets, only `validate`-accepted ones, one-pass
   counting), web host consumes it (retry on failure, foreign catalogue dropped),
   browser `beforeunload` guard while note/instruction drafts exist (not in the
   desktop shell), EN/DE action names and Yes/No in the review. Merge `cd7b0efd`;
   gate: fixture Chromium 417/0, Rust 4248/0/196, corpus 143/143.

## Decisions

- Catalogue lives in `knx-app`, reading the domain model directly; `knx-app`'s
  manifest reserves `knx-projection` for the UI layer.
- The editor mounts only after the first catalogue answer, so stored targets are
  never shown as "not in catalogue" while the list is loading.
- No `beforeunload` in the Tauri shell: its close is guarded by the draft dialog
  and quits with `destroy()`; a reload inside the desktop webview stays unguarded
  (KNOWN_LIMITATIONS, OPEN_WORK).

## Found on the way

- `dpt-monitor.e2e.ts` was red on untouched main after 23:00: a late-night humour
  toast plus an nth-based `.all()` dismiss loop. Shared `dismissToasts` helper in
  `e2e/communication-fixture.ts` (front-most toast not `.toast--leaving`).
  Reproduce time-of-day failures with a shifted `TZ`.
- `cargo test` under `unshare --map-root-user` fails `knx-mcp`'s file-mode test
  (namespace root ignores modes). Both in `docs/DEVELOPER_PITFALLS.md`.

## Lock and queue

Web lock taken/released per package (`f6c23bbe`/`4a0294b8`, `d5cffb2c`/`7c280b77`).
Notes went before ux-feedback by owner decision ("Go", 2026-10-13). Open for
notes: native WebKitGTK/Orca checks, desktop-webview reload guard, FUTURE-03.
