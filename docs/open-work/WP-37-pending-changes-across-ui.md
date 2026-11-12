# WP-37 — Show pending, not-yet-downloaded changes across the UI (#40)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **step 1 implemented (11 November 2026, main only); step 2 implemented
(server part 11 November, parameter-editor UI 12 November 2026, main only) per
[ADR-0173](../adr/0173-last-downloaded-parameter-values.md) (Accepted). No
hardware run.**

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
   **Implemented**: server part and parameter-editor UI (see below).
3. Per-link and per-object-flag marks in the communication-object view.
   **Designed** (12 November 2026) in
   [ADR-0174](../adr/0174-last-downloaded-links-and-flags.md) (Proposed): the
   effective object flags join the parameter baseline (they are written with
   Parameters), the written links get their own baseline section following
   the Group addresses part, same lifecycle as ADR-0173, native schema 21.
   Migration preserves existing parameter values and explicitly distinguishes
   unknown flags from known empty flags; links and flags compare with separate
   states. The shared effective-flag resolver prerequisite is implemented in
   `knx_productdb::image::resolve_effective_object_flags` (WP-37e): checked
   activation and flag precedence are shared with real image construction;
   duplicate active numbers refuse before mapping. The image's invisible
   Background-Assign consistency check is shared as well, so an assignment
   that would change a consulted controller cannot yield a known-empty set.
   It returns only the five
   written flags, with no size/priority image-admission gates. Read-on-init
   remains a separate request for the builder's existing loss/refusal handling,
   not a written sixth flag. This adds no baseline storage, settlement,
   comparison response or UI. Server part and UI part follow as separate
   packages. A first draft that
   put flags with the links led to the ADR-0129 amendment of the same day
   (flag edits mark Parameters).

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
  server that journals in the background); a device without a baseline keeps
  the background path.
- **Persistence:** a native save prunes devices absent from the saved project;
  Save As copies the set; a project without a file keeps it in memory until
  its first native save; a plain server save writes no baseline row; a plain
  save of another project drops every row, and a store save not admitted at
  the journal's generation (`knx import --replace`) keeps only rows whose device
  has the same source, program and product reference. A version restore keeps
  only rows whose restored device has the live device's identity within the same
  project id; the server then holds exactly the set the file kept.
- **API:** `GET /api/device/{id}/parameters` gains optional `lastDownload`:
  `{state: 'usable', at, differing: [{refId, downloaded, current}]}`, or only a
  state: `absent`, `otherProgram`, `productUnavailable` or `currentUnreadable`
  (two different stored values for one ref; a download refuses the device
  too). The write response (`POST`) does not carry it; the client reads it with
  the next `GET`. TS type `LastDownload` in `apps/knx-web/src/api.ts`.

## Step 2, UI part as implemented

- **Row mark:** in the Parameters tab (and the Manufacturer fields tab) an unscoped field
  whose `etsId` is a `differing` ref of a usable `lastDownload` gets the step-1
  amber dot after its caption (own class `parameter-download-mark`). Its tooltip
  and the control's accessible description say “Value at last download: …” /
  “Wert beim letzten Programmieren: …”; a selection shows the declared option's
  label, an undeclared value or a free value its raw text, a missing one “no
  value” / “kein Wert”. The dot is `aria-hidden`, so the field's accessible name
  is unchanged. The description is a caption after the row's label (never part
  of its name), always shown under the row and beside an open checked editor:
  visible for keyboard and touch users, and no layout shift when focus moves
  (a focus-only caption could move a button away under the pointer between
  press and release; re-review 12 November 2026). While a kept comparison
  waits for the next read, the line carries `aria-busy`; if the written row was
  the only differing one, the line is left out until the read decides. Under `forced-colors: active` the row dot, the summary dot and
  the step-1 dot (`device-pending-dot`) keep `forced-color-adjust: none` and paint
  in `CanvasText`. `refId` is the `ParameterRef` id, which is the unscoped field's
  `etsId` (`domain.rs` sets `ets_id: view.id`; baselines hold unscoped refs
  only, `resolve_unscoped_values`); module-scoped rows are never marked.
- **Summary line** (Parameters view only, above the fields, static text, not a
  live region): “{count} values differ from the last download ({time}).” with
  singular/plural, plus “{hidden} of them are not shown with the current
  settings.” for differing refs no unscoped value field of the panel presents
  (inactive under the current settings, or undeclared), and “{count} of them are
  in the Manufacturer fields tab.” for refs only manufacturer-restricted fields
  present (same `isManufacturerRestricted` predicate as the view filter), both
  counted against the unfiltered sections; “No value differs from the last download ({time}).”
  when none; one plain sentence for `absent`, `otherProgram`,
  `productUnavailable` and `currentUnreadable`, and no row marks. No line when
  the response has no `lastDownload` (older server) or the program is not
  resolved. The time uses the app's log-time helper (`formatLogTime`: time only
  for today, date and time otherwise, 24-hour clock).
