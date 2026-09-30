# U12 — ADR-0051 Debug setting and service-control UI

Timestamp: 2026-09-30 21:55 CEST

## Safety preflight (repository evidence, no hardware)

- ADR-0051 is Accepted. `apps/knx-server/src/service_control_routes.rs` gates both GET and POST by the saved `debugIndividualAddressWriteEnable: true` setting before connecting, validates the target and the dedicated scope phrase, and serializes the route with monitor, scan, download and address-programming sessions. `apps/knx-server/tests/http_service_control.rs` exercises default-off 403, wrong phrase before a tunnel, successful simulator bit change and fail-closed backup error.
- `crates/knx-net/src/commissioning/service_control.rs` reads mask and both octets inside one management session, calls the persistence callback before its first property write, modifies only bit 2 and requires exact readback. `crates/knx-app/src/service_control_backup.rs` creates a new owner-only record, syncs and reads it back; this is the **complete property value this procedure overwrites**, not a whole-device image, automatic rollback or proof of no vendor side effects.
- The previous UI safety pause applied to the route before the K12 backup existed. This review does not extend recovery claims to other routes, K13/reset, partial writes, or live hardware. The server setting still defaults off. No credentials, keys, device dumps or private corpus bytes were logged here.

## UI and regression evidence

- `settingsStore.ts` adds serialized, server-confirmed boolean reads/writes for the safety preference; no cached `true` is proof of server opt-in. A PUT result is followed by an actual GET readback, and a contradiction or network failure leaves the switch unavailable until re-check. A failed initial settings hydration can be explicitly retried without racing another caller. `ServiceControlDebugSetting.tsx` is a warning-labelled section of the existing Settings panel; a delayed StrictMode read cannot override a newer server answer.
- `ServiceControlPanel.tsx` is a separate Bus diagnostics tab. It does not connect on mount, input or review. A read requires an explicit click; after reviewing the address, raw property and mask, a second action requires typing `I confirm individual-address write enable to <address>` verbatim. Client checks the result's target, original value, typed hex words, mask, bit-only change, readback state and backup path; the server is still the authoritative write gate. Stale in-flight operations hold the tunnel controls locked across project revisions; leaving the view does not cancel an operation.
- RED→GREEN: `settingsStore.test.ts` first failed on missing confirmed APIs (4 failing), the new setting/action tests first failed on missing components, and the diagnostics route first failed on the missing tab. The safety guard mutations for a missing backup path and an untyped phrase each failed their focused test before restoration.
- Full Web suite: 82 files / 1,295 passed; TypeScript no diagnostics and build green. Local intercepted Chromium EN/DE at 360/1440 px: 4/4 new, 4/4 Device checks, 4/4 monitor, 10/10 existing. The fixture rejects unmocked `/api/**` requests and uses no production KNX gateway.
- Workspace Rust after Web build: 137 suites / 2,769 passed / 0 failed / 160 ignored / 0 `SKIP:`; strict Clippy 151 checked crates / 0 errors; fmt, layering, headers (352 valid / 161 baseline without), anchors (405 links / 221 Markdown files), corpus-gate and diff check green. An initial Rust attempt stopped before tests because the fresh worktree lacked `apps/knx-web/dist`; after building the Web app the complete Rust suite passed. All tests were offline, mocked or simulator-based.

## Pending delivery

Review the full diff and current upstream, rerun final gates after documentation, rebase without losing other sessions' status and `.ai` entries, publish with remote SHA readback, release the Web lock in a separate handover commit and remove only this package's link/worktree/scratch. Do not try the new action against real hardware as part of the UI track.
