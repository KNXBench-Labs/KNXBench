# UI feedback work packages — 18 October 2026

**Status (19 October 2026): delivered.** UI-01–UI-06 and the whole-set
corrections are on `main`; see *Delivery record* below. The next paragraph is
the original planning introduction and is kept as written.

Planning only, based on eight retained items from the owner's nine numbered
requests. Item 3 (“Update We”) was withdrawn by the owner as a typo on
18 October 2026 and creates no work package. No package is
claimed, no implementation is authorised by this document, and no product
behaviour has changed. Recommendations below are proposed acceptance criteria,
not a record of decisions already approved by the owner.

## Owner go and decisions (18 October 2026, later the same day)

The owner pointed the next session at these packages ("there are tasks in
OPEN_WORK, use multiple tasks and subagents"). That is the start signal this
plan asked for. Open behavioural choices take the recommendations written
below; nothing beyond them was decided:

- **UI-01:** a captured telegram keeps the interpretation of the session it was
  captured in (existing session contract); no re-interpretation after an edit
  without an explicit refresh.
- **UI-02:** the routine "Source: device instance" label goes; the metadata stays
  in model, API, import/export and technical diagnostics.
- **UI-03:** History, Help and Import / Export become submenus; frequent project
  actions stay directly in File.
- **UI-04:** decimal MB (`bytes / 1,000,000`, one decimal). The product database
  figure is its size on disk: main file plus `-wal`/`-shm` sidecars when present,
  labelled as such, measured without a checkpoint. A never-saved project shows
  "not saved yet", not 0 MB.
- **UI-05:** widths are browser-local UI settings keyed by table ID and stable
  column ID, validated on load, with a reset.
- **UI-06:** labels follow the measured meaning of each number.

Execution: one claim for the set, one worktree, branch and gate per package,
delivered one after the other. Implementation is done by coding-agent workers
pinned to a model per package; the controlling session reviews, gates, commits
and pushes. Wave 1 runs in parallel (UI-01, UI-02, UI-04), wave 2 starts after
UI-02 is on `main` (UI-03, UI-05), then UI-06. Excluded models: the Fable and
Astra families.

| ID | Worker | Model | Independent review |
| --- | --- | --- | --- |
| UI-01 | Codex CLI | `gpt-6.1-sol`, high | Claude Code, `claude-opus-5-5` |
| UI-02 | Claude Code | `claude-opus-5-5`, high | Codex CLI, `gpt-5.6-sol`, high |
| UI-03 | Claude Code | `claude-sonnet-5-5`, high | Codex CLI, `gpt-5.6-sol`, medium |
| UI-04 | Codex CLI | `gpt-6-sol`, high | Claude Code, `claude-opus-5-5` |
| UI-05 | Codex CLI | `gpt-6.1-sol`, high | Claude Code, `claude-opus-5-5` |
| UI-06 | Claude Code | `claude-sonnet-5-5`, high | Codex CLI, `gpt-5.6-sol`, medium |

Whole-set review after UI-06: Codex CLI, `gpt-6.1-sol`, xhigh.

State (18 October, evening; historical): UI-01 and UI-04 delivered (merge `fd1f69ee`, full
gate green); UI-02 and UI-05 accepted after review rounds; UI-03 and UI-06 in
review. Waves overlapped: UI-03 and UI-05 started on the UI-02 branch and
UI-06 on its first fix round, so no package waited for a gate.

## Delivery record (18–19 October 2026)

- **Wave 1:** UI-01 and UI-04, merge `fd1f69ee`, full gate green.
- **Wave 3:** UI-02, UI-03 and UI-06, published as `bc9d73c1` after the frozen
  16-stage gate on `471ca75a`.
- **UI-05:** nine correction rounds (GPT-6.1 high), reviews by Claude Opus 5.5
  and GPT-5.6; committed `0531ce32`, the web gate gained the
  `production-table-columns` stage.
- **Whole-set review** (GPT-6.1 xhigh) rejected the first integration with
  UISET-01 (tables ignored application zoom), UISET-02 (File submenus left the
  window above 650 px), UISET-03 (disabled toolbar buttons painted over the
  open menu; already on `main`), UISET-04 (stale status) and UISET-05 (overlay
  test not paint-sensitive). The first full gate on the integration also found
  11 px row actions from UI-05 (below the UI-02 24 px floor).
- **Correction rounds:** zoom-aware tables (GPT-6.1 high, reviewed by Claude
  Opus 5.5); File menu in four rounds (Claude Sonnet 5.5, then Claude Opus 5.5,
  each reviewed by GPT-6.1 xhigh): one shared 220 px minimum with an inline
  fallback, a stacking level, inline submenus that ignore hover, a press guard
  for mouse, touch and pen, placement in layout pixels under zoom, and a measured
  first-level fit, so Logout stays reachable at 150% in a 601 px high window.
- **Final:** the last review accepted without findings; the delivery merge onto
  `main` passed the full 16-stage gate (see IMPLEMENTATION_STATUS).
- **Follow-ups (unowned):** pane drags under zoom and a table zoom case
  through the scale setting ([open work](../OPEN_WORK.md#ui-feedback-packages-18-october-2026)).

## Checked starting point

Read the handover, architecture, implementation index and OPEN_WORK before
planning. Inspected checkout: `5c040933`; the locally recorded `origin/main`
was `3c3bce0b`. This is a local source inspection, not a fresh remote audit or
an inspection of the running application. Re-check published source and claims
before implementation.

- `apps/knx-web/src/App.tsx` already has File menu sections for Project,
  Exchange, History and Help. Existing actions, disabled states, native/web
  differences and focus handling must survive a submenu conversion.
- `apps/knx-web/src/BusMonitorPanel.tsx` renders structured decoded telegram
  results. `apps/knx-server/src/bus.rs` already uses
  `knx_core::resolve_project_group_address_dpts`. Trace this path before
  introducing another resolver or changing display text.
- The reported visual defects and the owner's specific DPT case have **not**
  been reproduced. A missing type, conflicting types, an unsupported codec,
  a decoding failure and stale project/session context are different outcomes.
- The census-3 product-database/size-limit claim is still reserved in the
  inspected handover. These UI packages do not take over that work.

## Package map

| ID | Package | Owner request | Priority | Dependency |
| --- | --- | --- | --- | --- |
| UI-01 | Bus monitor: investigate and correct DPT resolution | 8 | P1 correctness | independent; inspect shared resolver and session contract |
| UI-02 | Compact controls, visible navigation separators, less UI noise | 5, 6, 7 | P2 foundation | define shared styling before UI-03 and UI-06 |
| UI-03 | File menu with animated submenus | 1 | P2 usability | UI-02 styling decisions |
| UI-04 | Project and product-database status metrics | 2 | P2 transparency | agree metric meanings; respect active storage claim |
| UI-05 | User-resizable table columns | 4 | P2 usability | inventory and shared column contract first |
| UI-06 | Topology: labelled metrics and bounded layout | 9 | P2 clarity | UI-02; UI-05 if the affected surface is a table |


Recommended implementation order: **UI-01 → UI-02 → UI-03 → UI-04 →
UI-05 → UI-06**. UI-01 and
UI-04 do not technically depend on the styling work; the order is a delivery
recommendation, not a reason to couple their commits. Shared shell, CSS,
translations and browser fixtures require coordination between sessions.

## UI-01 — Bus monitor DPT correctness

**Outcome:** a telegram with an unambiguous supported project DPT does not
incorrectly say “No DPT assigned”. A genuine missing or conflicting assignment
remains explicit; never guess a DPT from a caption such as “brightness”.

Scope:

1. Reproduce the owner's source-device/group-address case using an authorised
   read-only project snapshot and a retained or synthetic telegram. Keep private
   project data out of public fixtures and notes.
2. Trace group-address links, configured communication objects, effective DPTs,
   the existing shared resolver, monitor session context, server projection and
   UI rendering. Determine the cause before choosing the owning layer to edit.
3. Fix the actual omission or misleading state; do not merely replace the text
   or hard-code the reported device/address. Reuse shared resolution rules.
4. Define expected behaviour after a DPT/link edit or project switch using the
   existing session contract. A captured historical telegram must not silently
   acquire a different interpretation without an explicit context policy.

Acceptance/tests:

- A failing regression reproduces the observed cause before the fix.
- Cover direct assignment and linked-object resolution, compatible/conflicting
  candidates, no project, missing type, unsupported type, malformed payload and
  stale context. Preserve raw telegram bytes and the distinction between states.
- Core/application or HTTP tests at the owning layer plus the monitor UI test;
  exercise the actual production path, not only a mocked label.
- No physical bus traffic or device writes are required by this package.

## UI-02 — Compact, distinctive controls and visible navigation

**Outcome:** controls feel like one professional application, navigation
boundaries are visible without hover, and routine views omit unnecessary
“Source: device instance” metadata.

Scope:

- Audit existing button variants, density, spacing, focus and motion settings.
  Reuse their owner rather than adding unrelated per-screen overrides.
- Establish a small consistent set of primary, secondary, quiet/icon and
  destructive treatments. Compact does not mean tiny click targets or
  unrecognisable destructive actions.
- Add restrained hover/pressed feedback without moving surrounding layout.
  Respect the application's motion settings and reduced-motion preference.
- Identify both navigation section dividers and resize handles; keep the
  appropriate separators visible at rest in light and dark mode. Preserve
  pointer and keyboard resizing, collapse state and focus feedback.
- Remove the identified source label from routine device/object presentation.
  Preserve source metadata in the model, APIs, import/export and deliberately
  opened technical diagnostics. This is presentation cleanup, not data deletion.

Acceptance/tests:

- Check normal, hover, pressed, keyboard-focus, disabled and pending states.
- Visually review representative toolbar, dialog, table and inspector controls
  in both themes and at narrow widths; no clipped labels or spacing jumps.
- Test separator visibility without hover and the first resize after unfolding.
- Assert absence of the routine source label and retention of the underlying
  metadata. Keep other useful source/error diagnostics intact.

## UI-03 — File menu with animated submenus

**Outcome:** File offers **History**, **Help**, and **Import / Export** as
submenus, normally opening to the right with a short, subtle animation.

Scope:

- Move the existing grouped actions into submenus; do not implement parallel
  versions of history, import, export or help.
- Keep frequent project actions directly accessible. Preserve native/web
  distinctions, action availability, unsaved-change guards and modal lifecycle.
- Support pointer and keyboard navigation, focus restoration, Escape and
  outside-click closure. Moving from the parent item to its submenu must not
  unexpectedly close it; opening one submenu closes its sibling.
- Handle viewport edges: flip or reposition when the right side has no room;
  use a contained narrow-screen alternative rather than off-screen flyouts.
- Respect reduced motion and existing animation settings.

Acceptance/tests: all existing commands still reach their original handlers;
no accidental double activation; correct disabled states; keyboard-only flow;
light/dark mode; normal and narrow widths; motion disabled. Cover component
behaviour and real-browser positioning/focus, not screenshots alone.

## UI-04 — Project and product-database status metrics

**Outcome:** project status shows meaningful storage sizes and explicitly
labelled counts, not just technical health messages.

Proposed metrics:

- Saved project file size in **MB** and whether it refers to the last saved
  file; show unsaved changes separately. A new unsaved project has no measured
  saved-file size, not an invented zero or estimated current save size.
- Active product database size in **MB**, clearly separate from the size of
  source archives or all databases on the machine.
- Project device instances, group addresses, areas and lines, and communication
  objects where available. Distinguish these from catalogue products,
  manufacturers and application programs. Count canonical entities once.
- Product catalogue counts available from its existing storage layer. A
  catalogue product is not a physical device in the project.

Implementation boundary:

- Inventory current status DTOs/read routes first. Measure storage metadata on
  the server, not from browser upload length or generated frontend JSON.
- Define decimal MB (`bytes / 1,000,000`), display precision and database scope.
  For SQLite, decide whether sidecar sizes are separately reported or included;
  label the choice, avoid inconsistent reads and never checkpoint just to
  update a status counter. An ADR is needed only if this changes an existing
  architectural contract.
- Return explicit unavailable/loading/error states. Do not expose private
  absolute paths. Avoid expensive per-row database queries or repeated full
  serialisation on ordinary UI updates.
- Coordinate with the active size-limit/storage claim; this package measures
  and displays data, it does not alter limits, schemas or growth policy.

Acceptance/tests: known byte fixtures and count oracles; new/unsaved, saved and
modified projects; save/save-as, failed save, import, project switch and
catalogue install; unavailable database and empty catalogue. HTTP/UI tests
must verify which resource and snapshot each number describes.

## UI-05 — Resizable columns across data tables

**Outcome:** users can adjust data-table column widths consistently instead of
being locked into the initial layout.

Scope:

1. Inventory actual data tables (including monitors, objects, devices,
   addresses, catalogue and reports where present); list coverage explicitly.
   Trees, cards and forms are not automatically tables. Manufacturer-declared
   parameter grids retain their source layout; do not resize them by accident.
2. Define a reusable presentation-only width mechanism with stable table/column
   identities, sensible minima, pointer and keyboard operation, and reset.
3. Proposed persistence: browser-local UI settings keyed by table and stable
   column IDs, not translated headings or project content. Validate stored
   values and fall back safely when columns change.
4. Convert tables in small reviewed slices within this package. Resizing must
   not trigger sorting, drag/drop or row selection, and must not rewrite data.
5. Keep horizontal scrolling local to the table when columns exceed the space.

Acceptance/tests: pointer and keyboard resizing, minimum widths, reset, reload,
changed columns, corrupt preferences, multiple table instances, sorting,
virtualised rows where present and narrow-screen overflow. Finish with a
coverage checklist; do not claim “all tables” after converting only one.

## UI-06 — Topology metrics and widescreen layout

**Outcome:** the right-hand numbers explain themselves and remain visually
close enough to the entities they describe, even on widescreens.

Scope:

- Reproduce the reported page and identify what each number actually means
  from its projection and selector before assigning a label. Do not assume it
  is a device count, address or communication-object number.
- Add visible headings or inline labels with units/meaning; a hover tooltip
  alone does not solve an unexplained number.
- Bound the relevant content/metric region or its grid columns instead of
  stretching the name-to-number gap across the whole viewport. Preserve
  hierarchy, selection and reasonable room for long names/addresses.
- If this surface is a data table, use UI-05 rather than inventing a second
  column-sizing system; otherwise keep the layout change local to the view.

Acceptance/tests: verify metric meaning against a small known topology; empty
and populated lines, long labels and large counts; visual checks on normal,
wide and narrow viewports in both themes. Keep navigation and inspector
behaviour unchanged.


## Shared delivery requirements

Before implementation, re-read fresh repository state and active claims;
confirm package scope with the owner, resolve open behavioural decisions and
claim only the package being started. A plan is not a start signal.

For each delivered package: focused change and commits, relevant regression
and integration tests, real-browser review for visual work, EN/DE copy,
keyboard access, both themes, narrow/normal/wide layouts and motion settings.
Use the established repository gates and preserve their full scope. Update
manuals, implementation status, limitations and the handover when behaviour
actually changes; do not mark a planned feature as implemented.
