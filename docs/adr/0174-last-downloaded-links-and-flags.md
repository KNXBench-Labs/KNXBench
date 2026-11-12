# ADR 0174: The project remembers the links and object flags a verified download wrote

Date: 2026-11-12
Status: Proposed (backend implemented in WP-37f; UI pending; package acceptance pending)
Session: WP-37d

## Context

[ADR-0173](0173-last-downloaded-parameter-values.md) gives every device a
**parameter baseline**: the effective parameter values a completed regular
download built its image from. The parameter editor marks each value that
differs from it. Links and communication-object flags got no per-item marks
(ADR-0173 §5, WP-37 "Not covered"); the user sees only the per-part flags.

Verified in source on main `be339a7e1`:

- **Which part writes what.** The group object table, which holds the
  object flags, lies in the application program's memory; a mask-`070n`
  partial download writes it only with Parameters. Every `0701`/`0705`/`2705`
  program of the private corpus places it in a segment of load state machine
  3 ([research note](../research/2026-11-12-group-object-table-load-machine.md)).
  Flag edits therefore mark **Parameters** (ADR-0129 amendment of 12 November
  2026), and `still_current` credits `flag_overrides` with the Parameters part.
  The group address and association tables, which carry the links, are the
  **Group addresses** part.
- **Links.** `ImageRequest::links` is a list of `TableLink { object: u8,
  group_address, sending }`, one entry per object number and group address; a
  send and a receive link to one address are one sending association
  (WP-39). `image_request::links` builds it from the project's object
  instances and refuses links on two instances with one number, numbers above
  255, dangling group addresses and several sending links on one object.
- **Flags.** `ImageRequest::flag_overrides` holds only the flags the project
  *states* over the product's, keyed by the instance's `ComObjectRef` lookup
  id. The device holds the *effective* flags of every object the parameter
  values activate: the product's flags with the overrides applied
  (`FlagOverrides::apply` in the image builder). An override equal to the
  product's flag, or one on an object the values leave inactive, changes no
  octet. `ProgrammingContent` already compares effective flags.
- **Module objects.** `com_object_lookup_id` drops the module segments, so
  all instances of one module definition share one lookup id, and module
  object numbers are module-local. A download refuses linked module objects
  and active module refs; nothing of a module object reaches a device yet.
- **Settlement** (`settle_download`) has the written request in hand and
  resolves the written parameter values in the catalogue lease that proves
  `SourcePlan::Unchanged`; ADR-0173 records them there. Server batch downloads
  (ADR-0172) use the same settler. The CLI currently does not call this
  settler or record these project baselines.

## Decision

### 1. Two additions, each in the part that writes it

- **Object flags join the parameter baseline** (ADR-0173). Next to the
  values, it records the **effective flags** of each object the written
  values activated, keyed by object number (what the group object table
  holds): read, write, transmit, update and communication. Read-on-init is
  nullable and currently **null** for every admitted regular-image format:
  `070n` discloses its loss and legacy images refuse enabled read-on-init;
  `FlagOverrides::apply` applies five flags, not six. Null means not written,
  never a resolved false or default. They are recorded, removed and pruned
  with the values, by the Parameters rules of ADR-0173 §3.

  Factor a **shared effective-object-flags resolver** from image construction:
  activation, product object/ref flag selection and written overrides. The
  image builder, settlement and current comparison must all use it. The
  existing inline resolution (`image.rs`, the active-object loop) is not a
  public hook; `resolve_values` resolves parameters only. Do not reuse the
  private `active_object` whole: that would pull unrelated size/priority
  admission into the comparison. Do not use the diagnostic-discarding active
  refs helper: evaluation uncertainty is an error, never an inactive object.
  Check duplicate active numbers before collecting the number-keyed map.
  Resolve under the same catalogue lease as the written values. Extend
  `WrittenParameters` (the settlement input), `ParameterBaseline` and its
  `Record` instruction together; the settler does not resolve product data.
- **Links get their own section, the link baseline**: the program id, the
  written request's `links` exactly as the download built the tables from
  them, and the settlement time. It follows the **Group addresses** part.

Both describe what a completed download built its image from, not a read of
the device: the UI says "at last download", never "in the device". A device
without one has an *unknown* baseline for that section, never "unchanged".

Module-scoped objects are out of scope in both, as module parameters are in
ADR-0173 §1, until module downloads exist.

### 2. Lifecycle

Everything ADR-0173 §2 and §7 say about the parameter baseline applies to the
link baseline: keyed by `DeviceId`, outside `Project`, history images,
versions and undo; pruned at save by the same identity rule; kept or dropped
on version restore by the same rule; copied by Save As; held in server memory
for a project without a native file. Prune, restore, Save As and a manual mark
treat both sections of a device alike (a device the rule keeps keeps both,
one it drops loses both); settlement changes each section by its own part.

The implementation must cover the union of both sections wherever ADR-0173
enumerates baselines: the store's `prune` (today it lists only
`device_parameter_baseline` and returns early when that is empty), the set a
restore keeps and hands back, `BaselineEdit` and the settled edits, Save As,
and the manual mark's rule for writing its journal entry synchronously (now:
when the device has either section).

