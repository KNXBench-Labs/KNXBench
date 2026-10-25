# Duplicate image-start admission acceptance — 25 October 2026

## Published source

`d73a2413b` is published on `main` with matching live and tracking-ref readback.
The main-first merge preserves upstream `7553fdcf5` outside five reviewed paths.
The regular memory planner refuses an allocation that matches multiple data-bearing
image segments at the same start, rather than silently choosing the first payload
([contract](../contracts/duplicate-image-segment-start.md)). The diagnostic names
that address and the first two segment IDs in image order.

This is conservative executable-plan admission, not XML/schema invalidity. Import
and the public image builder retain all source segments. Equal payloads, disjoint
masks and fully protected masks do not establish a precedence or merge rule. A
payload-free RAM/stack allocation still emits no data writes and remains allowed.
Distinct starts, ordinary mask handling and existing allocation checks stay intact.
An unallocated duplicate pair still reports `NeverWritten`; at an allocated start,
ambiguity precedes either twin's `SizeMismatch`. Partial overlaps of different
starts are outside this increment. Read-only comparison and byte-order preparation
that reuse this planner also refuse ambiguous input rather than selecting one twin.

## Source and regression evidence

The synthetic fixture follows public XML ingestion, a scratch catalogue, public
`build_download_image` and the regular planner. It proves that both distinct source
payloads survive with their IDs and source order before testing planner admission.

- Original runtime with the original test revision: four methods executed, two
  controls passed and two intended semantic refusal assertions failed. With A first,
  the successful plan emitted only `AA BB`; with B first, only `CC DD`. The retained
  original runtime matched the immutable base. Parent retained-binary replay confirmed
  both actual omitted-payload results, not a setup or compilation failure.
- Final tests tighten typed error/diagnostic assertions and add rows; their source is
  retained separately from the original RED. Six public integration methods pass,
  including both orders, identical bytes, four mask combinations, distinct starts,
  payload-free allocation and unallocated twins.
- Four new unit methods cover ambiguity, deterministic source order, unallocated
  twins and ambiguity before a size mismatch. All 25 planner-module methods passed
  in parent replay, including unchanged ordinary allocation/size/mask controls.
- The worker's full owning selection passed 778 library and 802 integration tests,
  zero failed, with 31 integration tests ignored. Executions used a verified non-root,
  capability-free, loopback-only namespace. Parent authenticated source and retained
  executable hashes and replayed the original and corrected focused selections.
- Independent Opus source review approved without blocking findings. It did not
  execute tests; parent owns executable and integration acceptance.

## Integrated acceptance on `d73a2413b`

All fourteen stages selected by the maintained change-scoped runner passed in order,
with identical start/end input commitments and empty touched/unchecked user-data lists.

- Rust: 4,225 passed, zero failed, 154 ignored across 24 result blocks. All 25 planner
  and six new integration methods passed on this merge too. Overlapping worker and
  focused totals are not added to the integrated count.
- Private corpus: 97/97 across six selected targets. This is selected regression
  evidence, not a census of ambiguous source products or broad compatibility.
- Formatting, Clippy, tooling, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration and final diff checks passed.
- The frontend was freshly built for the desktop Rust prerequisite. Browser execution
  was not selected; interactive desktop, release and deployment acceptance are not claimed.

## Remaining boundaries

No real product is claimed to contain this ambiguity, and no physical installation
was contacted. Source/parser/image bytes are not rewritten. No new family, encoding,
interval-overlap policy, Verify Mode, recovery or KNX Secure capability is admitted.
Application support classification remains the existing `ProcedureContents` fallback.
Only the first two twins are named if more exist. Parameter policies and wider
programming work remain open under [PRG-01](../open-work/PRG-01-programming-gaps.md)
and [KL-007](../limitations/KL-007-bounded-commissioning-support.md).
