# Project notes editor — isolated host contract

Exact ownership: new `apps/knx-web/src/features/notes/` client/types/text adapter/
editor/copy/CSS module/tests and isolated fixture/build/browser/type config only;
this host contract, own handover/log and later additive status/evidence updates.
Do not edit shared App/navigation/global styles/catalogues/HelpPanel/HelpTip.
No new dependencies. Existing authenticated [HTTP contract](project-note-http.md)
and [application service](project-note-service.md) remain mutation authority.

The host supplies real server/project incarnation and snapshot revision; map
existing ProjectTree snake_case keys explicitly to camelCase. Never invent
revision counters or mount writable controls against missing/stale identity.
Typed target choices use stable domain IDs and labels from the authoritative
project projection, not physical/group addresses or guessed imported RefIds.
A stored target missing from the host label catalogue stays unchanged and visibly
identified, never silently retargeted. Host refreshes authoritative state on a
confirmed result and integrates local-draft dirtiness into navigation policy.

UI is an ordered engineering notebook: list beside title/body/target and explicit
per-note documentation opt-in; review and confirmed Apply are separate gestures.
Literal UTF-8 text, no HTML/Markdown/resource fetching. Preserve unchanged text,
including CR/LF spelling, through a pure editor text adapter. Filtering/selection
never mutates source order. Switching notes/new draft requires explicit discard
of an existing dirty draft. Draft edits revoke plans; no implicit apply on close.

Same-project revision changes invalidate preview and re-read. If the ordered
notes are unchanged, a dirty draft moves onto the new revision and stays editable
(Save bumps the revision); otherwise it is retained and requires deliberate reload
(amended 2026-10-12). Project/server change and unmount cannot inherit late
read/preview/apply responses; stale previews are cancelled. Client cancellation
cannot retract a request already committed by the server. Successful mutation
and failed host refresh are separate outcomes; unknown transport/response outcome
requires authoritative refresh, not a silent retry or a false rollback claim.
Native working-history recovery is not implicit root Save; session-only is clear.

EN/DE feature copy is local. Reuse the existing application-help event and
HelpPanel (`workbench` topic), not a duplicate help database. Reuse existing
HelpTip for the group-range concept when relevant; specific note guidance remains
static feature prose, never authored project data. Adding a dedicated notes help
topic or shared-shell entry belongs to Session1, not this package.

Implemented bounded leaf: closed client admission, literal textarea adapter,
preview-first component, local EN/DE copy and scoped KNX theme-token styling,
existing workbench help event/GroupRange HelpTip, typed isolated fixture/build.
Current source has 40 feature tests and 22 actual owner stages, including whole
web 2836/0/182 files, production/fixture builds, inherited server/core/Rust,
strict Clippy and nonempty explicit-worktree repository gates. Three source guard
negatives catch double Apply, implicit preview mutation and line-ending loss;
canonical source is restored. Review is in-session self-review, not independent.

Fresh built loopback-only browser8 covers EN/DE 400/1440 Graphite/Porcelain,
existing-help event, literal display, no unexpected requests/overflow and no
hover scale/shadow. All eight final RGBA rasters equal individually inspected
prior frames. Five real confirmed browser Create/Replace/Move/Remove/opt-in
changes preserve literal body and private draft/preview; working-history restart,
Undo/Redo and non-note/retained-table digests pass. Root Save returned empty
success; the initial harness wrongly parsed JSON and remains a failed partial
attempt. Read-only SQL verifies the actual saved payload. A follow-up copies that
saved result byte-exactly and proves final backend-restart recovery without
replaying mutations or Save. Actual HTTP Save preserves recovery stacks; do not
infer the policy from lower-level `knx_store::save`. All owned listeners close;
synthetic seed and copied-from saved artifact remain unchanged.

The final source changed only German expiry address and removal of literal CSS
fallbacks after the five-mutation browser run. Those mutations bind the earlier
source manifest; current feature/broad gates and fresh browser8 bind final source.
No final-source five-mutation rerun or expiry-frame inspection is claimed.
[Guidance and limits](project-note-editor-guidance.md) describe the isolated
component; the production host is described below. A fixture is not production
workbench/Tauri/Orca/hardware or ETS compatibility.

## Workbench host (12 October 2026)

`apps/knx-web/src/NotesWorkspace.tsx` mounts the editor as the **Notes** view
(navigation and command palette, disabled without a project). Pure
`notesHost.ts` maps `server_incarnation`/`project_incarnation`/`snapshot_revision`
explicitly to the editor context and refuses incomplete identity (no editor, no
read). Target choices are installations, areas, lines, building parts, devices,
group ranges and group addresses by projection domain ID, deduplicated, with
ambiguous IDs withheld and equal labels disambiguated by `#id`; communication
objects are not offered because the tree does not project all of them. After a
confirmed Apply the host re-reads `GET /api/project` through the workbench's own
snapshot publication; a failed re-read stays "confirmed, refresh failed".
The view mounts on first visit, stays mounted (hidden) for the same
server/project incarnation and remounts on identity change, so projects that never
open it issue no notes request. A dirty draft guards Open/Import, New project,
Quit and the desktop window close with a two-answer dialog; workbench navigation
keeps the draft. The leaf additionally scrolls a refusal alert into view and resets
the global control height on its opt-in checkbox. Evidence: App/host/editor unit
tests, 11 intercepted production-build Chromium cases (EN/DE, Porcelain/Graphite,
1440/400) and one loopback-only real-server smoke; see
[IMPLEMENTATION_STATUS](../IMPLEMENTATION_STATUS.md). Self-review only.
