# Telegram flow visualization — the session-local nervous system

Date: 2026-10-04. **Approved design and alpha scope; not implemented.**
The user confirmed the interview decisions and explicitly authorized the owning
Goal sessions to implement them. This research/planning session neither starts
those sessions nor authorizes new bus access, hardware operations, or release
publication. Execution lives in [goal-ui](../goal-ui.md) U19–U21 and
[alpha-release-goal](../alpha-release-goal.md) AR20/AR21.
Architecture: [ADR-0077](adr/0077-session-local-telegram-flow-view.md).

## 1. Confirmed product decisions

- Devices appear as traffic is observed. Directed, curved connections carry
  the formatted group address; event-triggered traveling pulses suggest a
  living nervous system, not a physical wiring diagram.
- Connect devices directly when the project identifies participants. Mark
  destination endpoints **configured in project**, never received/processed.
  An unresolved destination is a visibly different group-address node.
- Update values **immediately when a valid, current telegram is admitted**.
  Pulse arrival is illustration only: it neither delays a value update nor
  certifies device reception, processing or physical travel time. This replaces
  the initial idea of updating the destination only at animation arrival.
- Keep separate value slots per device and group address. Show at most three
  current slots on a device, always with their addresses; show additional
  current slots and the observed source in the Inspector.
- A current value expires after **7 seconds**, or is replaced by a newer
  value-bearing telegram for the same slot. The newer event resets that slot's
  deadline; values on other group addresses are not overwritten.
- Dynamic, activity-dependent distances; the most actively **observed sending**
  device in the current time window is the central activity leader. Fan-out to
  project recipients is not measured reception and must not inflate that count.
- **Freeze layout** fixes positions and distances, not traffic, counts, value
  replacement or expiry. Existing monitor pause/stop is a different control.
- Inactive connections gradually lose emphasis but retain a legible resting
  line for the current monitor session. Restart starts a new map; no persistent
  communication history, device coordinates or project modification.
- At high load, bundle directional pulses and display the number represented
  and the reduced-rendering state. Do not silently discard semantic events.
- Colors, type and surfaces follow the selected built-in/imported theme.
  Global motion Off and OS reduced motion override dynamic layout and pulse
  animation; current data, static direction markers and the Inspector remain.
- This is **required alpha scope**, not an optional later enhancement. A later
  omission requires an explicit new user scope decision, not an inferred waiver.

## 2. Primary KNX evidence and what it permits

The authorized local PDF directory is `../knx-spec-kb/sources/` (plural),
under `The KNX Standard v3.0.0/03 Volume 3 System Specifications/`.
The suggested singular `source/` directory does not exist. PDFs were read
using bounded `pdftotext -layout` extracts; the printed footers independently
match the selected one-based PDF pages for all three sources below.
No specification text, complete tables, PDFs or private bus captures are copied
into this repository. The following are paraphrases, not quoted source text.

**K1 — `03_03_04 Transport Layer v01.02.03 AS.pdf`, §1.2, printed/PDF p.4.**
Group communication is connectionless point-to-multipoint; a device may belong
to multiple groups and any member can initiate communication. A group address
identifies the group, not an individually verified remote recipient.

**K2 — `03_03_07 Application Layer v02.01.01 AS.pdf`, §3.1.1 p.12,
§3.1.2 pp.13–15, §3.1.3 pp.16–17.**
Group associations connect local communication objects to group services.
A group write has local confirmation, not confirmation by each remote
application process. A read requests a value; responses are separate telegrams,
can originate from multiple configured members, and are visible to other group
members. A time-adjacent response does not prove a uniquely paired conversation.

**K3 — `03_04_01 Application Interface Layer AS v02.01.01.pdf`,
§3.1/§3.2 pp.6–7, §3.3.2 p.9 including Figure 3 note b,
§3.3.5 p.11 including Figure 6 note a.**
A device may transmit and receive, and an object may have several group
addresses. Configuration flags constrain value handling: a write update is
subject to Write Enable, while read-response updating has the specified
Update/Communication versus Write handling. Flag applicability must be resolved
from evidenced device/object data; unknown flags are not assumed true or false.

### Consequences for the graph

1. **Observed:** source individual address, typed destination, service,
   timestamp/sequence and payload actually present in the monitor row.
2. **Configured:** project device/object/group membership and known flags.
   Neither a project association nor an animation proves that the running
   device is configured identically, online, or has accepted the value.
3. **Unknown/ambiguous:** absent project, multiple devices at one individual
   address, duplicate/dangling IDs, uncertain activation/flags, unresolved DPT,
   conflicting DPT and unsupported service remain explicit diagnostics.
4. `Direction::Send` identifies the configured sending association. **Do not
   treat only `Direction::Receive` as the complete group-member set.** Inspect
   all associations and the relevant flags/activation; retain uncertainty.