- **Write responses:** a parameter `POST` response carries no `lastDownload`;
  `useDeviceParameters` keeps the shown comparison across an accepted write for
  the same device, language, project lifetime and program until the follow-up
  `GET` replaces it, so the other marks do not blink off. The entries of the
  written field's `etsId`/`writeEtsId` are dropped (that row's state is unknown,
  not compared), the count follows the remaining list and the line is dimmed
  (`data-refreshing`) until the `GET`. The client never compares values itself.
  A write answer for a previously selected device is refused by the read model
  and never reaches the new device's comparison.

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

- Per-link marks and marks on individual communication objects or flags:
  designed in ADR-0174, not implemented.
- Module-scoped parameters (no baseline until module downloads exist); a write
  of a module-scoped row never drops the unscoped ref's mark.
- The caption beside an open checked editor is visible but not linked to the
  editor's controls as their description.
- Devices imported, programmed by the System-2 path or marked by hand show only
  the pending marks until their next completed regular download.
- Device identity is source, product and program reference within one project
  id. New projects get a random id (`P-KB-` and 128 bits, main 12 November
  2026; `domain.rs` `new_project_id`, test `every_new_project_gets_its_own_project_id`).
  Files created before keep `P-0001`, ETS ids (`P-0512`) can repeat across
  unrelated installations, and copies of one project (Save As, a copied file)
  keep one id once edited apart: such projects still cannot be told apart, so a
  version left in a file overwritten by Save As can pair a baseline with another
  physical device of the same product and program.

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
  version image bytes with and without baselines, pruning (by device identity
  for an unadmitted save, by presence for an admitted one), a plain save
  writing no row (counting triggers), an older stored version restoring, a
  restore keeping only same-identity devices
  (`a_restore_keeps_baselines_only_where_the_version_names_the_same_device`) and
  none for another project's version
  (`a_restore_of_another_projects_version_keeps_no_baseline`), 500 devices × 200
  refs. `apps/knx-server/tests/it/http_project_history.rs`
  `a_restore_leaves_the_held_baselines_equal_to_what_the_file_kept`.
- `apps/knx-server/src/programming_state/source_freshness_tests/parameter_baseline.rs`:
  the real download and address settlers over a synthetic catalogue (record,
  record during an edit, changed source, interrupted, partial, nothing written),
  persistence with the flags, a failed persist leaving the held baseline,
  a plain save and reopen leaving it unchanged, Save As, session-only, prune
  on save, comparison states. `device_system2/tests/settlement.rs`: System-2
  removes. `journal_writer_tests/parameter_baseline.rs`: on a background
  journaling server a mark that removes a baseline is in the file before it
  answers; one without a baseline keeps the background path. The link
  adoption test asserts adoption leaves every baseline unchanged.
- `apps/knx-server/tests/it/http_parameter_baseline.rs`: `lastDownload` over
  HTTP for each state and the manual mark in both directions, durable.

Step 2, UI part (intercepted fictional data, no bus):

- `apps/knx-web/src/ParameterPanel.lastDownload.test.tsx` (18): marks and
  descriptions on differing unscoped rows only (the module row exists and is
  unmarked), the caption after the label, enum label, undeclared and missing
  value, singular/plural, the not-shown and Manufacturer-fields counts, no
  difference, each unusable state in English and German, nothing without
  `lastDownload`, line only in the Parameters view, always-shown caption in the
  Manufacturer fields view, retention across a write response without the
  written ref (by `etsId` and by `writeEtsId`) and with the line dimmed,
  `aria-busy` while waiting, no "no value differs" before the read when the
  only differing ref was written, a module-scoped write keeping the unscoped
  mark, replacement by the next read, no carry-over to another program, no retention
  after a device switch with a late write answer, at the row and at the read
  model.
- `apps/knx-web/src/parameterLastDownload.test.ts` (6): the two counts kept
  apart (own and block access, a ref with a restricted and an unrestricted
  field, a ref only a kind `None` field presents, module-only refs), dropping
  written refs, and the forced-colours rule for all three dots.
- `apps/knx-web/src/ParameterPanel.checked.test.tsx` (+1): the downloaded value
  stays beside the open checked editor and goes when its write is accepted.
- `apps/knx-web/e2e/parameter-last-download.e2e.ts` (dev and devices lanes, 7):
  en/de at 1440 and 400 px, summary text with time (browser pinned to the zone
  the expected text is formatted in) and the Manufacturer-fields count, visible
  dots, unchanged accessible names and the exact accessible descriptions, the
  caption always shown and inside the viewport, the checked-editor button not
  moving relative to its input when focus enters the row or the button, no
  sideways overflow, no unexpected request; an unusable state without marks;
  the dots under forced colours.

