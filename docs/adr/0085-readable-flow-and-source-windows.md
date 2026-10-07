# ADR-0085: Readability-first flow layout and source-bound secondary windows

- Date: 2026-10-07
- Status: Accepted by the user after a grill-me interview and explicit implementation go.
- Scope: Presentation and read-only editor navigation; no KNX core, storage or bus-protocol change.

## Context

The user's live-house screenshot showed a cramped drawing area, overlapping labels
and crossing paths. Repository inspection found a fixed 960 × 520 solver/view box,
a CSS height cap and an Inspector column reserved even without selection. Existing
activity springs pull busy pairs together. Individual monitor mounts own separate
in-memory flow models: a shared backend session does not share their full history.

The user explicitly prioritizes fewer crossings over stable node coordinates,
wants a dedicated browser/Linux window, project links, and switchable auto zoom.
This supersedes ADR-0077's activity-distance/local-position tuning, not its evidence,
expiry, configured-versus-observed semantics or read-only boundary.

## Decision

1. The canvas measures its actual viewport; its growing logical world is not
   clamped to the old box. Reserve Inspector space only on selection; allow close
   and an in-app maximized view with keyboard confinement and Escape restoration.
2. Use deterministic connected components, BFS ranks, barycentre ordering and
   bounded rank packing. Wide stars use compact hex rings. Rearrange on graph or
   viewport changes, not every telegram, plus an explicit rearrange control.
   Existing motion machinery makes bounded transitions to readable anchors instead
   of pulling busy nodes into a cluster. Freeze prevents changes to existing nodes.
   Motion Off/reduced motion permits instantaneous static layout, never animation.
3. Curves try bounded alternatives around foreign node/name/value footprints.
   This detailed routing is limited to 80 nodes / 250 edges; above that, explain
   reduced routing and retain every recorded edge. Pulse positions use exactly the
   same routed curve. This is best effort, not a crossing-free guarantee.
4. Auto fit is initially on and includes node footprints, curve extrema and edge
   labels. Fit on structural/viewport changes, not solver frames or value expiry.
   Manual pan/zoom disables it; Off preserves the camera; Show all is a one-shot
   fit. Cameras, selection, Freeze and layout belong to each view independently.
5. A mounted source monitor owns its graph. Keep it and visited presentation state
   alive across main-editor navigation and diagnostic/table switches, while stopping
   hidden animation and refresh timers. A secondary Flow role only subscribes to
   that source through a same-origin BroadcastChannel; it creates no polling loop,
   bus session, project editor or persistent traffic store. Late windows receive
   the accumulated graph, source capture/context diagnostics and subsequent updates.
6. Bind a channel to an opaque random source lifetime. Get random bytes through
   `crypto.getRandomValues`, which works on non-secure LAN HTTP too; do not conflate
   these presentation identities with existing UUID-formatted API/load tokens.
   Snapshots have monotonic revisions; session/server replacement cannot replay
   old data. Translate all observation times/deadlines by the documents' time-origin
   difference. Preserve send-time age too: a delayed snapshot is not newly live.
   Missing source heartbeats retain the last map with an explicit non-live notice
   and disabled links (6-second timeout, checked every 2 seconds).
7. Inspector links name an exact device or raw group address plus evidence
   generation. Participant contexts capture the source editor's transient project
   scope. Resolve against that scope, unique current installation/entity/address
   and evidenced name; do not guess from formatted addresses or reused ids.
   On activation the main editor also verifies the server's current project
   revision/incarnation and rechecks its local identity after the await, then uses
   existing select/reveal paths. Only the main window edits; a satellite requests
   selection there and focuses it. Unknown, ambiguous, deleted, replaced or
   unverified-context targets have an explained unavailable state.

## Platform and delivery boundaries

The Flow role shares the bundle/auth/settings bootstrap but mounts no editor or
BusMonitorPanel. Tauri's `flow-*` capability permits window enumeration and main
focus only. Creation is initiated through the existing main-window capability.
Repeated open focuses the same source window without reload. Blocked browser
popups and native creation errors are reported; the embedded view remains usable.

No bus/hardware authorization follows from the user's screenshot or implementation
go. Browser acceptance uses intercepted synthetic data inside a loopback-only
network namespace. Native capability compilation and adapter outcome tests do not
establish a real WebKitGTK window or screen-reader run.

A source in the diagnostic-only companion has no bound editing-tree identity;
project links are unavailable there rather than inferred. The fully verified
navigation path is main editor → Flow / dedicated Flow window → main editor.

## Consequences and limitations

- No project/product schema, saved coordinates, traffic database or migration.
- Graph history is the source's accumulated session history, not recoverable bus
  history before attachment. Reload/source loss does not persist it.
- Dense maps can still cross; fitting a large graph can make text small. Switch
  auto zoom off and zoom in or inspect text details. Visual long labels are
  ellipsized; full names/values remain in accessible node labels/the Inspector.
- Full subscribed-window snapshots trade simplicity for structured-clone work;
  they are emitted only to active subscribers. Many windows/long-running peer
  transfer have not been load-certified. Existing large-map Motion Off advice stays.
- The source's gap, pruning, stale/unverified context, ended-session and polling
  diagnostics stay inside maximized/shared views; no clean graph hides capture loss.
- An explicit-height flex canvas avoids SVG intrinsic-aspect/ResizeObserver feedback
  that otherwise makes the canvas grow continuously and nodes unclickable.

## Evidence and references

- `flowPresentation.test.ts`: deterministic placement, label separation, compact
  fan-out, lower sampled curve crossings on a six-device cycle, obstacle routing,
  camera bounds and cross-document expiry preservation.
- `flowChannel.test.ts`: late graph adoption, stale revisions/session replacement,
  malformed packets, source close and delayed-live refusal.
- `flowNavigation.test.ts`, `e2e/flow-navigation.e2e.ts`: real parent/device/GA
  selection, secondary-window selection, reused ids/external replacement and a
  project replacement while navigation waits; source camera/history retained.
- `e2e/flow-ux.e2e.ts`: canvas sizing stability, selected/closed Inspector, maximal
  view keyboard boundary, auto-fit/manual override, late-window history/expiry,
  pause/close, no satellite bus polling, blocked popup and retained source warnings.
- Existing busy-hub, motion, expiry, theme and diagnostic import-graph regressions.
- SDK implementation audited in pinned `@tauri-apps/api` 2.11.1:
  `WebviewWindow.getByLabel` calls `plugin:window|get_all_windows`; no guessed API.
- Browser primitives: [BroadcastChannel](https://developer.mozilla.org/en-US/docs/Web/API/Broadcast_Channel_API),
  [time origins](https://developer.mozilla.org/en-US/docs/Web/API/Performance/timeOrigin),
  [random bytes](https://developer.mozilla.org/en-US/docs/Web/API/Crypto/getRandomValues).