5. A target badge says **value observed on this group address**, not device
   state. Known flag-based handling differences belong in the Inspector.
6. `GroupValueRead` produces a read-request pulse/label, not a new value and
   not a reset of an existing value's 7-second expiry. Response and Write are
   value-bearing only where the decoder actually supplies a value.
7. Do not reverse an edge merely because a telegram is a Response: use its
   actual observed source. No inferred read/response pairing or success ACK.
8. Individual, broadcast, management, opaque/Secure and unsupported telegrams
   are not forced into a group-device edge. Keep the monitor evidence and an
   explicit unsupported/no-value explanation. `SessionClosed` is not traffic.

## 3. Existing implementation — reuse, do not replace

Inspected on isolated source `a89224e4` and preceding `6abf557b`:

- `crates/knx-core/src/device.rs`: DeviceInstance, ComObjectInstance,
  resolved flags, activation and directional links already exist.
- `crates/knx-core/src/flags.rs`: Send/Receive are directional group
  associations, not runtime delivery evidence.
- `crates/knx-projection/src/lib.rs`: GroupAddressNode already projects
  reverse device/object associations. Reuse source facts and established
  resolution; do not build an independent UI interpretation of the project.
- `apps/knx-server/src/bus.rs`: one BusSession/tunnel, bounded telegram
  buffer and atomically held GroupAddressContext. Its interpretation comparison
  is not automatically complete for device/link/flag/activation changes.
- `apps/knx-server/src/bus_routes.rs`: rows already provide seq, timestamp,
  source, destination, service, raw payload, decoded value and control flags;
  polling provides serverIncarnation, sessionId, contextStatus, nextSince and
  droppedBefore. Typed destination, flow-membership evidence and monotonic
  observation age are **not** claimed to be existing wire fields.
- `apps/knx-web/src/BusMonitorPanel.tsx`: existing attachment, poll/cursor,
  pause, context and session-race guards; current polling interval is 1 second.
  Feed the new tab from the same session/event controller. No second tunnel,
  auto-starting monitor, per-edge polling, or simulated physical timing.
- `apps/knx-web/src/busMonitorStatistics.ts`: bounded rankings explicitly
  exclude SessionClosed. Preserve that fact and consistent event admission.
- `theme.ts`, appearance/motion preferences and the existing Inspector provide
  the theme, motion and detail surfaces. No hard-coded CRT-only colors.
- The current Web manifest has no graph/physics dependency. An imported force
  engine is an implementation decision requiring a bounded evaluated case,
  not an already available library or a promised dependency upgrade.

## 4. Architecture and contract obligations

Keep capture/decoding in knx-net/server infrastructure, read-only project
participant resolution in the application/service layer, and layout/pulses in
the UI. The KNX core receives no UI dependency, position, timer or graph entity.
No native project/product schema migration is required for this feature.

AR20 must publish a documented, tested contract before U20 consumes it.
These are **requirements for that new contract, not descriptions of existing
routes or DTO fields**; name final fields/routes in its delivery receipt:

- A typed raw destination and stable source identity, without parsing display
  strings to guess group versus individual addresses. Formatting uses the
  existing project group-address style, not a slash-string identity key.
- Server incarnation + monitor session identity + exact flow-context generation.
  The comparison/snapshot covers device identity/address/name, object ownership,
  all group links, flags, activation and DPT/display facts needed for the graph.
  A link/flag/address edit must be detectable even if decoded DPT/name is equal.
- Immutable, session-bound participant evidence with exact/ambiguous/unresolved
  distinctions. Never resolve old events against a newly edited project; stale
  or unavailable context blocks new inferred target claims, while raw traffic
  remains visible and old context is labelled historical.
- Sequence/cursor admission, repeated-delivery deduplication, dropped evidence
  and restart/stale-response handling follow the existing monitor. Wire repeats
  with distinct admitted sequences count as observed frames; retrying a poll
  does not create another event. No fan-out multiplier in sender rankings.
- Reliable observation age for retained rows, suitable for a monotonic client
  deadline. Delayed batches/reattachment must not revive old values as live.
  Do not assume clocks match or invent a fresh receive time for historical rows;
  unavailable age has a visible historical/unknown fallback.
- Preserve decoded value/unresolved/conflict/error states and raw payload.
  Validate IDs/counters without unsafe JavaScript-number rounding or coercion.
- Bounded response/project snapshots, visible refusal/overflow diagnostics and
  unchanged existing consumers. Test no project, multiple installations,
  duplicate addresses, dangling/inactive objects and missing flags.
- No live bus operation, persistent traffic database, new keys/decryption or
  mutation API. Alpha cannot change generated Web bindings without the UI owner
  and its lock; the owner adopts any agreed additive wire types in U20.

### Deterministic presentation state