### 3. Only settlement writes it

`Settlement` gains `link_baseline: Keep | Remove | Record { program_id,
links, at }`, decided by the Group addresses part as `parameter_baseline` is
by the Parameters part (whose `Record` will also carry the resolved flags):

| Settler and end | Link baseline |
| --- | --- |
| download `Completed`, source `Unchanged`, Group addresses in scope (complete, partial or batch) | `Record` the written request's links, even when the project changed during the run |
| download `Completed`, source `ChangedOrUnknown`, Group addresses in scope | `Remove` |
| download `Interrupted`, Group addresses attempted | `Remove` |
| download `Interrupted`, Group addresses in scope but not attempted | `Keep` |
| download with Group addresses not in scope | `Keep` |
| download `NothingWritten` | `Keep` |
| System-2 download or recovery that may have mutated the device (every such settlement touches Group addresses, a verified run included) | `Remove` (the parameter baseline, flags included, is already removed by ADR-0173) |
| System-2 without possible mutation | `Keep` |
| individual-address programming | `Keep` (the address is a dependency of the tables, not their content; the per-part flag says so) |

A manual mark removes both baselines. Adopting values, links or flags from a
device comparison never records either.

### 4. The comparison

The current side is built from the project with the same functions the
download uses, but only the parts needed: expose the private request helpers
for links and flag overrides, use the already-public `resolve_values` and the
new shared effective-object-flags resolver (§1). A refusal elsewhere in the
request (a priority conflict, for example) must not hide the marks.

- **Links.** Per object number: a group address linked on one side only is
  added or removed; linked on both with a different `sending` is a changed
  direction.
- **Flags.** Per object number of the stored side and of the objects the
  current values activate: an object active on one side only, or with a
  different effective flag, differs.

Links and flags have **separate usability states and settlement times**.
A Group-addresses-only download can yield usable links while flags remain
unknown; an error in the flag comparison does not suppress usable link
marks, or vice versa. The parameter values retain their existing comparison
state independently of the new flag section.

Each section is **not usable** (state reported, no per-item verdict) when it is
`absent`; when its program differs from the device's current resolved program
(`otherProgram`); when the product data needed for the current side is
unavailable (`productUnavailable`, needed for flags and to resolve the
current program); or when the current side cannot be built
(`currentUnreadable`): no individual address, links on several instances of
one number, a number above 255, a dangling group address, several sending
links, two unscoped instances with one lookup id, an unreadable flag,
uncertain activation, or distinct active unscoped refs resolving to one
object number. The last case makes flags `currentUnreadable`: never merge
or choose a winner, even when their effective flags happen to be equal.
The image table writers already refuse duplicate active numbers. Perform
this check before map collection and row attribution; link-only errors affect
links and flag-only errors affect flags. A missing individual address is an
intentional shared prerequisite: each present, otherwise usable link or flag
section is `currentUnreadable`; parameter-value comparison stays independent.

The per-part flags stay the authority. A part can be pending while nothing
differs: Group addresses after an address change, a parameter change (the
tables depend on it) or a priority edit (it decides only whether an image is
admitted, encodes no octet and is not compared); Parameters after a path that
removed the baseline. The UI then says that nothing differs from the last
download, or that it is unknown.

### 5. Presentation

The server adds one optional, server-only field to the device detail
(`DeviceDetail`, filled in `domain::device_detail` like
`project_folder_currency`): the link and flag comparisons with their separate
states, the
settlement times and, keyed by `ComObjectNode.id`, each instance's added,
removed and redirected links and its flags at last download. Edit responses
do not carry it; the client reads it with the next detail read.

Number-keyed differences go to one selected row per number: the instance
that holds the links, else the unique active one, else none. Ambiguous active
numbers are refused before this step, not resolved by selection. Differences
with no selected row are counted in the summary line: active product objects
need not have project instance rows, and an inactive unlinked instance is
not selected. This is not limited to deleted objects.

In `ComObjectTable` a differing instance row carries the step-1 amber mark;
its editor row (`<details>`) lists links added since the last download,
links removed since then as struck-through entries, changed directions, and
"Flags at last download: …" when both sides have a flag set and differ.
Presence is explicit: a current-only active object says "not active at last
download"; a stored-only one says "not active in current settings" and shows
its stored flags. Neither means an unknown baseline. A summary above the
table gives the number of differing objects, how many the current collapse
or filters hide, and how many have no selected row. It reports the two
sections' settlement times separately and gives a plain sentence for each
unusable section without suppressing the other. Group address views get no
marks in this step.

### 6. Storage

Native schema 21 adds:

- `device_parameter_baseline.object_flags_known` (0/1, NOT NULL, default 0).
  Migrated headers retain their parameter values and have unknown flags.
  A completed Parameters settlement with successfully resolved flags sets
  this marker to 1 even when no objects are active. In memory unknown flags
  and a known empty map are distinct (for example `Option<Map<…>>`).
