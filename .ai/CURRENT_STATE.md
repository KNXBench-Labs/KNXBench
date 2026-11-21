# Restart snapshot — WP-37 docs closeout (Slice S1) by subagent (2026-11-20, branch work/wp37-docs-closeout-20261120)
## WP-48 corrective backlog (PARKED; owner WebUI audit; observed 2026-11-21)
- Owner requested defect search since 14 November and one work package, not implementation. `docs/open-work/WP-48-recent-change-correctness-audit.md` records 16 source-confirmed findings, six separate verification-lead groups and nine planned slices. Audit target `163e38b85`; helper probes are not full runtime/browser/device acceptance.
- Await explicit implementation Go; no bus/hardware/provider/VM/release/deploy authority. Runtime fixes remain open; source ownership must be rechecked on fresh main before each sequential subagent slice. Shared dirty root and foreign tracks stay untouched.
## PRG-01 (PARKED at evidence wall; N1–N14 merged on origin/main; observed 2026-11-21)
- Bounded non-Secure programming increments N1–N14 merged; all four open axes (run-state N4, parameter policies N5, mutation effects N6, recovery N14) end at an evidence wall; N15 (PR pending) records it in `docs/open-work/PRG-01-programming-gaps.md`. Simulator/loopback only; KNX-compatible, hardware-untested.
- Next step needs owner evidence (primary-source rules or authorised device observation) or a decision; no further increment authorised.
- Former gate follow-ups fixed on origin/main 2026-11-21 (GATE-01): C49 is now an `OriginalData` corpus test (#122); simulated TCP gateways retry TCP/UDP trap port collisions via `knx_testsupport::tcp_listener_with_udp_trap` (#124).
## WP-37 all slices delivered on origin/main as of 2026-11-20 (STANDING; observed 2026-11-20)
- Steps 1–4 shipped in Alpha 21 (per-parameter marks ADR-0173 schema 20, per-link/object-flag marks ADR-0174 schema 21, section/ancestor marks #48 closed); module-scoped parameters shipped 17 November 2026 (schema 22, ADR-0177) with the image-builder module admission via PR !60 (`b4d141288`, 18 November); group-address last-download link marks delivered 20 November 2026 via PR !104 (`aa14a7329` on origin/main, gate receipt 1795199079555335506, web-only). Remaining "Not covered" items in docs/open-work/WP-37-pending-changes-across-ui.md are documented boundaries, not open work. No bus/hardware/VM/release/deploy; simulator and intercepted-browser evidence only.
## WP-37-Rest docs closeout (RUNNING; owner WebUI; observed 2026-11-18)
- WP-37 open-work doc + LEDGER updated on `work/wp37-modules-docs-20261118`; PR open, merge next. No bus/hardware/release/deploy.
## Retained boundaries (STANDING; observed 2026-11-02 20:47 CET)
- Long-standing STANDING acceptance/reference sections live in `.ai/archive/` per `.ai/archive/README.md`; root intentionally stale/dirty, foreign tracks preserved, private bytes stay private.