A shared, session-keyed reducer owns graph membership, windowed sender counts,
last activity, value slots and expiry. The renderer consumes this state; it does
not mutate project facts. Slot identity includes flow context, device and the
raw group address. A newer admitted sequence wins; animation callbacks can
never modify values. The timestamp label remains actual observation evidence.

The 7-second lifetime starts at the admitted observation time, adjusted for
retained-event age, not pulse arrival, a later tab mount, or a timer callback.
Expire on the monotonic clock and before rendering after visibility resume.
Update current source and configured target badges together; mark the target
badge's inferred association. Showing the same value at a target is not readback.

## 5. Layout, appearance and load — engineering recommendations

These initial tuning values are recommendations to measure in U19/U21, not
user-prescribed protocol constants or already achieved performance results:

- A **60-second rolling observation window**, labelled in the UI, for sender
  ranking and edge activity. Keep the current leader on exact ties; otherwise
  choose deterministic identity order. Empty/no-current-traffic has no invented
  leader. A short damped transition moves the current leader to the center.
  Freeze/motion-Off explicitly suspends centering; the leader label stays live.
- Frequent communication shortens a bounded preferred distance; infrequent
  communication relaxes it. Smooth changes and collision separation prevent
  jumps and unreadable clusters. Stable node keys/positions survive new events;
  do not recreate the whole map on each poll or invent physical distances.
- Begin the inactive fade after an initial quiet interval (start with 10 seconds)
  and ease toward the resting line over roughly the next minute. Never reach
  invisibility; test readable resting lines in light/dark/imported palettes.
  Retain observed graph membership for the session, independently of the rolling
  activity window and the much shorter value TTL.
- Device body as soma, branching curved paths as axons, a restrained directed
  traveling highlight as the signal. No idle flicker, random particles or
  perpetual glow unrelated to traffic. Reverse traffic has its own direction;
  distinct group addresses remain identifiable even on a shared device pair.
- Prefer a **native SVG first slice** plus semantic HTML controls/Inspector.
  Evaluate Canvas only if measured SVG workload fails the stated envelope;
  Canvas would still need a semantic accessible alternative. Do not add WebGL,
  a generic graph platform or a worker merely because they are fashionable.
- A force-layout engine can express per-link preferred distances.[4]
  D3 force simulations support cooling, stop/restart and fixed nodes, but mutate
  input node/link records.[5] If selected after U19 evaluation, keep disposable
  layout copies and never pass domain/DTO objects to the solver. Motion Off and
  freeze must actually stop its timers; a CSS-only guard is insufficient.
- Use a single timestamp-driven animation scheduler, not a React state update
  or timer per pulse. Browser animation frames may pause in hidden tabs.[6]
  Retire old pulses on resume; do not play an accumulated backlog as live.
- Existing global motion policy remains authoritative; OS reduced motion requests
  less non-essential movement.[7] Off/reduce cancels active effects and moving
  layout immediately without freezing values. Colors and fonts resolve through
  admitted theme tokens; no arbitrary theme CSS or silent contrast bypass.
- At overload, coalesce pulses by directed device pair and group address within
  a short render interval, keeping represented count and newest value. Admit
  every available semantic event first. Distinguish renderer coalescing from
  capture loss (`droppedBefore`) and from graph/model capacity limits.
- Retaining edges for an entire session does not permit unbounded memory. Set
  and test explicit model/render limits in U19/AR20. At capacity, preserve
  existing observed edges and use an explicit overflow/aggregation state with
  counts and inspectable membership, or refuse new graph expansion visibly;
  never silently evict quiet edges or declare a complete map after a gap.

## 6. Ownership and dependency chain

| Package | Owner | Deliverable and prerequisite |
| --- | --- | --- |
| U19 | UI session | Evidence/contract reconciliation, tested synthetic visual slice, theme/motion/keyboard requirements and measured renderer/load limits; after the owner's current package |
| AR20 | Alpha session | Read-only, session-bound participant/event-age/type contract and semantic regressions; depends on U19's agreed handoff, never UI rendering |
| U20 | UI session | Shared monitor-feed consumer, nodes/edges/Inspector, immediate values and 7-second slot reducer; depends on integrated AR20 |
| U21 | UI session | Dynamic/frozen layout, directional pulses, persistent resting lines, overload behavior and owner closing gates; depends on U20 |
| AR21 | Alpha session | Consume both owners' exact receipts; integrated contract/browser/performance tests, docs/manual and alpha acceptance; depends on AR20 and U21 |

AR15–AR18 must include this feature before final readiness, and AR19 still
requires the user's separate exact release approval. Do not preempt active
owner work, acquire another owner's Web lock or duplicate a package.
Commissioning has **no new K-package**: no device write or device-state claim
belongs to this read-only visualization.

## 7. Required acceptance scenarios

