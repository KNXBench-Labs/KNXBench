# ADR-0096: Devices navigation and a snapshot-bound catalogue batch

- **Status:** Accepted implementation decision; local delivery/publication tracked separately
- **Date:** 2026-10-08
- **Approval:** Owner accepted Q1–Q5 of the Devices interview and explicitly gave
  implementation go with the proposed Q6–Q12 defaults. Review is in-session
  self-review, not an independent-agent or hardware acceptance.

## Context

Devices were selectable in the explorer, topology, buildings, search and Flow,
but selecting one appended its editor below the current central view. Large
structures could put the editor out of sight. There was no project-wide Devices
navigation item, and several device mentions were plain text.

The existing project tree is a **placement projection**, not an inventory of the
canonical device registry: devices can occur in both topology and buildings, or
be absent from all placement branches. Product/manufacturer/order text belongs
to the separately stored product database, not the domain device or the UI.
A per-row device-detail fetch would also evaluate/translate unnecessary objects
and parameters. Source investigation: `knx-projection::build_project_tree`,
`build_device_node`, server `domain::device_detail`, and the existing
`knx_productdb::query::device_product` contract.

## Decision

1. Add a dedicated Devices list and a central device-editor view. Reuse the
   existing DeviceWorkspace, properties inspector, command palette and shared
   multi-selection/bulk commands. Keep list state session-local across editor
   visits; use app-local Back, without introducing a router or browser history.
2. Add authenticated, read-only `GET /api/devices` alongside the unchanged
   device-creation POST. Return version-1, server/revision-bound canonical
   DeviceNodes and lightweight product identity metadata. Reuse the canonical
   pure device projection; do not duplicate it in the server or redefine the
   internal domain model around a UI table.
3. Read identity/reference inputs under the project lock, release it, then
   query under the product-database lock. Resolve each distinct product
   reference once per batch. No parameter evaluation, store migration,
   mutation command or protocol acquisition occurs on this read path.
4. Product-only resolution states distinguish no reference, no database,
   absent product and query failure. A query failure returns canonical device
   identities with explicit unavailable metadata and a problem marker instead
   of hiding unplaced devices. Client admission rejects malformed/duplicate
   identities and unsupported/stale response bindings rather than guessing.
5. Build a scoped UI navigation index by canonical ID and individual address.
   Explicit IDs require membership; observed addresses require exactly one
   distinct current-project device. Unresolved targets remain passive with
   an explanation. Link buttons cannot submit hardware forms or activate a
   monitor row's compose action.
6. Keep the existing monitor mounted and polling during editor visits. The
   separate diagnostic companion gains no editing-navigation provider or
   mutating API capability. Existing guarded Flow-to-main navigation remains
   the source-bound route for Flow windows.

## Consequences and boundaries

The KNX core, project/product schemas and KNXnet/IP behavior are unchanged.
Device table grouping, URL deep links, browser history, persistent UI state,
project-diff links and parsing arbitrary log prose into device targets remain
outside this package. Product resolution is not application compatibility,
commissioning readiness or physical-device identity.

The new UI/API require a matching source build; the released alpha.5 AppImage
is not replaced by implementation. No commit, push or deployment follows from
this architecture decision. Native WebKitGTK/Orca, real hardware and broader
vendor/corpus acceptance remain separate.

## Verification

[Device navigation contract](../DEVICE_NAVIGATION.md) names the HTTP, component,
stateful-parent, capability-inventory and strictly intercepted production-browser
checks. [Delivery evidence](../evidence/devices-navigation-2026-10-08.json) binds
the actual executed gates and the reviewed local candidate. Historical negative
attempts remain negative; final passes do not rewrite their outcomes.
