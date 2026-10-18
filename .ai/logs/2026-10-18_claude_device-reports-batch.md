# 2026-10-18 — Claude: eleven more real diagnostic exports

## Input

Owner note: more device data in the private collection `OriginalData/DeviceReports/`
(12 ZIPs). One (`knxbench-device-diagnostics-…`) is the issue #22 UP 258D12 export
already reviewed at 20:21 (`4acc7a20`); eleven are new, same source build
`0.1.0-alpha.14+g0341483c`, received by email. Archives stay private and uncommitted.

## Findings (counts and catalogue facts only in Git)

- All 12: `simulated:false`, `written:false`, complete, disconnect attempt completed,
  manifest hashes OK, every catalogue assertion (11 runs) agreed with the device.
- PID 78: 10 read (6 octets), 2 refused with zero elements (two Hager `0701` devices
  whose applications declare no PID 78 compare). First real `refused-ambiguous`.
- `property_matches` `[A]`: 8 of 8 devices with a declared PID 78 compare match
  (10-octet data, 4-octet zero tail); byte-exact over 10 octets would match none.
- Bounded XML scan of every held product archive: 398 apps with a PID 78 compare
  (369 ten-, 28 six-, 1 seven-octet; the seven-octet Gira one has a non-zero tail,
  a counterexample to the zero-tail assumption; fails closed).
- G10: MDT `07B0` `HWT_Check` (48-bit `BinaryValue`, `Access=None`, placed on PID 78)
  default equals the device's reply. First device-side data point, still a hypothesis.
- System B (`07B0`) apps compare other properties (MDT PID 162, GVS object 4 PID 66).

## Delivered

- `crates/knx-app/tests/device_reports_corpus.rs`: ignored private gate, strict
  reader + byte-identical re-bundle of all 12, aggregate pins; negative controls
  (missing dir, removed archive, tampered byte) red.
- Evidence note `docs/research/2026-10-18-device-diagnostics-eleven-devices.md`;
  contract, KL-184 (detail, `catalogue.json` + regenerated overview, ledger),
  OPEN_WORK (diagnostic section, G10 item, WP-12), manual 08, System-2
  prerequisites, census G10 follow-up, research index, implementation history.
- Privacy: no order numbers, sender labels, sender identity, project references,
  timestamps or digests in Git; added-line scan clean.

## Gates

fmt, Clippy `-p knx-app --all-targets -D warnings`, corpus gate 1/1 (offline
namespace), docs 0 errors, anchors 1,548, ledger 208, headers, corpus-gates lint,
layering, docs tool tests 51, runner tests, local-info (tree + messages), diff-check.
Upstream delta during the package was docs-only (backlog audit), merged main-first.

## Open

Silent device, post-run behaviour, firmware strings; G10 tool semantics; correct
comparison for the seven-octet Gira case. Optional: an issue #22 thank-you reply
(owner decision; keep the installation inventory out of the public thread).