- One/zero/multiple mapped sources; zero/multiple group members; sending
  associations also present on candidate receiving devices; known/unknown flags,
  inactive/dangling objects, duplicate physical addresses and installations.
- Write, Read without value, actual independently sourced Response, malformed
  payload, conflicting/missing/unsupported DPT, individual destination, opaque
  service and SessionClosed; no fabricated value, recipient or acknowledgement.
- Several group addresses at one device; three badges plus Inspector overflow;
  same/different-source replacement on one address; no cross-slot overwrite.
- Fake-clock boundary just before/at/after 7 seconds; newer-sequence races;
  delayed poll, stale async reply, reconnect, restart, changed project links,
  unavailable context/age and old retained rows. Pulse completion never writes.
- Tie/new leader/window expiry, freeze with incoming traffic and value expiry,
  inactive edge still present after long silence, no-current-traffic state.
- Bidirectional pulses and distinct addresses on one pair; dense bursts with
  exact counts/newest values; separate visible capture-loss/overflow states.
- Start with synthetic loads of **500 devices, 2,500 directed edges and 1,000
  admitted events/second**, plus a long-session unique-edge growth case. U19
  defines a reproducible workload and target environment; U21 records real
  frame time, event lag, CPU/memory and resource cleanup. These are test inputs,
  not claims of supported throughput or hardware traffic measurements.
- Real application integration with intercepted synthetic traffic, not only a
  detached study. Opening the view does not start/connect/write to the bus;
  panel/companion view changes do not create a second session or poll loop.
- Built-in/System/imported light/dark themes, live theme change, selected/focus
  states, keyboard selection/pan/zoom/freeze and semantic Inspector alternative.
  No per-telegram screen-reader announcement storm. Off/OS-reduce changed
  mid-pulse/relayout must stop effects immediately; hidden/resume/unmount has no
  stale timer, observer, animation or worker leak.
- Named RED/GREEN regressions and behavioral guard mutations; actual integrated
  owner/full gates and source-bound review. Existing user-accepted native/Orca/
  real-network evidence boundaries stay disclosed, not silently reopened or
  called verified by Chromium. No live bus required for this feature's alpha
  implementation acceptance.

## 8. Boundaries and remaining implementation decisions

Approved concept is not a shipped feature, verified compatibility, receipt
confirmation or benchmark. Final wire fields, event-age transport, rendering
capacity, force engine/dependency and precise motion/fade tuning are resolved
and recorded by the assigned implementation packages before shipping.

This narrowly supersedes the no-animation-first recommendation in
[RESEARCH](RESEARCH.md) §16.
Its uncertainty and snapshot safeguards remain binding. [ADR-0019](adr/0019-building-model-stays-topological.md)
continues to exclude physical/project coordinates and floor-plan editing.
The original frozen 180-ID inventory and existing source-ID dispositions are
unchanged; this new user feature is tracked by package checklists, not by
reopening an accepted spatial-editor row.

## 9. U19 resolution (goal-ui owner, 2026-10-04)

U19 delivers an evaluated, **visibly synthetic** study and the exact AR20
handoff. It is not the productive view: nothing reads the monitor feed, and
U20 owns the shipped reducer and integration. Study code lives under
`apps/knx-web/e2e/flow-study/` (test-only, typed by
`tsconfig.flow-study.json`); screenshots and measurements are in
[design/2026-10-04-telegram-flow-u19](design/2026-10-04-telegram-flow-u19/README.md).

### 9.1 Code reconciliation (source at `2231d87c`)

- `apps/knx-server/src/bus.rs:665-671`: a row's `timestamp` is an RFC3339
  **string** of server wall-clock time at drain, not a bus time and not a
  monotonic age. A client cannot derive a trustworthy age from it, because the
  clocks may differ.
- `bus.rs:672-680`: `source` and `destination` are **formatted strings**, and
  `destination` follows the project's group-address style. No typed raw value
  reaches the client.
- `bus.rs:1048-1060`: individually addressed frames are dropped before a row
  exists (no row, no `seq`, no `droppedBefore`). The graph therefore never
  sees them, which matches §2 item 8.
- `bus.rs:650, 985-1040`: a 5,000-row ring buffer. Eviction and lagged
  receivers both add to `droppedBefore`; the two causes are indistinguishable,
  which is acceptable for a visible gap marker.
- `bus.rs:821-846, 1359-1368`: the interpretation context and its `current` /
  `stale` comparison cover style, DPTs and names **only**. Device, link, flag
  and activation edits are invisible to it, as §3 suspected.
- `bus_routes.rs:736-748, 788-797`: the row and poll DTOs (`seq`, `timestamp`,
  `source`, `destination`, `destinationName`, `service`, `rawPayload`,
  `decoded`, `control`; `sessionId`, `serverIncarnation`, `contextStatus`,
  `projectOpen`, `status`, `nextSince`, `droppedBefore`). `seq` is serialised
  as a JSON number.