- `device_object_flags_baseline` (device id, object number 0..255, the five
  written flags as 0/1, read-on-init nullable 0/1): part of the parameter
  baseline; primary key (device id, object number); rows only for a device
  with a `device_parameter_baseline` row whose `object_flags_known` is 1.
  Flag-resolution failure must not keep an older flag set or invent an
  empty one: use unknown, preserving resolved parameter values. Parameter
  value resolution failure follows ADR-0173's existing Remove rule.
- `device_link_baseline` (device id primary key, program id, settlement
  time) and `device_link_baseline_link` (device id, object number 0..255,
  group address as its raw value 0..65535, sending 0/1; primary key (device
  id, object number, group address)).

A link header without link rows is a recorded empty baseline (no links).
For flags, a known header without flag rows is a recorded empty set (no
active objects), distinct from a migrated unknown header with no flag rows.
The migration from 20 is additive, preserves every existing parameter-value
baseline and leaves the new flag and link sections unknown. Each becomes
known only through a completed download of its own part, not merely any
next download. The snapshot codec excludes the new tables; the
excluded-table lists of the snapshot, manufacturer-file and migration tests
and both schema constants (store and core) change with it. Rows are written
in the transaction that persists the settled flags, as in ADR-0173 §7.

## Backend contract (WP-37f)

The backend implements native schema 21 and independent section settlement.
`WrittenParameters` carries resolved values and optional effective object flags;
`settle_download` also receives the settlement instant independently, so a failed
parameter resolution cannot prevent recording written links. `BaselineEdit`
names parameter and link changes explicitly: Keep does not refresh or remove
an unselected section. The held collection and native lifecycle enumerate the
union of section owners, including link-only devices.

`GET /api/device/{id}` adds optional `last_download: ObjectLastDownload`.
Its `links` and `flags` have separate `state` and usable `at`; `differing`
is number-ordered with presence-aware raw-address/direction and effective-flag
differences. `instances` keys are actual selected `ComObjectNode.id` values;
`differing_objects` counts each number once and `no_selected_row` includes
product-active numbers without a project instance, not only deleted objects.
Edit responses do not carry this comparison. Read-on-init remains null.

Current comparison uses the writer's link/override helpers and checked flag
resolver, not whole-image admission. `resolve_effective_object_flags_with_refs`
returns active reference provenance from that same checked pass for attribution;
no second diagnostic-discarding activation pass chooses a row. The existing
full-download duplicate-lookup overwrite and object-table placement guard remain
separate follow-ups. No communication-object UI marks or summary are implemented
in this package; proposed presentation above remains pending. Acceptance evidence
is scoped software evidence, not hardware or whole-compatibility proof.

## Alternatives considered

- **Compare the downloaded table images byte by byte.** The association and
  address tables are sorted and packed: one added address shifts every later
  entry, so a byte difference names no link. The object table also carries
  product-defined bits the user never edits.
- **Keep flags with the links in one section.** It was this ADR's first
  draft. The flags are written with the Parameters part; a group-addresses
  download would have recorded flags it never wrote, a parameters-only one
  kept stale ones.
- **Store the stated overrides instead of effective flags.** Cheaper (no
  product data at the comparison), but an explicit flag equal to the
  product's would show as different and an override on an inactive object as
  written, unlike `ProgrammingContent` and the device.
- **Key links by the project's object instance id.** The device holds
  numbers, and `links` already carries them; an instance id says nothing
  about what the device has after a product change.
- **Derive the per-part flags from the comparison.** As in ADR-0173, the
  flags stay the authority; imported projects have no baseline.
- **Mark group addresses too.** Useful, but an address gathers objects of
  many devices; this step keeps the view where the device's tables are.

## Consequences

- The user sees which links and object flags changed since the last download
  KNXBench completed, for devices programmed by KNXBench since schema 21.
- A schema step (21) with migration, refusal and snapshot-exclusion tests;
  files saved by such a build no longer open in builds before it.
- Settlement, prune, restore, Save As and manual-mark paths handle a second
  section; their tests cover both and that prune, restore and marks treat
  them alike.
- Acceptance must amend ADR-0173's `WrittenParameters`/`Record` contract
  and per-item-mark scope when this extension is implemented. Its values comparison remains independent of flag usability. The System-2
  settlement contract must remove both baselines for every settlement
  representing possible device mutation, including verified downloads;
  no-mutation paths keep both.
- Required regression cases: migrated flags unknown versus recorded empty
  flags (parameter values preserved); known empty links versus absent; one
  section usable while the other is unknown or unreadable; uncertain
  activation and duplicate active numbers; current-only/stored-only objects;
  inactive/equal-to-default overrides; read-on-init null; all lifecycle paths
  over the union of section owners.
- Separate download-path follow-ups, not fixed by comparison refusals: the
  existing duplicate-lookup overwrite in `flag_overrides` must become an
  explicit refusal, and a `070n` product with an object table outside the
  application program's segment needs a guard (research note).
- Implementation in two further packages: the server part (schema 21,
  settlement, comparison in the device detail) and the UI part (object-view
  marks, summary line).
