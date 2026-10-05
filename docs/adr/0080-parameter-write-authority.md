# ADR 0080: Parameter writes honour Access and leave manufacturer calculations alone

Date: 2026-10-05
Status: Proposed — AR07 write-authority package; acceptance follows the package gates.
Session: 4 (manufacturer semantics), AR07
Amends: parameter-editor design D24 ("`access` is display only, never gates a
write") and KNOWN_LIMITATIONS §3's "`Access` … is not used for write gating".

## Context

**Primary source [D].** The user-provided licensed *Project Schema23
v01.00.00* (KNX Standard v3.0.0, local `knx-spec-kb` extraction), §1.1.2.1
`Access_t`: "This enumeration encodes the rights for the ETS user to view and
modify parameters." Facets: `None`, `Read`, `ReadWrite`. RESEARCH §4.3 had
looked for `Access` as a *visibility/memory* mechanism (correlation with a
`Memory` child, roughly 50/50) and left it open; the type's own description
answers a different and simpler question: it is a user right.

**Corpus [V].** Read-only, aggregate-only census of 3,599 distinct application
programs (332 from `OriginalData/`, the rest from the 2026-10-03 public
crawler download; deduplicated by SHA-256, no identifiers kept):

- 7,048,845 `ParameterRef`s are offered by a `Dynamic` tree. For 2,123,414 of
  them the `Parameter` says `Access="None"` and the `ParameterRef` says
  nothing; 200,258 more carry `ParameterRef/@Access="None"`; `Read` occurs on
  both levels. KNXBench offered every one of them as writable.
- `ParameterRef/@Access` occurs on 80,962 refs in 301 of the 332 `OriginalData`
  programs, and 37,030 offered refs in 1,234 programs pair
  `ParameterRef/@Access="ReadWrite"` with `Parameter/@Access="None"`.
  The static parser stored only `Parameter/@Access`; `ParameterRef/@Access`
  was neither stored nor reported (probe: no `ingest_unknown` row), so the
  panel showed `None` for fields the manufacturer explicitly opened.
- `ParameterCalculation` occurs 116,799 times in 809 programs (87,889
  `Language="JavaScript"`, 28,910 `VBScript`). Its `LParameters` side is
  offered and writable in 779 programs; 1,129,422 of 1,148,529 `RParameters`
  refs carry `Access="None"`. KNXBench never runs these scripts (no vendor
  code, AR07), so an edited left-hand value leaves the right-hand values the
  manufacturer derives from it stale: the panel would show a configuration
  that the stored result parameters — and a later download — do not have.

**What is not established.** The schema text available here does not state
how `ParameterRef/@Access` combines with `Parameter/@Access`, nor the default
when both are absent. The calculations' semantics are vendor scripts.

## Decision

1. **Store `ParameterRef/@Access`** verbatim (`parameter_ref.access`), and
   record which `ParameterRef`s each `ParameterCalculation` names on its
   `LParameters`/`RParameters` side (`parameter_calculation_ref`). Nothing
   about a calculation is interpreted: its element stays a reported unknown
   construct and its bytes stay retained; the table is a safety index.
2. **Effective access** is the `ParameterRef`'s when present, otherwise the
   `Parameter`'s — the same layering KNXBench already applies to `Text` and
   `Value` (design D22). This precedence is an inference [A]; it only decides
   which fields *keep* today's write authority, it never grants a new one.
3. **A field is not writable** (`writeEtsId: null`, `editable: false`, a
   write is refused with 400 and changes nothing) when
   - its effective access is present and is not exactly `ReadWrite`
     (`None`, `Read`, or any unknown token), or
   - its `ParameterRef` appears on either side of a `ParameterCalculation`, or
   - the product database has not recorded write authority for its program
     (a program whose v20 backfill failed): fail closed.
   An absent access on both levels keeps today's behaviour; no default is
   invented.
4. The panel shows the effective access in `access`, and each section gets
   one warning per reason — `parameterAccessReadOnly`,
   `manufacturerCalculation`, `writeAuthorityUnavailable` — naming the
   affected fields in its detail. Fields stay listed: hiding `None` fields is
   a presentation decision for the UI owner (existing English fallback for
   unknown kinds applies until it adopts the tokens).
5. ProductDB schema v20 adds the column, the table and a per-program
   `write_authority_recorded` flag, backfilled from each retained program
   blob in its own savepoint. A blob that cannot be re-read is recorded as
   `WriteAuthorityBackfillError` and its programs stay unrecorded (rule 3).

## Alternatives considered

- **Keep D24 (display only).** Contradicts the documented meaning of
  `Access_t` and keeps 2,505,462 offered refs (35 %) writable whose effective
  access is `None` or `Read`; rejected.
- **Most restrictive of both levels.** Would remove write authority from the
  37,030 refs the manufacturer opened on purpose; it is not safer than the
  layering rule in any case where the layering rule grants authority that
  today's code does not already grant. Rejected.
- **Run or translate the calculations.** Executing vendor JavaScript/VBScript
  is out of scope by policy (AR07: no unknown vendor logic); rejected.
- **Allow the edit and warn.** Leaves an inconsistent configuration in the
  project for a later download; rejected for a read-only refusal.
- **Hide `None` fields in the backend DTO.** Hiding is presentation and would
  also hide evidence; left to the UI owner.

## Consequences

- Fewer fields are editable — by design. Projects imported from ETS keep
  every stored value; only new KNXBench writes are refused.
- An existing product database is migrated in place; a blob that cannot be
  re-read makes its programs read-only until the product is reinstalled.
- `ParameterCalculation`, `Allocator`, `Repeat`, rename and button semantics
  stay unimplemented (KNOWN_LIMITATIONS PDB-9); this ADR adds no evaluator.
- The UI owner adopts the three diagnostic tokens and may hide `None` fields.

## UI presentation (UI owner, 2026-10-05)

- The three tokens are part of Web's manual `ParameterDiagnosticKind` union
  and translated in English and German like every other kind; so are
  ADR-0061's `unsupportedControlKind` and ADR-0062's
  `evaluationWorkBudgetExhausted`, which had been waiting on the same English
  fallback. `detail` (the affected field ids) stays English, as for every
  kind (KNOWN_LIMITATIONS §66).
- **`None` fields are folded, not dropped.** Because `Access_t` is the user's
  right to *view* and modify, a section folds its fields whose effective
  access is `None` behind one button that names their count ("Show 2 fields
  without user access (Access None)", `aria-expanded`); opened, they are
  listed read-only like any other refused field. `Read` fields stay visible
  and read-only. The section's `parameterAccessReadOnly` warning remains
  visible either way, so a folded field is never unexplained. The fold is
  per section and per panel view, not a stored preference.