- `BusMonitorPanel.tsx:63`: one poll per second, one session. `crates/knx-projection/src/lib.rs:195-212`
  already projects every group link with device, object and direction
  (`Send`/`Receive`). `lib.rs:568-598` projects the object flags and
  `is_active`. The participant facts exist; what is missing is a
  session-bound, generation-tagged snapshot of them.

### 9.2 Handoff to AR20 — proposed contract

These fields and routes are **proposals** for AR20 to implement, test and
then name in its receipt; none of them exists today. Everything is additive,
and existing consumers keep their fields.

**Per telegram row** (`GET /api/bus/monitor/telegrams`):

| Field | Type | Meaning |
| --- | --- | --- |
| `sourceRaw` | integer 0–65535 | Individual address of the observed sender, unformatted |
| `destinationRaw` | integer 0–65535 | Group address, unformatted. Rows exist only for group destinations (`bus.rs:1048`) |
| `observedAgeMs` | integer ≥ 0 or `null` | Server-monotonic age of the row at response time (an `Instant` stored at push). The client's deadline is its own monotonic receive time minus this age. `null` means unknown: no live value badge |
| `flowGeneration` | decimal string | Generation of the participant snapshot the row was interpreted under, fixed at push |

**Per poll response:** `flowGeneration` (decimal string, current). The client
fetches a snapshot only when it sees a generation it does not hold.

**New route** `GET /api/bus/monitor/flow-snapshot?sessionId=&generation=`:
read-only and bounded. It returns `serverIncarnation`, `sessionId`,
`generation`, `status` (`current` | `historical` | `unavailable`),
`groupAddressStyle`, and the following:

- `devices`: `deviceId`, `installationId`, `name`, `individualAddressRaw | null`.
- `groups`: `gaRaw`, `gaId`, `name`, `dpt | null`, and `members`. Each member
  has `deviceId`, `comObjectId`, `direction` (`Send` | `Receive`), `active`
  (`true` | `false` | `null`) and `flags`: `communication`, `read`, `write`,
  `transmit`, `update`, `readOnInit`, or `null` when unknown.
- `diagnostics`: duplicate individual addresses, dangling links, ambiguous
  group or device ids, and objects without resolvable flags.
- `truncated`: counts per list.

The server keeps only the current generation's snapshot. Rows of an older
generation are shown raw and marked historical, unless the client already
holds that generation's immutable snapshot.

**Generation rule:** the generation counter increases whenever the extended
comparison changes: style, DPT and name as today, **plus** device identity,
address and name, object ownership, every group link, flags and activation. A
link-only edit must bump it even when DPT and name are equal (§4).

**Counters:** `seq`, `nextSince` and `droppedBefore` are u64 on the server.
AR20 either guarantees values ≤ 2^53−1 or moves them to decimal strings. The
client refuses an unsafe number instead of rounding it.

**What stays as it is:** one tunnel and one poll loop, no persistence, no
write or decryption API, no active probing. Sender ranking counts observed
rows only; fan-out never counts.

### 9.3 Measured renderer and layout evaluation

Workloads (§7): a deterministic synthetic installation (`9.x.y` addresses,
one group in ten unresolved), skewed group choice, 80 % Write, 10 % Read and
10 % independently sourced Response. Each scenario runs after a 3-second
warm-up and is then measured for 8–30 s. The environment is an AMD Ryzen 7
5800X with Chromium 152, headless under Playwright. **Other sessions and the
Hermes UI were running throughout**, even though all shared gate leases were
held; `load1` is recorded per scenario. Absolute values are therefore upper
bounds under contention, while comparisons within one run are fair.

