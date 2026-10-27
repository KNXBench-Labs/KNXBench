# Group-address CSV import profiles: source acceptance

## Delivered boundary

Published main-first merge `ed8b8ec136d8d009bbfc5bd85806c66227a6152a`
(feature `301a795b6799a68b9b671d3cd95dd4fd863371fd`) adds four fixed
name/address column layouts to the shared address-only CSV import path.
[Contract](../contracts/group-address-csv-profiles.md).

Native CSV output and native extra-column precedence are unchanged. Range rows
are diagnosed and never applied; nonempty Description, unsupported Security,
ambiguous headers and malformed addresses refuse the complete import. DPT
remains read-only. `Auto` is explicitly not imported, not a guessed security
value. No schema, external export, ESF/OPC, hierarchy/security roundtrip or
general application compatibility claim.

## Executed evidence

- Original native parser baseline: 62 tests passed. New profile tests produced
  35 intended original-source failures; native-preservation and later DPT-token
  corrections each have separately retained semantic failures and passing runs.
- Final owning CSV suite: 104 passed, no failures or ignored cases.
- Focused built-CLI CSV selection: 14 passed. Server CSV selection: 15 HTTP
  and two unit cases passed. MCP CSV validation: two passed, never applied.
- Existing Web CSV/API consumer tests: 104 passed in two files. This is unit
  coverage, not a new browser acceptance or new UI implementation.
- Independent whole-feature review APPROVE; six minor findings corrected and
  focused independent correction review APPROVE. Range preservation, exact
  refusal diagnostics, plain-digit tokens and padded names have regressions.
- Maintained change-scoped branch and integrated package gates: **14/14 each**.
  The integrated selection passed **2,878 Rust tests** in 20 result blocks,
  **129 ignored**, and the separately selected **91/91 corpus cases**. Tooling
  registered 461 methods, with six optional skips. Clippy and all selected static
  checks passed. Test totals across overlapping runs are not added together.
- Both receipts prove complete plans, identical start/end source fingerprints,
  and empty changed/unchecked user-data collections. Local integration,
  tracking ref and live remote equalled the published merge at readback.

## Exclusions

No new external receiving-application, browser, physical-bus, device, recovery,
release or deployment acceptance is implied. The package gate selected no Web,
bindings, website or story stage; the real web bundle was built as the desktop
build prerequisite. Runtime and test inputs remain unchanged by this result note.
