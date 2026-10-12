# 2026-10-13 — Claude — KL-170 evidence review and read-only byte-order probe

Branch `feature/kl170-little-endian`, worktree `KNXBench.worktrees/kl170-20261012`,
from origin/main `7974a812`. User request: "Arbeite KL-170 soweit ab wie es ohne
Testhardware geht. Nutze alle Informationen die du dazu finden kannst."
No hardware, no bus traffic, no web lock (no `apps/knx-web` change).

## Research (docs/research/parameter-byte-order.md)

- KNX Standard corpus (179 PDFs, `knx-spec-kb`): no `ParameterByteOrder`
  definition. Only related clause: Management `03_08_03` v01.07.03 §2.2 p. 6,
  big-endian parameter presentation for KNXnet/IP device downloads (scoped).
- Schema copies: knx_project_20 XSD copy + XKNX generated schema-23 model:
  optional, default `BigEndian`. Official XSD / ETS SDK docs / MT help not
  obtainable; `.vd*` samples password-protected (no guessing).
- Implementations: xknx-editor (GPL-2.0) and koolenex (MIT) both reverse the
  octets of whole-octet numeric fields; no source cited, no LE device capture.
  Behaviour only, no code copied (KNXBench is AGPL-3.0).
- Public vendor corpus (knxprod-crawler, 853 archives, 3,341 unique programs):
  590 LittleEndian. Base images of 370 LE programs low-octet-first, 2 Siemens
  07B0 high-first. Control: ABB/BJ BigEndian and undeclared 0701 programs are
  ALSO mostly low-octet-first → base images do not show what ETS writes.
  32 BigEndian→LittleEndian re-declared pairs with identical data/placements.
- Private sample: 12/32 LE programs, 54 devices, no LoadedImage on any of
  them, base images non-discriminating. Aggregates only.
- Owner's own installation: all BigEndian; no LE hardware available.

## Implementation (ADR-0119)

- `knx_core::commissioning::byte_order_evidence`: decode-only classifier
  (high/low first, either order, neither, single-octet controls, not
  classified) + tally/verdict.
- `knx_productdb::byte_order_probe::plan_probe`: shares `image::activate`
  (extracted, behaviour unchanged), `field_of`, union rule and value checks;
  names every skipped parameter; bound 4096 octets; refuses non-memory masks.
- `knx_net::commissioning::memory_download::read_memory_runs`: read-only
  session, mask/manufacturer check first; `check_identity`/`read_region`
  extracted from the compare/backup path (their tests unchanged and green).
- `knx_app::device_download::prepare_byte_order_probe` (device lookup shared
  with `prepare_device_download` via `device_at`).
- CLI `knx device byte-order-probe` (read only; exit 0 reported, 1 not probed).

## Measurements (throwaway tests, archived in scratch as .rs.keep)

- 40 small public packages, 42 LE 0701 programs at product defaults: 36
  planned, 6 refused by shared activation (`TypeRestriction Base="BinaryValue"`
  controlling a `choose` with no covering `when`), max 202 octets per device.

- Private sample, project values: 54/54 LittleEndian devices plan, 24 with
  discriminating fields (124), max 78 octets; 729 Property-placed parameters
  on 11 devices (blocked by Property placements as well). Reproduced twice.
- The 159 MiB ABB package was stopped after 26 min in debug; nothing claimed.

## Review and gates

- In-session review: IMPORTANT — planner lacked the download plan's 16-bit
  segment rule; `segment.address + offset` overflowed on a malformed address
  (regression test failed first with the overflow, fixed). MINOR — declared
  order printed Debug-quoted (fixed).
- 9 hand mutants, 9 killed (`no-short-read-check` survived first: no test
  anywhere covered a short memory answer → simulator `short_memory_answers` +
  test).
- Frozen gate (inputs frozen): fmt, clippy -D warnings, Rust 4229/0/196
  (= 4213 alpha.10 + 28 new − 12 knx-desktop, excluded: needs web dist), five
  xtask checks, docs, diff check; private corpus 143/143 in 31 targets.
- Privacy scan: 0 of 1,165 private identifiers in the added lines.

## Open

- Probe output from ETS-programmed LE devices (issue #10 reporter has one).
- Owner decision: warning for untested downloads of BigEndian/undeclared
  ABB/BJ 0701 programs with low-octet-first base images (§170 related risk).
