# Device-download path audit — 15 October 2026

## Request and boundary

Owner asked which KNX device download paths still require implementation, then
explicitly requested original specification PDFs, not the extract. Read-only
capability research; no implementation or live-operation authorisation.

## Work performed

- Read the selected root handover first; freshly fetched origin/main and inspected
  current implementation/status via Git objects, preserving the stale dirty root.
- Baseline `3a9fd92f8fff211a3922d095c2888a1216b53170`; selected root HEAD
  `1817fb2ab3d6cae503433954658f8a54626d9151` unchanged.
- Rendered original local Configuration Procedures, Management Procedures,
  Resources, Profiles and modern RF-profile pages using pdftoppm. Visually read
  relevant bodies, matrices and footers; no knowledge-base extract or pdftotext
  used as normative evidence. This is not a complete specification audit.
- Cross-checked actual 070nh/ProductProcedure admission, simulator-only System-B
  and RF substrate, explicit System-2 prerequisites, Secure/PL110/access-key limits.
- Added `docs/research/2026-10-15-device-download-paths.md`: family/procedure
  matrix, precise inspected PDF references, current source boundary and explicitly
  labelled prioritisation recommendation. No ledger/task status changed.
- A supplementary official-site search returned a Data Secure excerpt, but
  full-page extraction failed (configured search-only backend). Neither is used
  as normative evidence; the audit relies on original PDFs and repository code.

## Owner clarification: device-free implementation

Owner emphasised finding how to implement the new paths without access to the
devices. Inspected the actual simulator controls, ManagementTransport,
DeviceBackup exclusions and System2 source/payload fixtures. Additionally rendered
original `08_03_07 System Conformance Testing - AIL and Management Tests
v01.07.04 AS.pdf`: contents3–6 and actual memory/property test bodies32/58.
The dossier now includes seven concrete offline deliverables and per-family next
slices. Reuse the real executor against independently specified simulated device
state; add independent bytes, persistence/failure recovery and no-connector gates.
Device/server test vectors inform tests but do not certify a client or a device.
Firmware/hardware compatibility remains a separate target-specific verification,
not a prerequisite for all codec/planner/simulator development. No new source
implementation or task assignment is implied.

## Findings

Existing usable path is bounded BIM M112/070nh, not universal memory programming.
Open families/configurations include System2, SystemB end-to-end integration,
BCU1, System300, non070nh System7, couplers and RF. Secure is cross-cutting;
PL110/Easy/local USB configuration remain separate scopes. Property load control
can coexist with memory-written payloads. A parsed TaskCtrl2 record is not a
complete System2 sequencer or recoverable hardware download. CP's current
contents/body omit historical sections3.2/3.3; no invented algorithm was inferred.

## Verification and non-actions

Documentation whitespace and local-reference/source-anchor checks are recorded
at task closure. No runtime code changed, so no build or feature tests were run.
No bus access, credentials, firmware execution, device promotion, commit, push,
release, deployment, subagents or unrelated root synchronisation.
