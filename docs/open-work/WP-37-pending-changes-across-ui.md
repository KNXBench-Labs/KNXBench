# WP-37 — Show pending, not-yet-downloaded changes across the UI (#40)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **step 1 implemented (11 November 2026, main only); step 2 designed in
[ADR-0173](../adr/0173-last-downloaded-parameter-values.md) (Proposed); its
server part implemented (11 November 2026, branch), the UI part open.**

## Packages

1. Show the existing pending flags (parameters, group addresses, address) in
   the device editor header and overview and on the affected sections; clear
   them after a covering download (existing flags, no schema change).
   **Implemented.**
2. Per-parameter “differs from device” marking needs the last downloaded value
   per parameter in the project model: separate package with a native schema
   change and ADR. **Designed** in ADR-0173: a baseline of the chosen values a
   completed download wrote, kept outside undo, compared per effective value
   (native schema 20). WP-40 (version restore, ADR-0173 §6) is implemented.
   **Server part implemented** (see below); **UI part open**: row marks,
   tooltip “Value at last download: …” / “Wert beim letzten Programmieren: …”,
   the panel's count sentence and the unusable-state sentence (ADR-0173 §5).

## Step 2, server part as implemented

- **Baseline:** `knx_core::commissioning::parameter_baseline` (program id,
  resolved values, settlement time), held in `AppState::parameter_baselines`
  next to the project and replaced with it; loaded from and written to the
  native file (schema 20, two `STRICT` tables), never part of `Project`, the
  undo stack, snapshot images or versions.
- **Settlement:** `Settlement::parameter_baseline` is `Keep`, `Remove` or
  `Record`; `apply_settlement` turns it into an edit that the server writes in
  the same SQLite transaction as the journal write of the settled flags and
  only then applies in memory. The resolved values come from the catalogue
  lease that proves the source unchanged. A completed run whose values cannot
  be resolved removes the baseline.
- **Manual mark:** either direction removes the baseline outside the undoable
  command, in the mark's own journal write (written synchronously even on a
  server that journals in the background).
- **Persistence:** a native save prunes devices absent from the saved project;
  Save As copies the set; a project without a file keeps it in memory until
  its first native save; a plain save of another project drops every row.
- **API:** `GET /api/device/{id}/parameters` gains optional `lastDownload`:
  `{state: 'usable', at, differing: [{refId, downloaded, current}]}`, or only a
  state: `absent`, `otherProgram`, `productUnavailable` or `currentUnreadable`
  (two different stored values for one ref; a download refuses the device
  too). The write response (`POST`) does not carry it; the client reads it with
  the next `GET`. TS type `LastDownload` in `apps/knx-web/src/api.ts`.

## Step 1 as implemented

- **Device editor heading:** below the device name a notice line names the
  pending parts in display order, for example “Not programmed yet: Parameters
  and Group addresses” / “Noch nicht programmiert: Parameter und
  Gruppenadressen”. It is amber-tinted, not an error. No line appears when no
  part is pending, nor when the tree does not report the state (absent
  `pending_programming`); the client never guesses.
- **Tab marks:** an amber dot on **Parameters** when parameters or the
  application are pending, and on **Communication objects** when group
  addresses are pending. Each marked tab has a tooltip and an accessible
  description naming its parts (“Not programmed yet: Group addresses”).
  A pending address appears only in the line under the heading.
- **Device overview:** the Inspector's existing **Programming** section in
  Properties already lists all four parts and the unknown state; unchanged.
- **Clearing:** the marks read the editor's current tree. A download or address
  programming whose end may have written reloads the tree
  (`downloadEndChangesProject` / `addressEndChangesProject` →
  `handleTreeUpdate`), so parts the server settled disappear without a separate
  action. Manual **Mark as programmed** takes the same path.
- The heading's existing **Download to device…** button stays the only start
  action. No server, model or pending-flag semantics changed.

**Deviation from the triage wording:** the tab mark is the tab's accessible
*description*, not appended to its *name*. Tab names stay exactly
“Parameters” / “Communication objects”, which keeps keyboard use and the
existing exact-name browser locators stable (including capture specs that run
against imported projects with pending flags); screen readers announce the
description after the name.

## Not covered

- Per-parameter marking in the UI (step 2, UI part); per-link marking.
- Marks inside the parameter panel or on individual communication objects.

## Evidence

- `apps/knx-web/src/DeviceWorkspace.test.tsx`: part order, unknown/clean show
  nothing, tab marks per part, unchanged tab names and keyboard navigation,
  clearing on a reloaded tree, German wording.
- `apps/knx-web/e2e/pending-programming.e2e.ts`: en/de at 1440 and 400 px,
  heading line, tab descriptions, no sideways overflow; clearing after the tree
  reload at 1440 px.

Step 2, server part (synthetic data only, no bus):

- `crates/knx-core/src/commissioning/parameter_baseline.rs`: comparison rules
  (exact raw strings, one side only differs, both absent not compared).
- `crates/knx-app/src/programming_state/tests.rs`: every settlement row of
  ADR-0173 §3 for `settle_download`/`settle_address` and the edit
  `apply_settlement` hands back.
- `crates/knx-store/tests/it/parameter_baseline.rs` and the snapshot codec unit
  tests: 19 → 20 migration and read-only open, future-schema refusal, roundtrip,
  a failed baseline write rolling back the settled flags, identical history and
  version image bytes with and without baselines, pruning, an older stored
  version restoring, 500 devices × 200 refs.
- `apps/knx-server/src/programming_state/source_freshness_tests/parameter_baseline.rs`:
  the real download and address settlers over a synthetic catalogue (record,
  record during an edit, changed source, interrupted, partial, nothing written),
  persistence with the flags, Save As, session-only, prune on save, comparison
  states. `device_system2/tests/settlement.rs`: System-2 removes.
- `apps/knx-server/tests/it/http_parameter_baseline.rs`: `lastDownload` over
  HTTP for each state and the manual mark in both directions, durable.

