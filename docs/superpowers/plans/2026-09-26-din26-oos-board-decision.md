# DIN-26: Board out-of-scope acceptance — remaining documented boundaries

Source: goal.md §10 ("every remaining exception carries the user's explicit
out-of-scope acceptance") and §4. Prepared by Sigrid Holz (Project Lead),
2026-09-26. Re-verified against the current `docs/KNOWN_LIMITATIONS.md` on
this date at commit `7b64496` (branch `din-26-board-oos-acceptance`).

This was a decision request; it was decided on 2026-09-27 (see **Decision**
below). Each line below is a remaining
documented boundary that goal.md §4 explicitly scoped as Priority 3
("reporting, diff and CSV residue — none is a correctness risk") or that
independently has no task scheduled to close it. None of them touches
hardware, data integrity, or persisted-format compatibility. The Board
accepts or rejects each one; nothing here decides on the Board's behalf.

## Reporting (`knx-report`)

- **§45 — no native PDF output.** HTML + browser print/PDF is the supported
  route; a Rust PDF renderer was deliberately not added (unnecessary
  dependency for what the OS already does).
  Evidence: docs/KNOWN_LIMITATIONS.md#45-project-documentation-export-has-no-native-pdf-output

- **§48 — prose catalogue is English/German chrome only, not complete.**
  Primary navigation, title and product strings are localized; detailed
  table labels, enum/debug values and some diagnostic sentences remain
  English; no third language or report-specific language pack exists.
  Evidence: docs/KNOWN_LIMITATIONS.md#48-project-documentation-export-has-englishgerman-chrome-but-not-a-complete-prose-catalogue--partially-resolved-2026-09-23-t14

## Project diff (`knx-diff`)

- **§52 — cannot correlate a device with no individual address and no
  matching `ets_id`.** No stronger identity exists in the domain model;
  measured zero occurrences across all three corpus projects.
  Evidence: docs/KNOWN_LIMITATIONS.md#52-project-diff-cannot-correlate-a-device-with-no-individual-address-and-no-matching-ets_id

- **§53 — can collide two same-named sibling building parts.** Name-based
  path key has no way to distinguish same-named siblings; measured zero
  occurrences across the corpus.
  Evidence: docs/KNOWN_LIMITATIONS.md#53-project-diff-can-collide-two-same-named-sibling-building-parts

- **§54 — does not detect an ETS re-import's regenerated `RefId`s as "the
  same project".** No re-import case with regenerated ids has ever been
  observed in the available corpus to design against.
  Evidence: docs/KNOWN_LIMITATIONS.md#54-project-diff-does-not-detect-an-ets-re-imports-regenerated-refids-as-the-same-project

- **§55 — cannot merge or apply a diff back onto a project.** Applying a
  two-way observation is a merge engine (conflict semantics, revision
  checks, inverse commands for every field), not a renderer extension; a
  partial implementation risks silently corrupting project data.
  Evidence: docs/KNOWN_LIMITATIONS.md#55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project

- **§56 — no three-way comparison.** Nothing in the codebase tracks project
  ancestry or a common base; a useful three-way merge action would also
  depend on §55, which is itself out of scope here.
  Evidence: docs/KNOWN_LIMITATIONS.md#56-project-diff-does-not-do-a-three-way-comparison

## Group-address CSV

- **§39 — CSV import does not create or rename group ranges.** Range CRUD
  is separately modelled structure; inferring range mutations from repeated
  `MainGroup`/`MiddleGroup` text would introduce ordering/boundary/rename
  ambiguities the project will not guess at. No dedicated range-exchange
  contract is scheduled.
  Evidence: docs/KNOWN_LIMITATIONS.md#39-csv-import-does-not-create-or-rename-group-ranges

- **§41 — unverified spreadsheet transformations remain.** German-locale
  `,`/`;` separator detection, RFC 4180 quoting, BOM and line-ending
  handling are implemented and tested; broader Excel-induced reformatting
  beyond the separator is not enumerated anywhere and has no concrete
  failing sample to design a fix against.
  Evidence: docs/KNOWN_LIMITATIONS.md#41-german-locale-separators-are-supported-unverified-spreadsheet-transformations-remain

## Manufacturer data

- **§12 (ambiguous-DPT residue only) — an ambiguous, space-separated
  `DatapointType` list still fills nothing.** Measured: 22 slots across 11
  devices of 907 communication objects genuinely stay unfilled (down from
  a much larger apparent count once already-stated and `Empty` slots are
  excluded). No mechanism exists anywhere in the stack to surface or
  persist a user's manual choice; building one is a separate, unscheduled
  design. (Everything else in §12 — program-default resolution — is
  already lifted; only this residue remains open.)
  Evidence: docs/KNOWN_LIMITATIONS.md#12-manufacturer-data-resolution--one-of-three-gaps-closed-2026-09-20

## Product-package integrity

- **§85 — a `.signature` package member is stored, never verified.**
  Verifying it would need the file's own format/algorithm and the
  manufacturer's public key; neither exists in any corpus this project can
  reach (checked by direct grep of the accessible KNX Standard corpus and
  by querying both project knowledge bases across sixteen distinct search
  terms — zero relevant hits). The display already stopped implying
  anything was checked (T05).
  Evidence: docs/KNOWN_LIMITATIONS.md#85-a-signature-package-member-is-stored-with-role-signature-never-verified

## Import validation

- **§2 — no authoritative XSD is publicly available.** Official schemas
  require KNX membership and are not accessible to this project; structural
  validation (stage 4) is hand-built and bounded by the corpus rather than
  checked against a real schema. Lifted only if authoritative schemas
  become available to the project — nothing here can be built around that
  absence.
  Evidence: docs/KNOWN_LIMITATIONS.md#2-no-authoritative-xsd-is-publicly-available

## Re-verification note

All entries above were re-checked against the live text of
`docs/KNOWN_LIMITATIONS.md` on 2026-09-26 (not from memory or an older
draft). None of the goal.md §6 "Accepted out of scope" items (T19 KNX
Secure, T20 Functions, T22 multi-user, T21 spatial canvas, E4 LTE DPTs,
§68/§69/§71 module handling, §13 AES half, §1 schema evidence, `.vd2` /
unobserved schemes, ETS re-import, §6 plug-in DLLs, plugin API, online
catalogue update) are repeated here — those already carry a recorded
decision and reopening them was explicitly ruled out. The commissioning/
hardware-write exclusions from goal.md lines 48-53 (§7, §92, §93, §99,
§101, §104, §105, §108, §109, §111, §112, §113, §114, §115, §116) are
likewise excluded here — they are governed separately by the hardware
rules and are not this issue's remit.

## Decision

**Accepted 2026-09-27.** All twelve documented boundaries listed above — §45,
§48, §52, §53, §54, §55, §56 (reporting and project diff), §39, §41
(group-address CSV and manufacturer data), §12, §85 (product-package integrity)
and §2 (import validation) — are accepted as permanent documented boundaries.
No work is scheduled against them.

Accepted by the project owner. This closes the "pending explicit acceptance"
state that goal.md §10 requires before the alpha gate can be called complete.

What this decision does **not** say:

- It does not claim ETS parity in any of these areas; each remains a documented
  difference, and the corresponding `KNOWN_LIMITATIONS.md` entry stays open as
  the public record.
- It does not forbid revisiting an item. Acceptance means "no scheduled work
  now", not "never"; a future concrete need re-opens the question.
- It does not cover anything touching hardware, data integrity, or persisted
  formats. No item in this list does — that was a precondition for asking.

## What the Board is being asked

For each item above: accept as a permanent documented boundary (no work
scheduled against it), or send it back with a concrete reason to schedule
work. No merge, tag or release depends on this decision; it only closes the
"pending explicit acceptance" state goal.md §10 requires before the alpha
gate can be called complete.
