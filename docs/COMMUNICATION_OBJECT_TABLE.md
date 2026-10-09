# Communication-object table

## Scope and status

Owner-approved Q1–Q12 design, 2026-10-09. UI-only implementation in the single-device
editor. The owner subsequently authorized commit/main integration/push; release
and deployment remain separate. The [main-delivery receipt](evidence/communication-objects-publication-2026-10-09.json)
records the actual reconciled source, acceptance and publication boundary.
[ADR-0102](adr/0102-communication-object-table-view-state.md) records the UI
lifetime boundary. Final aggregate acceptance is recorded in the
[receipt](evidence/communication-objects-2026-10-09.json).

## Presentation and derived data

The table shows Number, Name, Function, effective DPT, Group addresses and evaluated
Status. Default grouping follows opaque evaluated channel keys and program order;
flat view adds Channel. Channel text/name/number remain verbatim facts as in
ADR-0052; no ownership is guessed from labels. Inactive/undetermined/unevaluated
objects retain the unassigned group. All states are initially included; future
states are explicitly unknown, never treated as inactive. Stored `is_active`
and evaluated activation remain distinct. Effective DPT is `dpt ?? program_dpt`;
program defaults, translated text/fallback and source provenance stay disclosed.
Missing-target links are visible by ID and count as links, not as unlinked objects.

## Sorting and filters

Original order is the default and a separate reset. Column clicks select ascending
or descending sorting, within each channel or globally in flat view. Numbers and
recognized canonical DPT components compare numerically, text naturally in the UI
locale. Missing values remain last in either direction, ties retain source order.
The smallest resolved linked GA is the GA sort key; no underlying links are sorted.
Recognized DPTs use their numeric family/subtype, unrecognized strings deterministic
natural fallback. Status order is Active, Inactive, Undetermined, NotEvaluated,
then unknown; this is not severity, bus capability or a stored-state ranking.

Case-insensitive substring search covers number, name, function, description,
channel text/name/number, effective DPT ID/text, every linked GA name/address.
Slash and dotted GA spellings are searchable; no regex/query language is added.
Description-only matches disclose the description. Search AND status AND link
AND DPT filters apply together. DPT filters offer family, exact raw subtype/type
and missing. Unknown raw types remain selectable without invented membership.
A selected type disappearing after a refresh remains selected as a zero-result
filter, not silently changed to All.

Counts describe matches/total, independent of expanded rows and editing exceptions.
Each filter change initially opens matching channels, hides zero-result channels
unless an editor is still open, and permits manual collapse. Reset restores the
unfiltered expansion set; reapplying even an identical filter opens matches again.
Reset filters leaves view/sort unchanged. Empty device and no matches are distinct.

## Editor and request lifetime

Every real object owns a keyed component under the same `tbody` in grouped and
flat views. Header rows are siblings, never parents of object components. Each
component returns summary and editor rows; an editor mounts on first expansion
and stays mounted when subsequently closed. This preserves drafts, local refusal
messages, selected link targets and pending actions through filtering, sorting,
view/channel changes and same-device detail refreshes. Dirty DPT/description
drafts are not overwritten by a refreshed committed value.

Open nonmatching objects remain visibly marked editing exceptions until closed;
they never inflate match counts. Open editors also remain visible when their
channel is collapsed. Removing a real object unmounts its editor, prunes its open
ID and announces the removal. Device changes reset the keyed table; the central
workspace is additionally keyed by project load epoch. Leaving the editor resets
all transient state. Tabs/refresh of the same editor retain view state; there is
no project-file/local-storage/per-device cache.

Existing save gestures remain: DPT/description save on blur/Enter; flags/links
save immediately. Focusing a filter from a dirty edit field can trigger that
existing blur-save; filtering itself does not synthesize a command. One per-object
in-flight admission disables the editor fields, refuses duplicate starts and
withholds delayed success publication after unmount/removal/navigation. Closing
while pending is held open so the result/refusal stays inspectable. This UI guard
is not a new server-side concurrency/confirmation protocol, and does not undo a
request already admitted by the server.

## Layout and boundaries

Translated DE/EN headings and named filters, keyboard-operated sort buttons,
accessible sort direction and expanders, local horizontal/vertical table scrolling
and sticky headings retain every column at narrow widths. Controls wrap without
whole-page overflow; long values remain reachable. Existing editable fields and
GA drag/drop reuse the existing API/application/core commands.

No new dependency, core/storage/API schema, protocol behavior, export, bulk edit,
project-wide CO explorer, resizable/reorderable columns or persistent preferences.
Native WebKitGTK/Orca, full accessibility certification, real bus and full ETS
compatibility are not established by Chromium or synthetic evidence.

## Acceptance

Pure derivation tests and component tests cover opaque channels, source/numeric
sorts, stable ties, all-address search, effective/unknown/missing DPTs, dangling
links, filter reveal/reset, stable editor DOM/drafts/refusals/pending, removal and
device-bound delayed results. Existing editor/flag/paired-link/drag-drop and
keyboard tests retain their behavioral assertions with table-based selectors.

Production-app intercepted Chromium scenarios cover all four built-in themes,
DE/EN, 1440/400 px, headings/alignment, settled-button contrast (waiting for the
actual CSS animation to finish), keyboard expansion/sort, combined
filters, exactly zero view-only mutation requests and unchanged project snapshots,
refused and successful edits under filtering. A synthetic 1,000-object device
records one render/flat/filter/sort sample as browser-driver action-to-visible
assertion wall time. This includes driver overhead and initial layout, not isolated
JS time or a latency guarantee; all 1,000 editing forms are not expanded.
A separate real freshly built server serves the production frontend with a copy
of the fictional 32-device home demo and a fresh scratch catalogue. It opens via
the real picker, exercises headings/search/sort/editor expansion without API
interception and checks zero view mutation requests, an unchanged project snapshot
and original-demo digest. HOME/XDG data/config/state and the project directory are
isolated; the network has only lo. Initial harness refusals/exposure are disclosed
separately in the receipt and known limitations, not admitted as acceptance.

Final frontend/build/type, browser, fresh checkout-bound repository and documentation
gates plus self-review are required. Actual exits/counts, rejected attempts and
source binding belong to the receipt, not speculative claims in this contract.
