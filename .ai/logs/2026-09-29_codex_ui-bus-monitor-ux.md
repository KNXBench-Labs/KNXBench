# U7 bus-monitor UX — branch-gated (2026-09-29)

Branch `ui-bus-monitor-ux` rebased on `origin/main` (`64f3768`); merge and push are still pending. Web lock remains held by the UI session. No live bus operation, tunnel or device write.

## What changed

- Pause/Resume is client-side: the server session and cursor survive; late replies after Pause are discarded, and slow polls never overlap. Reconnect, gateway closure, dropped server rows and disconnect while paused have fake-timer regressions. Final self-review corrected the pause text: it explicitly warns about the server's finite buffer rather than attributing retention to the gateway.
- The decode DTO adds DPT plus `unsupportedDpt` or `decodeFailed` on `error`, preserving legacy fields. The panel labels unresolved, conflict, unsupported, failed and unknown legacy errors without parsing human messages.
- A 1000-row client capture counts local eviction separately from the server's gap. Statistics rank up to 10 real retained services, destinations and sources, excluding synthetic close markers. Filters do not alter statistics or the cursor.
- `knxbench-bus-monitor` JSON v1 captures retained raw/decoded data and provenance. Browser delivery uses a local Blob; desktop uses a Tauri-owned save dialog, version/row/16 MiB validation and atomic write. The session-log export only shares the local delivery/write primitives, not its schema.
- A labelled, keyboard-focusable scroll region keeps the telegram table legible on narrow panes; filter, Pause, Resume and export remain visible. Manual, architecture, research, limitations §137, status and ISSUE-11 checklist are updated.

## Verified on the branch

- TDD RED/GREEN for pause/overlap, decode states, capture/pruning, statistics, export and responsive table. Five guard mutations were rejected by focused regressions, then reverted.
- TypeScript/build and 77 web test files / 1163 passed. Rust fmt/Clippy and 123 suites / 2384 passed before rebase, then 2390 passed, 0 failed, 136 ignored after rebase. Layering, headers (287/161, ceiling 161), 397 Markdown anchors and corpus gate passed with an `xtask` binary bound to this worktree.
- Headless Chromium against only `127.0.0.1:4791` with intercepted fixture endpoints: at 640 px document width stayed 640 px; a 928 px telegram table scrolled within a 606 px region and the Right key moved it 40 px. Fixture showed five retained telegrams, statistics, Pause/Resume and a dropped-row notice. This is not native WebKitGTK or a live bus test.

## Integration next

Amend the reviewed commit explicitly as KNXBench without co-authors. Fetch `origin/main` again before fast-forwarding `main`, preserving both commissioning and UI entries in shared docs and handover. Rerun all merged-result gates, push, release the web lock and remove only U7-owned worktree/scratch artifacts. Leave root `docs/paperclip-shutdown/` untouched; §137 was checked as the next free limitation number.