| Scenario | devices / edges | frame ms p50 / p95 | script ms | draw ms | value lag ms | layout steps | edge writes / frame | load1 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `slice` | 17 / 21 | 16.7 / 16.8 | 0.7 / 1.4 | 0.6 / 1 | 11.3 / 61.4 | 644 | 17.1 | 24.41 |
| `mid-svg` | 182 / 609 | 16.7 / 33.3 | 5.1 / 7.7 | 4.3 / 6.8 | 13.2 / 29.9 | 603 | 477.7 | 22.38 |
| `target-svg-full` | 631 / 3110 | 66.7 / 83.4 | 22.6 / 30.5 | 17.4 / 24.8 | 66.1 / 83.2 | 252 | 2816 | 17.22 |
| `target-svg-frozen-geometry` | 631 / 3103 | 16.7 / 33.4 | 7.2 / 11.4 | 6.4 / 9.9 | 16.4 / 33.1 | 63 | 1.7 | 16.64 |
| `target-svg-motion-off` | 631 / 3088 | 16.7 / 16.8 | 5.7 / 8.4 | 5.1 / 7.3 | 16.1 / 16.7 | 0 | 1.5 | 16.3 |
| `target-svg-labels` | 632 / 3199 | 283.3 / 566.7 | 48.3 / 104.5 | 39.7 / 83.2 | 283.1 / 566.1 | 67 | 2958.1 | 17.06 |
| `target-canvas-full` | 631 / 3107 | 66.8 / 100 | 22.5 / 28.2 | 17.4 / 22.4 | 66.6 / 99.5 | 231 | 0 | 15.97 |
| `target-canvas-motion-off` | 631 / 3106 | 66.7 / 100.1 | 14.8 / 22.4 | 13.8 / 20.7 | 66.2 / 99.9 | 0 | 0 | 15.35 |
| `long-growth` | 1000 / 4477 | 100 / 133.3 | 32.6 / 43.2 | 24.4 / 33.1 | 98.3 / 131.53 | 405 | 3332.6 | 23.19 |
| `long-growth-canvas` | 1000 / 4464 | 100 / 133.3 | 29.3 / 35.7 | 21.3 / 25.6 | 99.2 / 132.27 | 418 | 0 | 20.11 |

The `target-*` scenarios run 1,000 events/s, `mid` runs 100/s, `long-growth`
runs 300/s for 30 s, and the small slice runs 4/s. The value lag is the age
of a batch's oldest event when it becomes visible. It equals one frame
interval, so values appear in the next frame and never wait for a pulse.

**Decisions:**

1. **Native SVG; no Canvas, WebGL, worker or graph dependency.** At rest the
   target load renders at 16.7 / 16.8 ms in SVG; Canvas 2D needed 66.7 /
   100 ms because it repaints every curve each frame. With moving geometry
   both measured 67–100 ms. Canvas wins in no scenario measured here.
2. **The cost is moving geometry, not traffic.** Pulses and live values over
   frozen geometry at the full target load run at 16.7 / 33.4 ms. A layout
   that keeps every edge moving costs 2,800 path writes per frame.
3. **Own bounded layout, no force library:** springs toward an
   activity-dependent distance, grid-local repulsion and centre/leader pull,
   O(nodes + edges) per tick, with cooling (`alpha`) and reheats. At target
   load it stays unsettled, because new edges keep appearing and each one
   reheats the whole map.
4. **Labels:** an address label on every edge costs 283 ms per frame at target
   size. Beyond small maps, labels appear only on active or selected edges,
   and all addresses stay in the Inspector.

**Binding requirements for U21 from these numbers:**

- **Reheat locally:** a new node or edge heats only itself and its
  neighbours, never the whole map, and settled regions are not rewritten.
- **Bundle pulses over a pulse's lifetime**, not per frame. At 1,000
  events/s a frame held about 16 events, so per-frame bundling rarely
  triggered, and 21,000 pulses in 10 s exceeded the 160-element capacity.
  The represented count and the over-capacity count must both be visible.
- **Keep the activity hub readable:** collision separation must account for
  badge height, or hub badges appear only on selection. The screenshots show
  neighbours' badges piling up around the leader.
- **Contrast:** check badge text (`--knx-success-color`) on the surface in
  light palettes; in Porcelain it is legible but weak.

### 9.4 Resolved study tuning (inputs for U20/U21, not protocol constants)

| Parameter | Value | Evidence |
| --- | --- | --- |
| Value lifetime | 7,000 ms from observation; expired at exactly 7,000 | `model.test.ts` boundary test |
| Same-slot rule | Per device and group address; a sequence at or below the high-water mark is dropped (repeat or stale delivery) | `model.test.ts` |
| Reads | Never set a value or renew a deadline, even if a row carries one | `model.test.ts` |
| Badges | At most 3, newest first, stacked one per line, `+n more` | `model.test.ts`, screenshots |
| Activity window | 60 s rolling. Sender counts only; an exact tie keeps the leader, otherwise the lowest id; no traffic means no leader | `model.test.ts` |
| Quiet edge | Fades after 10 s over 60 s to 0.35 opacity, never lower; membership persists | engine constants, screenshots |
| Distances | 40–220 px × area scale (0.6–2.5); busier pairs closer | `layout.test.ts` |
| Cooling | alpha × 0.985 per step, rest below 0.005; reheat 0.3 on growth or leader change, 0.08 every 5 s | `layout.test.ts` |
| Pulses | 700 ms, at most 160 elements; bundled when a frame holds more than 24 events | engine; see the U21 requirement above |
| Model limits | 1,000 nodes, 5,000 edges. Growth beyond them is refused and counted (`long-growth`: 2,433 nodes / 1,963 edges refused in 30 s); existing edges are kept and values are still admitted | `model.test.ts`, measurements |
| Motion Off / OS reduce | No layout steps and no pulses; values keep arriving | `flow-study.e2e.ts` |

