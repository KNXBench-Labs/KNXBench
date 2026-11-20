# Descriptor-qualified exact-0702 run state — software acceptance (PRG-01-N2)

Software acceptance: **20 November 2026**, simulator/loopback evidence only
(owner decision 2026-11-20: B-05/B-08 allow simulator-only evidence on the
declared list; WP-12 stays blocked). Branch `work/prg01-n2-20261120` off
`origin/main` `64ea464ae`. This record certifies the branch tree; publication,
readback and release gates remain separate delivery checks.

The [contract](../contracts/post-download-run-state-0702.md), the
[original-source investigation](../research/2026-11-20-property-run-state-observation-0702.md)
and [ADR-0180](../adr/0180-exact-0702h-run-state-observation.md) add exact
`0702h` to the bounded optional-property observation with the same shape as
descriptor-qualified exact `0700h`: Resources footnote 94 grants the legacy
`PDT_UNSIGNED_CHAR` datatype only to masks `0020h`/`0701h`, so `0702h` is
`PDT_CONTROL` and requires an independently read qualifying PID 6 property
description before one strict `PID_RUN_STATE_CONTROL` value read. Identity
match, all load-state reads and strict candidate-3 PID 1/PID 13 binding
precede it, unchanged. The `0700h`/`0701h`/`0705h` schedules, gates and
serialized record bytes are untouched; other `070nh` masks (tested `0703h`,
`070Fh`) still report `UnsupportedMask`.

## Executed evidence (this branch, real tool output)

- `cargo test -p knx-net --lib commissioning::post_download` — 61 passed, 0 failed.
  New `exact0702` module: six state octets, qualification precedence, legacy-bit
  values, candidate/description/value failure stops, literal wire continuation
  (`0x0702` description read present, `0701` stays description-free).
- `cargo test -p knx-app --lib post_download` — 27 passed, 0 failed. New
  `exact0702` validator tests over both mask markers and the full description
  vocabulary (qualified, `notQualified`, `unavailable`), plus the shared
  web case table (accepted 33 / refused 137, including 12 new `0702` cases).
- `cargo test -p knx-cli --bin knx` — 240 passed, 0 failed, 9 ignored; the two new
  `exact0702` CLI tests pass (qualified observation reports `running` with the
  `0702` marker and survives report/reopen; unqualified descriptions never
  reclassify the written transfer).
- `cargo test -p knx-server --bin knx-server` — 5 passed, 0 failed; the new
  `exact0702` HTTP test carries worker results through status, report and
  reopened history with zero consumer-triggered device traffic.
- `npx vitest run src/postDownloadCheck.test.tsx` (apps/knx-web) — 21 passed,
  0 failed, including EN/DE rendering of the three new `0702` fixtures.
- e2e `device-download-verification.e2e.ts` gained three `exact0702` fixture
  rows asserting the *rendered* substitutes (`runState.step.description` +
  `runState.cause.noCorrelatedAnswer`), not the raw template — the UI
  interpolates `{step}`/`{cause}` before display.

## Package-gate receipt and documented pre-existing failures

`python3 tools/run_package_gate.py --base origin/main` (change-scoped,
inputsFrozen=True), receipt
`target/package-gates/1795191228056680738/receipt.json` on this branch
(`b67bb2431`, the only commit in `origin/main..HEAD`; `git diff
64ea464ae..HEAD --name-only` touches none of the failing test files).
Green stages: `fmt`, `documentation`, `browser-lanes`, `check-layering`,
`check-headers`, `check-anchors`, `check-ledger`, `check-corpus-gates`,
`tool-tests`, `web-gate` (37 stages, 1475 browser tests: 1380 functional +
95 exclusive), `clean-desktop`, `clippy`. The `rust-tests` stage failed with
exactly 6 failures, all reproduced on a clean `origin/main` scratch checkout
(`d18687ba1`) and therefore pre-existing / environment-caused, not caused by
this diff:

1. `c49_partial_group_addresses::the_49_report_plan_reproduces_offline`
   (knx-app) — known pre-existing: `C49_PROGRAM_XML path: NotPresent`
   (private-corpus env unset); documented, not fixed.
2–6. `http_parameter_grid::{a_declared_grid_is_versioned_geometry_with_cells_and_active_captions,
   a_stored_axis_text_parameter_value_wins_over_the_product_default,
   an_axis_text_parameter_ref_resolves_to_the_parameter_value_with_provenance,
   an_unknown_axis_text_parameter_ref_keeps_the_source_text_and_names_the_reason}`
   and `http_parameter_captions::translated_axes::literal_axis_http_text_reference_resolves_the_parameter_not_the_source_caption`
   (knx-server) — reproduce identically on clean `origin/main`
   (`cargo test -p knx-server --test it http_parameter`: 132 passed, 5 failed
   there too); they belong to the unmerged WP-05n2/WP-05c literal-axis-captions
   work, not to this branch.

The change-scoped suites this increment owns are green on both trees with the
branch delta: knx-net post_download 61 vs 54 passed (main), knx-app
post_download 27 vs 24 (main) — the +7/+3 are the new `exact0702` tests,
absent from clean main.

## Boundaries

KNX-compatible, **hardware-untested**: no live bus, no real device, no ETS VM,
no release. Unknown data preserved; `notQualified`/`unavailable` records keep
their cause and never imply absence, authorisation or a hardware fault. Other
object layouts, memory encoding and physical function remain unproved (PRG-01
open list).
