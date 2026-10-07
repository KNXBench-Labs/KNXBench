# KL-142 — partial-download scope in the Download to device tab (goal-ui owner, 2026-10-05)

Worktree `ui-kl142` on `origin/main` `435b25ed` (web lock taken there for
KL-142 and the ADR-0080 adoption; still held after this commit).

## Contract (COMMISSIONING_ALPHA_LEDGER "Handoff to the UI owner")

`POST /api/device-download/plan` `{ address, partial?: { parameters, groupAddresses } }`
(absent = complete); response `partial`, `notWritten: [address, octets][]`;
`start` re-derives the same partial plan from `planId`. Semantics read from
`partial_memory_download.rs` (CP §3.9.2.4 rules 1–3, KNXBench's application /
load-state check before writing) — the UI texts say only that.

## Change

- `api.ts`: `DeviceDownloadParts`; `planDeviceDownload(address, partial?)` sends
  `partial` only when given; `DeviceDownloadPlan.partial` / `notWritten`.
- `DeviceDownloadPanel.tsx`: *What to write* radio group (complete default,
  parameters, group addresses, both) before the plan; changing it drops a shown
  plan; radios disabled while planning/downloading (on each input — a disabled
  fieldset is not reflected in `input.disabled`). A partial plan shows its scope,
  the pre-write check, and every `notWritten` write as `XXXXh` + octets, or that
  none is skipped. A refusal (422) is the existing alert with no plan. Start,
  consent dialog and phrase unchanged.
- en/de messages; manual 07 step 2; KNOWN_LIMITATIONS §142 (Web UI delivered;
  comparison view still compares the complete plan only); ledger `KL-142` DONE.

## Evidence

- RED first: 10 Vitest cases (panel + API body) failed on the old code; the
  "complete sends no partial" case was already green (current behaviour).
- Mutants 8/8 (complete sends partial, parts swapped, plan survives scope
  change, notWritten hidden, notWritten always "none", scope not locked, API
  drops partial, API sends null partial).
- `e2e/device-download-scope.e2e.ts` (full app, `page.route`, no write route
  reached): RED on the previous panel/API (negative control, byte-exact restore).
- Gate (leases 7/8/9, inputs frozen): build, flow-study, theme-fixtures 0;
  Vitest 2,015 / 116; Chromium 134; new spec ×3 6; anchors 549; ledger 188;
  diff-check clean. **check-headers 1**: the new e2e header was 104 columns
  (shortened, rerun: spec 2/2, tsc, diff-check clean) — and `origin/main` itself
  reports 161 files without a header against the ceiling 157 since `9b232142`
  (ADR-0080: four new Rust files whose first `//!` paragraph spans two lines,
  counted as absent by ADR-0018). Measured on clean checkouts: 157 at
  `03609b60`/`104916d6`/`1fd1664a`, 161 at `9b232142`/`3a427055`; this package
  adds none. Reported to Alpha in the handover.

No hardware, no real bus, no KNX socket; intercepted/mocked traffic only.
