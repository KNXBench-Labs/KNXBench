# ADR 0125: A site over several installations is not modelled; its invariants are fixed and it waits for evidence

Date: 2026-10-13
Status: Proposed — blocked design (no implementation until the evidence in *Lifted when* exists)
Session: Goal A4 (`docs/goals/unassigned-data-integrity.md`), design only — no code, no UI

## Context

MODEL-05 asks whether separate installations can be grouped beneath one
site (RESEARCH §17.3: "Grouping *separate* installations under one site is
unmodelled and would need its own ADR"). ADR-0038 already answers the
single-installation case: a site is a `Ground` space at the root of one
installation's building structure (KL-127 keeps its ETS evidence open).
This ADR covers only the multi-installation case.

Evidence is marked **[D]** for a statement in *Project Schema23 v01.00.00*
(SHA-256 prefix `8fe9193e70135064`, read with `pdftotext -layout`; page
numbers are PDF pages) and **[M]** for a measurement made for this ADR on
2026-10-13.

- **[D] A project has 1 to 16 installations.** `Project_t`: "Most project will
  just have one Installation. Count of installations must be in [1…16]."
  (p. 33; §1.2.3.12–13, p. 38.)
- **[D] Every building structure belongs to exactly one installation.**
  §1.2.6.1 `element Project_t/Installations/Installation/Locations`, "Contains
  the building structure" (p. 54), and the location, topology and group IDs
  are scoped by "ID of the parent Project and InstallationID" (the `parid`
  rows on pp. 40–43 and 55–60).
- **[D] The only cross-installation link the schema defines is a global group
  address.** `GroupAddress/@Global`: "If true, the group address will be used
  in all installations of the project. Global groups must have the same
  address and type in all installations of a project." (p. 59.) No element
  groups spaces, buildings or devices of several installations.
- **[M] No sample has more than one installation or a `Ground` root.** The 6
  readable private-corpus projects and the 8 readable public xknxproject test
  projects (checkout `f97db068`; two further ETS6 samples need a derived
  password and were not opened) each have exactly one `Installation`, and
  every root space is a `Building`. No `Type="Ground"` occurs anywhere.
- **[C] Current model.** `Project.installations` is a list; MODEL-01 made
  commands act in the installation that owns the addressed entity, and root
  creates take an explicit installation. Building parts, group ranges, group
  addresses, lines and parameters are owned by one installation; IDs are
  project-unique.

## Decision

KNXBench does **not** add a cross-installation site, and it implements **no
operation that moves or links entities across installations**. A site that
covers several buildings stays the ADR-0038 `Ground` root inside one
installation.

Should a multi-installation site be admitted later, it must satisfy these
invariants, so that no future design has to rediscover them:

1. **Single owner.** Every building part, device, line, group range, group
   address, parameter and Function has exactly one owning installation. A
   site *references* installations; it never owns their content or
   duplicates it.
2. **Native, not ETS.** The ETS format has no site element above
   installations [D], so the site is KNXBench-native metadata. Import never
   synthesises one, and an export to an ETS-shaped format must state that it
   is dropped (or keep it in a KNXBench-only extension), never fold it into a
   `Ground` space of one installation.
3. **No implicit moves.** Moving a building, device or group address to
   another installation is a separate, explicit operation with its own ADR:
   it changes topology scoping, group-address uniqueness and `parid`-scoped
   identities, and must be atomic and undoable. Assigning an installation to
   a site never moves anything.
4. **Global group addresses stay the only shared address space.** A site
   does not widen group-address uniqueness beyond what `@Global` already
   defines [D].
5. **Reports and diffs** name the site and its member installations; removing
   a site leaves every installation and its content unchanged.

## Alternatives considered

- **Treat several `Building` roots of different installations as one site in
  the UI only.** Rejected: UI-only grouping without a model owner breaks
  save/load and diff, and would hide the ownership question.
- **Merge installations into one so ADR-0038 applies.** Rejected: installation
  IDs carry KNXnet/IP and topology meaning [D, p. 38]; merging changes
  addresses and identities and is not a grouping.
- **Add `Site` as a `Space/@Type`.** Rejected in ADR-0038 already: `SpaceType_t`
  has no such value, and an unknown type stays reported (KL-127).

## Consequences

- MODEL-05 is **blocked**, not open-ended: nothing is implemented, and these
  invariants are the acceptance contract for any later attempt.
- **Lifted when** (any one starts a real design): a real ETS export with more
  than one installation is added to the corpus, showing how ETS scopes
  buildings, global addresses and IDs across them; or a user requirement
  states a KNXBench-native site with the operations it needs (then invariant
  2 applies and ETS compatibility is not claimed).
- Goal B's workbench shows installations as today; it must not present a
  cross-installation site or offer cross-installation moves.