### 9.5 Reproduce

```text
cd apps/knx-web
npx vitest run e2e/flow-study            # 23 study semantics tests
npx playwright test e2e/flow-study.e2e.ts # 4 browser checks (normal suite)
npm run check:flow-study                 # type-check the study
FLOW_STUDY_OUT=/tmp/flow npx playwright test -c playwright.study.config.ts
```

The last command is the measurement run (about 3.5 minutes). It is not part
of the normal suite, and its numbers depend on the machine and its load.

### 9.6 Boundaries

Synthetic data only; no monitor feed, project, bus or hardware. Chromium only:
native WebKitGTK rendering cost, Orca and real traffic are not measured. Guard
mutants (11, all caught) cover the study's semantics. The productive U20/U21
code still needs its own RED/GREEN and mutation evidence.

## 10. AR20 delivered contract (alpha, 2026-10-04)

This is the backend contract U20 consumes. Everything is additive; existing
fields and routes are unchanged. Implementation: `apps/knx-server/src/flow.rs`
(participant snapshot and bounded wire form), `bus.rs` (rows, context,
generation, counters) and `bus_routes.rs` (DTOs and route). No Web file,
generated binding, core type, project/product schema or migration changed.

### 10.1 Monitor rows and poll (`GET /api/bus/monitor/telegrams`)

| Field | Type | Meaning |
| --- | --- | --- |
| `telegrams[].sourceRaw` | integer 0–65535 or `null` | Sender's individual address, unformatted. `null` only on the `SessionClosed` marker |
| `telegrams[].destinationRaw` | integer 0–65535 or `null` | Group address, unformatted. `null` only on the marker |
| `telegrams[].observedAgeMs` | integer ≥ 0 or `null` | Server-monotonic milliseconds between admission and this response. Always present today; `null` stays reserved for "unknown" |
| `telegrams[].flowGeneration` | decimal string or `null` | Generation of the context the row was decoded with, fixed at push. `null` only on the marker |
| `flowGeneration` | decimal string | The session's current generation |

Rows still exist only for group destinations; individually addressed frames
are not admitted (no row, no `seq`), unchanged from §9.1. `seq`, `nextSince`
and `droppedBefore` stay JSON numbers and never exceed 2^53 − 1
(`MAX_SAFE_COUNTER`): at the limit the buffer refuses further rows and counts
them in `droppedBefore`, which saturates there too.

### 10.2 Participants (`GET /api/bus/monitor/flow-snapshot`)

Query: `sessionId` and `generation`, both optional decimal strings; anything
else is `400`. No active monitor session: `404`. Response:

- `serverIncarnation`, `sessionId` (number), `generation` (decimal string:
  the requested one, else the current one), `groupAddressStyle`
  (`"Free"`/`"TwoLevel"`/`"ThreeLevel"` or `null`).
- `status`: `"current"` (lists of the current generation); `"historical"`
  (another session or generation was requested; the server keeps no older
  snapshot, so the lists are empty and the client shows those rows raw);
  `"unavailable"` (no project context; empty lists).
- `devices[]`: `deviceId`, `installationId` (`null` when no or several
  installations place it), `name`, `individualAddressRaw` (`null` when none).
- `groups[]`: `gaRaw`, `gaId`, `installationId`, `name`, `dpt` (single
  resolved DPT or `null`; rows carry conflicts exactly), `members[]`.
- `members[]`: `deviceId`, `comObjectId`, `direction` (`"Send"`/`"Receive"`),
  `active` (boolean), `flags` with `communication`, `read`, `write`,
  `transmit`, `update`, `readOnInit`, **each** `true`/`false`/`null`
  (`null` = no layer states it; the object is never `null`).
- `diagnostics`: `duplicateIndividualAddresses[{individualAddressRaw,
  deviceIds}]`, `ambiguousGroupAddresses[{gaRaw, gaIds}]`,
  `ambiguousDevices[deviceId]`, `danglingLinks[{comObjectId, gaId}]` (not
  members), `unknownDevices[{comObjectId, deviceId}]` (still members),
  `objectsWithoutFlags[comObjectId]`.
- `truncated`: omitted `devices`, `groups`, `members`, `diagnostics`.
  Bounds: 10,000 devices, 20,000 groups, 100,000 members, 1,000 entries per
  diagnostics list (`FLOW_LIMITS`).

Members are configuration evidence. A `Send` link does not exclude
receiving, and a listed `Receive` member is not proof of delivery.

### 10.3 Generation and staleness

The session context now compares style, DPTs, names **and** the participant
snapshot (devices with installation, name and address; object ownership;
every link with direction; all six flags; activation). Consequences:

- `contextStatus` turns `stale` for a link-, flag-, activation- or
  device-only edit, which it did not detect before. The Web monitor will show
  its stale state more often; that is the intended correction.
- The generation starts at `1` per session and advances by one whenever the
  session's context is **replaced by a different one**. Today that happens on
  the routes that republish the context to the session: group-address style,
  undo and redo. Other edits leave the session on its generation and report
  `stale` until the context is republished or the monitor restarted. An equal
  replacement keeps the generation.
- Every session starts at generation `1` (its context is built fresh at
  start) under a new `sessionId`; a server restart changes
  `serverIncarnation`. A snapshot request naming another session is
  `historical` (tested); a stop/start sequence itself is not exercised by
  these tests, because the fake connector serves one tunnel.

### 10.4 Tests

`apps/knx-server/tests/http_bus_flow.rs` (HTTP contract, 9: rows and age, snapshot, generation bump and keep, stale on flag/activation/device/link edits, no project, historical and bad parameters, no session, closed marker, Write/Read/Response/opaque mix with individual frames excluded),
`flow::tests` (7: ordering, members/flags/activation, diagnostics, change
detection, truncation, empty project, wire names) and two `bus::tests`
(safe-counter refusal and saturation; generation keep/advance). Nine guard
mutants, all killed (log `.ai/logs/2026-10-04_claude_ar20-flow-contract.md`).

### 10.5 Not done here

No Web consumer (U20, Web lock), no hardware or real traffic, no
measurement. The flow view itself is accepted only at AR21.

## 11. U20 consumer decisions (goal-ui owner, 2026-10-04)

Part 1 (pure, not yet rendered): `apps/knx-web/src/flowWire.ts` validates
the §10 wire fields and `flowModel.ts` reduces admitted rows. These are the
rules the view in part 2 renders; they are product decisions made under §1–§4,
not KNX protocol facts.

- **Wire validation.** IDs are checked against their Rust widths (u8/u16/u32),
  counters against `Number.MAX_SAFE_INTEGER`, a generation against the
  canonical decimal form (`01` is refused), and each member must state all six
  flags (`null` is a statement; a missing key is a deviation). A refused
  snapshot is not guessed at: its rows are drawn raw (reason `failed`).
- **One model per monitor session** (`serverIncarnation` + `sessionId`). A
  snapshot of another session, server or generation is refused.
- **Admission.** A batch is ordered by `seq`; anything at or below the highest
  sequence seen (admitted or queued) is a repeated delivery and counted, not
  drawn again. The `SessionClosed` marker ends traffic and is not traffic.
  Rows without the AR20 fields (legacy) and rows with out-of-range fields
  (malformed) are counted and not drawn.
- **Own generation only.** A row waits in a sequence-ordered queue until the
  context of *its* `flowGeneration` is known; the caller fetches each new
  generation's snapshot once. The queue is bounded (5,000); when it is full,
  its oldest row is drawn raw (reason `pendingOverflow`) instead of waiting.
- **Sources.** An individual address held by exactly one device is that
  device (`d:<deviceId>`). Several holders give one ambiguous source node
  (`ia:<raw>`) that lists the candidates and picks none; no holder gives an
  unresolved source node. Without participants (`historical`, `unavailable`,
  `failed`, `pendingOverflow`) the source is a raw node with its reason.
- **Configured targets.** If exactly one group has the raw destination, its
  targets are every **active** member, Send or Receive (§2 item 4), except
  the device(s) holding the source address. Several groups with one raw
  address name no members (a group node marked ambiguous); none, or no
  remaining member, gives the group-address node. Edges to devices are
  marked *configured*; they are project evidence, not observed receipt.
- **Values.** Only Write and Response with a decoded `value` set slots;
  Read, undecodable/unresolved/conflicting payloads and rows of unknown age
  never do. Observation time is `receive time − observedAgeMs` on the
  client's monotonic clock; the value lives until exactly 7,000 ms after it.
  A row already past that when it arrives (reattachment, delayed batch) sets
  nothing. One slot per node and raw group address: the source, each
  configured target (marked inferred) and an unresolved group node. A slot
  is replaced only by a higher sequence, across generations; the slot keeps
  its generation. At most three current values per node are badges, newest
  first; the rest are counted for the Inspector.
- **Limits.** 1,000 nodes, 5,000 edges, 10,000 value slots. Growth beyond
  them is refused and counted; existing nodes keep receiving values.
- **Node evidence follows its generation.** A node keeps its identity; a
  changed kind replaces its description, so an earlier ambiguity or raw
  reason does not outlive the generation that stated it.
- For U21: an edge whose rows all had unknown age has no observation time
  (`lastObservedAtMs` is `-Infinity`); fading must treat it as unknown.

## Sources

[4] https://d3js.org/d3-force/link
[5] https://d3js.org/d3-force/simulation
[6] https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame
[7] https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion
