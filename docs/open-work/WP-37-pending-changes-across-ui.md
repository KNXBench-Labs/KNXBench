# WP-37 — Show pending, not-yet-downloaded changes across the UI (#40)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **step 1 implemented (11 November 2026, main only); step 2 open.**

## Packages

1. Show the existing pending flags (parameters, group addresses, address) in
   the device editor header and overview and on the affected sections; clear
   them after a covering download (existing flags, no schema change).
   **Implemented.**
2. Per-parameter “differs from device” marking needs the last downloaded value
   per parameter in the project model: separate package with a native schema
   change and ADR. **Open; not started.**

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
  A pending address appears only in the heading line, beside the address.
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

- Per-parameter or per-link marking (step 2).
- Marks inside the parameter panel or on individual communication objects.

## Evidence

- `apps/knx-web/src/DeviceWorkspace.test.tsx`: part order, unknown/clean show
  nothing, tab marks per part, unchanged tab names and keyboard navigation,
  clearing on a reloaded tree, German wording.
- `apps/knx-web/e2e/pending-programming.e2e.ts`: en/de at 1440 and 400 px,
  heading line, tab descriptions, no sideways overflow; clearing after the tree
  reload at 1440 px.
