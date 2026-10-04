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

## Sources

[4] https://d3js.org/d3-force/link
[5] https://d3js.org/d3-force/simulation
[6] https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame
[7] https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion
