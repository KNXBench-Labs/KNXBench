# WP-27 — Compare button in the device editor

Delivered on main, 3 November 2026: source `080f2723d`. Independent review and
source-bound software acceptance are complete; this does not add hardware or
native-shell acceptance. [Measured coverage and boundaries](../status/2026-11-03-wp27-device-editor-compare.md).

## Delivered scope

Next to *Download to device…* in the device editor heading
([`Inspector.tsx`](../../apps/knx-web/src/Inspector.tsx)), *Compare* opens the
existing explicit read-only review for exactly that edited device. Opening or
navigating does not read the device. Gateway validation and confirmation remain
in [`DeviceComparisonDialog`](../../apps/knx-web/src/DeviceComparisonDialog.tsx).

[`App`](../../apps/knx-web/src/App.tsx) owns one app-lifetime
[`useDeviceComparisons`](../../apps/knx-web/src/useDeviceComparisons.ts)
controller and one retained dialog. The editor and
[`DevicesWorkspace`](../../apps/knx-web/src/DevicesWorkspace.tsx) consume that
same controller; there is no second comparison engine, backend path or result
store. The dialog retains its gateway draft and focus across foreground changes
without isolating Settings, Log, Monitor or an unrelated workspace. Global
System-2 lifecycle notices keep their existing placement.

The controller accepts an explicit absent project. Project/server scope
withdrawal invalidates old review/result authority, including a return through
no project to the same incarnation. Stop and scope changes cancel queue
continuation but retain synchronous ownership until the pending read actually
settles. Same-tick confirmation cannot admit two reads. Content revisions keep
existing conservative *Outdated* semantics. The existing separately confirmed
device → project adoption reaches the root tree; download handoff and sibling
editor actions are not replaced.

## Focused evidence and final acceptance

Synthetic, offline fixtures only; no real KNX access. Retained semantic REDs
show duplicate same-tick calls, premature pending-owner release after scope
replacement, and the missing EN/DE editor button. They are not compiler/setup
failures. A shared-lease timeout executed no npm/test stage and is not RED.

The implementation's focused run passed **485 tests in 12 files**, including
eight hook and five mounted App comparison cases. It covers invalid/empty
gateways, the exact frozen editor target despite table filter/selection, shared
running/Stop/results, retained modal/draft/focus, late resolve/reject across
project/null transitions, revision invalidation and separately confirmed adoption
followed by Undo. Three standalone browser consumers were adapted explicitly;
all 27 application/fixture TypeScript configurations then passed.

On the final runtime-tested merge `51d8f417f`, the selected package gate completed
**11/11** stages with frozen inputs, no touched/unchecked user data, **5,056/5,056**
Vitest tests and the complete selected default/production browser inventories.
The comparison spec ran **13/13** cases under each default and production-devices
configuration; the standalone editor and table-column owning configurations also
ran. Eight EN/DE editor frames at 1440/400 pixels were actually inspected.

While gating, main advanced only by six upstream documentation files. Published
main-first `080f2723d` equals the accepted tree plus that exact patch, proven with
an isolated Git index and a mismatching-tree control. Its separate documentation
delta completed **10/10** stages; runtime results retain their original source
identity rather than being relabelled as a new full run.

The catalogue polling correction discovered during acceptance was delivered
separately as `5296e56c5`; inherited footer locale preparation was corrected
without weakening strict request, explicit-connect or overlap assertions.
Historical failed runs remain retained, not waived or counted as final success.

## Retained boundaries

The existing comparison engine's family, image, identity, read and adoption
restrictions are unchanged. No automatic read, reconnect, second queue, new
backend path, transport, schema or storage format was added. The browser proof
uses synthetic intercepted requests; no real bus/device, native WebKitGTK,
release or deployment acceptance follows. Log/Monitor/Overview suspension has
mounted-App/source evidence, not an additional dedicated real-browser matrix.
