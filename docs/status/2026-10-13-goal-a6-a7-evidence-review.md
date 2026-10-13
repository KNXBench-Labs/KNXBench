# Goal A6/A7 evidence review — 13 October 2026

Session `62a4cca1c10f`, [Goal A](../goals/unassigned-data-integrity.md)
packages A6 and A7. Documentation only: no runtime change, no hardware, bus,
ETS or manufacturer execution, no repeated research. Formal status stays in
the [ledger](LEDGER.md); nothing here is an artificial `DONE`.

## A6 — PDB-04, KL-68, KL-171, KL-172: blocked, no new material

**New-material check (method).** The goal admits only genuinely new,
authorized primary sources or artifacts. Checked on 13 October 2026:

- the private corpus (`OriginalData/`, gitignored): files modified since
  10 October are three ETS6 plugin-pair exports of 13 October, which belong
  to the parked ETS6/SmartSensor host track the goal excludes, and one
  private example project modified on 10 October at 07:16, before the
  KL-171/KL-172 research commit `4fd23f6a` (10 October, 14:47) and the
  PDB-04/KL-68 slices of 10–12 October;
- the local KNX specification collection: unchanged since 11 September;
- the *Project Schema23* already cited by
  [schema23-offline-boundaries](../research/schema23-offline-boundaries.md)
  (the copy used for Goal A4 has the same title and version).

No new primary source exists, so no research was repeated. Each item stays
blocked on external evidence with its current fallback:

| ID | Evidence gap | Test plan once evidence exists | Current fallback |
| --- | --- | --- | --- |
| `PDB-04` | No documented value codec for `TypeColor` (lexical forms differ in projects, `@Space` uninterpreted) or `TypePicture`; no evidence for mapping module-scoped parameters beyond the delivered subset ([coverage dossier](../limitations/parameter-and-dynamic-coverage.md)) | Codec round trip per documented form (parse → canonical → byte image) with refusal of undocumented forms; corpus counts of every lexical form seen; module mapping against an ETS project whose module instances are known | Original-value view, non-empty XML-safe check, no editor; image refuses what it cannot encode |
| `KL-68` | Schema23 lists `RepeatIndex` but defines no activation, iteration-to-instance mapping or omitted-counter rule ([dossier](../limitations/KL-068-repeated-module-activation.md)) | A real project with repeated modules and ETS-known instance values: expansion count, per-iteration `RepeatIndex`, value isolation between siblings, and refusal of anything else | Own stored instance values are readable (ADR-0108); activation, writes and inference refused |
| `KL-171` | No source fixes whether a UTF-8 `TypeText` field counts octets or characters, or how a sequence that does not fit is cut ([dossier](../limitations/KL-171-utf-8-text-download-encoding.md)) | Image tests with 1–4-octet characters at the field boundary, exact fit, one octet over, and the documented truncation rule; corpus check of the 21 `utf-8` program files | Whole image refused once a UTF-8 text field is encoded; display and editing unaffected |
| `KL-172` | `LoadedImage`/`CheckSums` inflate to binary of an undocumented format ([dossier](../limitations/KL-172-stored-loaded-image-semantics.md)) | With a documented format: decode a synthetic image, then aggregate-only comparison with KNXBench's own image for the same program and values; never an input or authority | Kept verbatim as known, unmodelled attributes; never decoded |

Lifted only by a new authorized source (publication, manufacturer
documentation, or an owner-approved controlled comparison under the
hardware rules), not by more searching.

## A7 — KL-110, GAP-T30-04: prerequisites reconciled, gates remain

Both were prepared on 11 October 2026 (source `aeb5a386`) and need no new
research:

- **KL-110 (PL110 group responser table):**
  [powerline readiness](../contracts/powerline-readiness.md) lists the
  prerequisites and phases P0 (re-read sources, adapter contract) and P1
  (offline codec/service/simulator tests) without hardware, then P2–P4
  (read-only bench, one mutation, recovery), each needing its own approval.
  The [dossier](../limitations/KL-110-pl110-group-responser-table.md) still
  holds: no PL110 support exists, so no plan can target such a device yet;
  the property is implemented, or such a plan explicitly declined, when
  PL110 support is scoped.
- **GAP-T30-04 (changed-parts download):**
  [incremental download readiness](../contracts/incremental-download-readiness.md)
  separates complete download (the unchanged fallback), partial part/segment
  reload and changed-range writes, lists six design prerequisites (device
  scope, immutable image receipt, exact baseline, diff over verified
  writable regions only, recovery receipt before any write, exact
  provenance) and phases D0–D6, of which D3–D6 need separate hardware
  approval.

**Remaining gates (unchanged):** a nominated device and topology, the
applicable CP/RES/profile and manufacturer clauses, a durable recovery
receipt before any write, and an operation-specific owner go. Stored
`LoadedImage` (KL-172) is no baseline. Neither KNXBench nor this goal
implements a new medium or a differential download; ledger rows stay
`LATER / RECORDED_SCOPE`. What changed on 13 October: the ledger rows and
the KL-110 dossier now link the readiness contracts.
