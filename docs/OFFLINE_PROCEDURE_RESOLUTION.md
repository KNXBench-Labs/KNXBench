# Offline System-B AP1 procedure resolution

Status: implemented and locally verified in the unpublished `feature/offline-procedures-20261009` worktree. Commit/main integration/push/release/deployment remain separate.
Date: 2026-10-09. Owner go follows the Q1–Q11 interview.

## Evidence and first family

Read directly from the local primary PDFs (not previous extracted summaries):
- KNX Configuration Procedures `03_05_03` v02.01.01, §3.9.3.1–3.9.3.4, printed pp. 71–77: merge points integrate application-specific procedures into the default sequence; 2/4 are AP1 allocation/data, 3/5 are AP2; 1/6/7 are optional. The complete sequence explicitly excludes AP2 loading/writing when only AP1 is loaded and handles AP2-unload errors separately.
- Project Schema23 v01.00.00, §1.1.2.11–1.1.2.14, printed pp. 12–13: three load-procedure styles and Load/Unload procedure types. These enumeration definitions do not describe every manufacturer-side attribute or its default.

A bounded, read-only scan of all 105 local product archives (2026-10-09) found 75 `MergedProcedure` application occurrences with `MV-07B0`, each a distinct program ID in this scan. This is raw package occurrence evidence, not the 467-program installed first-winner coverage denominator. Fragments include conditional `choose` nodes, not just flat controls; 28 occurrences supply only merge 1. Every completed scan preserved the original archive hashes. No manufacturer content is reproduced here.

First slice: `MV-07B0`, explicitly `Load/ap1`, from the same analyzed product package. Master data carry an AP1 template with merge points 1/2/4/6/7. Its sequence is supplied by the selected source; constants and sentinel-looking values are not interpreted or rewritten. `all`, partial scopes, other masks, legacy formats and new media are outside this slice.

## Corpus-derived master path correction

The first full resolver run returned 75 unavailable results: 69 missing-template
refusals and six source-byte-limit refusals. It exposed a synthetic-fixture bug,
not new compatibility: real AP1 templates are under the exact expanded path
`KNX/MasterData/MaskVersions/MaskVersion/HawkConfigurationData/Procedures/Procedure`,
not directly below `MaskVersion/Procedures`. A bounded follow-up inspected 19
relevant packages' AP1 templates; all selected wrappers and Procedures containers
had no attributes. A separate `LegacyVersion="1"` wrapper in the inspected sample
had no AP1 candidate. Neither inspected primary PDF defines that qualifier.

Only the observed unqualified path is admitted. Exactly one explicit Load/ap1
candidate is required across those containers; duplicates or qualified/container
metadata are named refusals. No first/newest/legacy-wrapper choice or qualifier
semantics is inferred. Direct-mask synthetic templates are not a fallback.
Corrected-fixture regressions and the new complete corpus run passed before
local acceptance; the first run remains a superseded diagnostic, not successful
real sequence reconstruction.

## Contract

- Productdb owns a bounded, namespace-aware, read-only resolver. Application services select exact source identities within the analyzed package. UI only displays results; no parser in UI.
- Program selection uses exact canonical ancestor paths, unqualified attributes and a single matching ID. Duplicate program/mask/template/fragment identities, missing/multiple master sources, namespace disagreement, corrupt hashes and malformed input are named refusals, not first/last-wins.
- `MergedProcedure`: splice the selected program's fragments into the selected mask AP1 template in source order. Mandatory merge points 2 and 4 must exist and have fragments. Absent optional 1/6/7 fragments are explicitly recorded as omitted, never silently discarded. Unused fragments are reported.
- `DefaultProcedure`: inspect the explicit template; unresolved mandatory merge points remain incomplete. No product fragment is silently used to turn a default into a merged procedure.
- Unknown controls, children/conditions, foreign namespaces/attributes, nested merges and unknown merge IDs remain visible, with named issues. Nested merges are not recursively interpreted; that also prevents cyclic expansion. Preserve their original subtree and source offsets/hash; no shortened executable sequence is derived.
- `expanded` means the ordered declarative sequence was reconstructed. `partial` means unresolved issues remain. Neither means device semantics, image/table substitution, parameters or a complete offline download plan. `executable` is always false. Existing live readiness and evidence remain unchanged.
- Local provenance includes program identity, source SHA-256, exact namespace, template attributes and source byte offsets/step locations. These are local source-bearing diagnostics, not anonymous reports.

## Bounds

Per source: UTF-8 XML only, 8 MiB, 200,000 events, depth 64, 65,536 nodes, 256 attributes per element (including namespace declarations), and a 64 MiB conservative retained-tree charge. Expanded namespace/name strings, retained text and attributes are charged cumulatively before storage; the charge is not a measured process-RSS guarantee. Per resolution: 2,048 emitted steps and 1 MiB conservatively charged output; overflow is explicit, never complete. The caller retains the existing 64-program analysis cap and additionally bounds aggregate resolution disclosure. Original bytes stay retained by the existing product storage, not copied into new database tables. No schema migration or production input-limit widening.

## Findings and disclosure

Reuse support-gap UI/API/CLI, exact preview/consent/export and manual GitHub handoff. Distinguish observation, missing/conflicting evidence and unsupported interpretation. No global novelty claim, automatic upload, telemetry or private mailbox provisioning. Reduced outgoing reports retain value-free counts/categories; clear all detailed resolution objects (including IDs/hashes/steps/template attributes). Selected contextual members still require the existing explicit permission and preview. Known secret-bearing inputs withhold detailed local resolution data too.

## Acceptance

Named unit/integration regressions for source order/provenance, optional omissions, mandatory/unused/duplicate fragments, missing/conflicting/corrupt sources, exact namespace/attribute handling, nested/cyclic merges, unknown conditions/controls, malformed XML and budgets. Exercise actual application analysis and reduced preview/export, HTTP/CLI and built UI. Existing memory download readiness must still refuse the new family and hardware Verified must not change. Run the full private-corpus regression separately, retain aggregate results only, verify original hashes unchanged. No new hardware or ETS acceptance is implied.

## Measured local acceptance — 2026-10-09

[Privacy-safe execution receipt](evidence/offline-ap1-resolution-2026-10-09.json).

- Final owning-crate Rust suite: **1945 passed / 0 failed / 118 ignored**,
  ProductDB/App/server/CLI, including all seven XML declaration/resource controls.
  Fmt, warning-denied all-target Clippy, server/CLI builds, all five nonempty
  checkout-built repository checks, documentation and whitespace pass. Not a
  full-workspace, native or hardware gate.
- Web: **2494 passed / 0 failed in 154 files**, production TypeScript/Vite build
  pass; exact web-input equality binds these results to the final backend round.
- Separate explicit ignored Release tests: new resolver and existing download
  coverage each **1 passed / 0 failed / 0 ignored**. All **105 original archives**
  unchanged; existing **467 programs / 1 Verified / 98 Untested** unchanged.
- Selected family: **75 program IDs / 75 package-source-bound results**:
  **69 partial ordered reconstructions, six unavailable at the 8 MiB source
  bound, zero expanded real sequences**. No complete plan or new live support.
  Counts and issue-code totals are pinned, not just `results > 0`.
- Real built, unmocked browser analysis/preview/consent/export/manual-link checks:
  EN/DE, 1440/400px, Graphite/Porcelain, loopback-only namespaces. Eight actual
  reduced ZIPs (expanded synthetic and unknown-step synthetic cases) checked
  against their manifest bytes/hashes. Source identities, fingerprints and
  detailed resolution objects absent; fixed unknown-step count remains. No
  external requests or issue submission. Nonempty seeded HTTP project/product
  state preservation is tested separately; browser starts with no project.
- Representative built screenshots inspected: readable contrast, ordered source
  labels, explicit incomplete status and wrapping within the dialog. Narrow
  views require vertical scrolling; sticky header/scrolled captures do not
  establish an entirely visible warning at every scroll position. No native
  WebKitGTK/Orca/accessibility or large-project performance claim.
- Independent bounded XML shape estimate: 85 distinct relevant documents,
  maximum estimated retained-tree charge 40,883,658 bytes and 24 retained
  attributes per element (namespace declarations excluded by the estimator).
  It normalizes line endings and is not the authoritative parser charge or RSS;
  the actual full regression confirms the 64 MiB/256 bounds preserve outcomes.

Reproduce the opt-in corpus check with the private originals available:

```sh
KNXBENCH_PRODUCT_CORPUS="<private-corpus>" cargo test --release -p knx-app \
  --test procedure_resolution_corpus -- --ignored
```

Review is **self-review**, not independent review. Important findings (report
projection amplification, XML declarations, actual master ancestor and retained
tree/attribute budgets) were reproduced by named failing controls before their
fixes. Source/configuration stayed frozen during final gates and built runtime.
Raw private inventories, corpus identities, temporary executables and probes
are not public deliverables; retain the aggregate receipt and implementation.
