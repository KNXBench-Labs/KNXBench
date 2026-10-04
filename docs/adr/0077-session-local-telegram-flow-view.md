# ADR 0077: A session-local telegram-flow view is not physical topology

Date: 2026-10-04
Status: Accepted — architecture and user scope; implementation is pending
Owners: UI U19–U21; alpha AR20/AR21

## Context

The user requested a living-nervous-system visualization in the Alpha and
confirmed a focused interview: direct device links with inferred-recipient
labels, source-based rolling activity leader, dynamic but freezable geometry,
session-only retention, immediate values rather than arrival-delayed updates,
three per-group-address badges plus Inspector overflow, and explicit pulse
coalescing under load. Theme preferences remain authoritative. The user
subsequently authorized the owning Goal sessions to implement the agreed plan.

[The research/contract](../TELEGRAM_FLOW_VISUALIZATION.md) records the primary
KNX PDF filenames, independently checked printed/PDF pages, existing source
paths, browser/layout evidence, acceptance cases and ownership handoffs.

The Application Layer specification §3.1.3 (pp.16–17) distinguishes local
confirmation of GroupValueWrite from remote application confirmation. The
Application Interface Layer §3.1/§3.2 and §3.3 (pp.6–11) supports several
objects/addresses and flag-dependent handling. Observed traffic, project
membership and actual device state therefore cannot share one evidence label.

The earlier [flow-view research](../RESEARCH.md) §16 preferred a selected-row
inspector before animation. [ADR-0019](0019-building-model-stays-topological.md)
rejects invented physical/project coordinates and a floor-plan editor.

## Decision

1. Add a **read-only, monitor-session-local communication graph** as required
   Alpha scope. Coordinates represent presentation and communication activity,
   never building positions, cable length, physical distance or KNX topology.
   They never enter the project/domain/product stores. ADR-0019 remains intact.
2. Narrowly supersede §16's no-animation-first recommendation with the user's
   new scope decision. Preserve its observed/configured/ambiguous distinctions
   and the requirement for session-bound interpretation evidence. The frozen
   spatial-editor exclusions and existing source-ID dispositions do not reopen.
3. Reuse the existing capture/session/decoder paths. Add the minimum read-only
   service snapshot/type/age contract in AR20; keep project participant
   resolution out of the UI and do not open another tunnel or auto-start capture.
4. Bind source and participant evidence to the server incarnation, monitor
   session and exact flow context. Detect relevant link/device/flag/activation
   changes even when existing DPT/name comparison still looks current. Do not
   reinterpret old events against a different project. Destination type is
   authoritative data, not a guess based on address formatting.
5. Project-linked endpoints are **configured participants**, not confirmed
   recipients. Sending associations do not exclude receiving membership.
   Unknown/ambiguous/unsupported data stays explicit; no first-match guessing,
   fabricated object flags, delivery acknowledgement or read/response pairing.
6. Update per-device/per-group value slots immediately from admitted,
   value-bearing observations. Expire after 7 seconds or replace with a newer
   same-slot event. Pulse arrival never updates data. Reads carry no fabricated
   value and do not extend an existing value lifetime. Late/history observations
   cannot be revived as current just because a view is mounted or resumed.
7. Activity ranking counts observed source frames once within a labelled rolling
   window, not the number of projected targets. Inactive observed edges retain
   a readable resting line for the session. Explicit capacity/overflow behavior
   must not silently evict quiet edges or pretend gaps are complete evidence.
8. Separate layout freeze from monitor pause: freeze stops geometry only;
   values, expiry and pulse traffic remain active. Motion Off/OS reduced motion
   cancels moving geometry and pulses; static direction and values stay usable.
9. Theme tokens govern the view. No graph-specific arbitrary CSS pack, CRT-only
   palette, physical location schema, persistent traffic store, new KNX writes,
   protocol timing claim or extension framework.
10. U19 resolves measured renderer/solver/tuning/capacity and the exact AR20
    handoff; AR20 publishes the semantic contract; U20 consumes it; U21 closes
    UI motion/load/regression acceptance; AR21 verifies the integrated Alpha
    feature. AR15–AR18 include it before readiness; AR19 still needs a separate
    user-controlled release decision. Existing active owners/locks are respected.

## Alternatives

- **Keep the older inspector-only first slice:** no longer meets the explicitly
  approved Alpha scope, though the Inspector remains the semantic alternative.
- **Animate presumed reception as fact:** rejected; no remote application
  confirmation is supplied by this monitor evidence.
- **Wait for animation arrival to update values:** rejected by the user; it can
  present older values after newer telegrams and confuse rendering with data.
- **Persistent spatial canvas/history:** rejected for this slice. It requires
  separate domain/storage/privacy scope; no such migration is needed here.
- **A graph framework/WebGL/worker by default:** rejected as speculative. Native
  SVG is the first evaluated slice; a solver or other renderer needs measured
  justification, disposable layout data and lifecycle/motion acceptance.

## Consequences and acceptance

The feature is approved, not implemented or benchmarked by this ADR. The
contract document's acceptance cases and Goal checklists are the implementation
exit conditions. This does not add hardware evidence requirements to previously
accepted native/Orca/live boundaries, waive source/runtime gates, start a Goal
session, or authorize release publication.

No new KNX core entity, project migration or commissioning package is needed.
New traffic/model limits and inference uncertainty are user-visible. A static
accessible view remains meaningful when all non-essential motion is disabled.
