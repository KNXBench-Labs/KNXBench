# Goal: owner UI feedback, packages P1–P4

Owner request 2026-10-14 (seven points), settled in a grill-me interview the
same day: the owner accepted every recommendation. This file is the decision
record; sessions work from it, not from chat history.

Order and ownership: **P1 → P2** in the requesting Claude session, **P3** and
**P4** each in a fresh session, strictly one after another (P4 reuses P3's
change tracking; P1/P2 share `styles.css` and the motion settings). Every
package has its own worktree, review, full gate and push. Before starting a
package, fetch `origin/main`, read the newest `.ai/CURRENT_STATE.md` entry and
claim the package there.

| Package | Scope | State |
| --- | --- | --- |
| P1 | Toast entry, resume after reload, overwrite question | delivered 2026-10-14 (see `IMPLEMENTATION_STATUS.md`) |
| P2 | Explorer: visibly expandable nodes, structure lines with spread/pulse | delivered 2026-10-14 (see `IMPLEMENTATION_STATUS.md`) |
| P3 | Changes not yet programmed, per part | delivered 2026-10-15 (source after Alpha 11; [acceptance](../evidence/p3-pending-programming-2026-10-15.json)) |
| P4 | Project file versus real devices: tags and comparison table | package 1 published; package 2 corpus prerequisites corrected, final integrated acceptance pending |

Starter for a fresh session (replace the package number):

```text
Read docs/goals/ui-feedback-p1-p4.md and .ai/CURRENT_STATE.md.
Implement P3 in its own worktree: review, full gate, push.
```

## P1 — toast, resume, overwrite

- **Toast (owner point 4):** every toast enters with its exit animation played
  backwards: the same keyframes, length and curve, run in reverse (CSS also
  reverses the curve). It applies to all kinds, because they share one
  lifecycle. Motion categories, global Off and OS reduced motion keep
  working as before.
- **Resume (point 6):** every web editor mount (a reload, a reopened tab, the
  companion's return) reads the server's open project through the existing
  read-only `GET /api/project` and shows it. An automatic resume announces
  itself, saying so when there are unsaved changes; the companion return
  stays silent. No "Leave page?" prompt: the server keeps the project, and
  only a server restart loses unsaved work (autosave covers that).
  ADR-0127.
- **Overwrite (point 7):** the server refuses Save As onto an existing file
  with `409 fileExists` unless the request sends `overwrite: true`. The
  browser asks in its own dialog after that refusal. The desktop sends the
  confirmation directly, because its native GTK save dialog already asked
  (rfd 0.16 `build_save_file` sets `do_overwrite_confirmation`; verified in
  source for Linux only). Save As onto the open file asks too; plain Save
  never asks. Overwriting keeps the file's previous state in its project
  history, and a file that is not a KNXBench project is refused.

## P2 — navigation and structure lines (points 1 and 2)

- Scope: the project explorer's shared `TreeNode` only. Other trees later, if
  the owner likes it.
- Expandable at a glance: a larger chevron with a hover target that rotates
  when opened, a child count on collapsed nodes (e.g. "Line 1.1 · 14") and
  Expand all / Collapse all in the panel header. Nodes still start open
  (search reveal and drag and drop rely on that).
- Lines: static connector lines always (also with motion off); they grow from
  the parent downwards when a node opens; about every 12 s a soft pulse runs
  from the root through the open branches. Only visible rows animate, and
  nothing animates in a hidden tab (the explorer is not virtualised).
- Switch: a new motion category "Structure lines" in Settings › Motion:
  inherit = grow and pulse, subtle = grow only, off = static lines. Global Off
  and OS reduced motion always win. On by default.
- As built: lines grow only below a node the user opens (click, keyboard,
  Expand all), not on load or on a search reveal, so the eye is already
  there. The pulse marks the rows visible when it starts (`treeLines.ts`) and
  the CSS animates only those; the count is a small pill beside the label.
  Durations derive from `--knx-transition-duration`, as the motion guard asks.

## P3 — changes not yet programmed (point 5)

- Per part, like ETS: Address, Application, Parameters, Group addresses. The
  starting point is the imported ETS `*Loaded` flags; they are already
  persisted (`crates/knx-store/src/devices.rs`), so **no store migration**.
- What marks which part: individual address changed → Address;
  product/application changed → Application; any parameter value →
  Parameters; link/unlink, object flags, renumbering or deleting a linked
  group address → Group addresses, on every linked device. DPT changes do
  not count (the group object table carries the product's object types).
- Marking happens in the same undo step as the edit, so undo clears it again.
- What clears: a full download clears every part, a partial download only
  the parts it wrote, programming the individual address clears Address. A
  download that fails partway marks the parts it touched as pending, even
  ones that were clean.
- Manual "Mark as programmed / not programmed": undoable and logged, for
  devices programmed outside KNXBench.
- Display: a "Programming" column in the device view with up to four small
  chips (Adr · App · Par · GA, pending parts only), a "pending only" filter
  and a small dot on the device row in the explorer.

## P4 — project file versus real devices (point 3)

Delivery order within P4 (owner approved): **package 1** includes single/filtered
batch, session tags, application identification and decoded address/association
tables; **package 2** records actual parameter bit attribution and adds named
project/device values. Each is independently reviewed, fully gated on branch
and fresh-main merge, and published. Package 2 follows immediately after package 1.
Package 1 is published after branch/merged/corpus acceptance. Package 2 now
captures actual active parameter writes and decodes named numeric/text values,
with explicit unknowns and all raw ranges. Full branch49/49 and actual merge51/51
pass, along with isolated OriginalData152/152, targeted RED/GREEN, seven semantic
mutants and self-review. Integration preserves the published universal-support
allocation preflight. Two inherited additional product opt-in failures were
reproduced on unchanged upstream and corrected separately in `a6cdfd9d`: v23
unknown-row pins now match independently predicted counts, and the nesting
census explicitly accounts for unchanged-standard-size refusals. Targeted16/16,
ordinary ProductDB887 and seven semantic mutants pass. No production admission
limit changed; final combined full/template and corpus acceptance is pending. [Contract](../contracts/device-comparison.md),
[ADR-0130](../adr/0130-session-device-comparison.md).


- A Compare button per device, and Compare all over the devices currently
  shown in the device view (filtering by line = comparing a line): one
  gateway confirmation, progress, Cancel, a failing device does not stop the
  run, refused with a clear message while the bus monitor holds the tunnel.
- Tags for the current session only: Synced, Different, Outdated (the device
  was edited after the comparison, via P3's change tracking), Wrong
  application, No answer, Not comparable (grey, reason as tooltip). No tag
  means not compared yet.
- Wrong application must be detected; today `compare_with_plan` checks only
  mask and manufacturer, so a different application reads as "everything
  differs".
- Comparison table, decoded: group address table, associations, then
  parameter name with project and device value; a raw-bytes row for anything
  that cannot be attributed, never hidden. Deliver in two steps (tables
  first, then parameters); parameters need the download image to record
  which parameter sits at which bit.
