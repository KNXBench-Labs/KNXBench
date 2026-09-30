# U12 — Device checks: offline readiness and read-only comparison

Timestamp: 2026-09-30 20:15 CEST

## Source contract and boundaries

- `GET /api/device-readiness` is the mounted offline route in `device_readiness_routes.rs`; `goal-ui.md` formerly said `/api/readiness`, which is not a route. It returns every project device, including `excluded` and `no-address`, the grades' category/detail and nullable plan sizes, and counts by grade. It needs an open project and product database; it does not open a KNX tunnel.
- `POST /api/device-compare` in `device_compare_routes.rs` takes `{address, gateway, partial?}`, prepares the project's actual plan and reads the target's planned memory ranges/load states through the one gateway tunnel. It accepts neither access key nor write confirmation. The server's `written: false` claim, mask/manufacturer, states, counts and every differing byte range are shown only after comparing the response to the requested address and complete scope and checking its count consistency. No optional partial-scope UI is offered here.
- The UI presents both under a new Bus diagnostics **Device checks** tab. The table keeps unknown grades and original refusal/evidence visible, including zero-size plans; duplicate addresses are not guessed into a compare target. Selecting an eligible device/gateway only opens a confirmation step, never a tunnel. The second explicit action calls compare. Input/project changes invalidate stale results. The UI warns that an already-started read is not cancelled by navigation and that displayed memory may be private. No live KNX connection, monitor or device write was made for this UI feature.
- ADR-0051 Debug write UI was safety-deferred when this package began. While the Web lock was held, commissioning published `44b42cee` and handovers `6a45fa02`/`afec251d`: a property-specific, durable pre-write receipt is now present. The UI owner must review that new contract in a fresh package after releasing this read-only lock; it is not a full device backup or a hardware go.

## RED/GREEN and regression evidence

- `BusDiagnosticsPanel.test.tsx` first failed because the Device checks tab did not exist, then passed after integration. Additional RED tests caught a missing keyboard-scrollable region and an unreported empty load-state array; both now pass.
- Safety mutations: moving `compareDevice` into the review step made the no-request-before-confirm assertion fail; disabling the `written !== false` guard made the explicit write-warning assertion fail. Both mutations were restored.
- Focused UI tests: 12/12. Full Web suite before rebase: 80 files / 1,270 passed, TypeScript no diagnostics, Vite build green. Local mocked Chromium: 4/4 EN/DE at 360/1440 px for this panel, 4/4 prior monitor cases, 10/10 existing device-editor cases. Browser fixture rejects unknown `/api/**` requests; no production gateway.
- On the final rebased branch including K12 and ADR-0055 activity changes, workspace Rust passed 137 suites / 2,769 passed / 0 failed / 160 ignored / 0 `SKIP:`. Strict Clippy, fmt, layering, headers, anchors, corpus gates and `git diff --check` passed. Web passed 80 files / 1,270 tests; TypeScript and Vite build clean; local mocked Chromium 4/4 Device checks, 4/4 monitor control and 10/10 existing.
- Corpus-backed Rust gate before upstream K12/ADR-0055: 136 suites / 2,761 passed / 0 failed / 160 ignored / 0 `SKIP:`. Focused server contracts: `http_device_readiness.rs` 3/3; ignored-by-default `http_device_compare.rs` 6/6 against the private corpus + simulator (writes asserted absent). No raw private corpus bytes entered this log.

## Delivery to finish

- Commit as `KNXBench <github@knxbench.com>` with no co-author, rebase onto current `origin/main` without dropping upstream commissioning entries, rerun merged-equivalent gates including the new K12 Rust tests, fast-forward push, read back remote SHA, release the Web lock, remove task-only worktree and scratch.
- Root `main` is intentionally left alone: it lags upstream and contains foreign edits in `.ai/CURRENT_STATE.md`, `docs/RESEARCH.md`, `stats.md` and untracked documents. No root merge/reset, no unrelated cleanup.
