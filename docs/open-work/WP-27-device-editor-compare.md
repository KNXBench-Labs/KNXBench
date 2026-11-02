# WP-27 — Compare button in the device editor

Owner request and bounded implementation admission: 2 November 2026. The
implementation candidate has focused verification; independent review, browser
acceptance, integration and publication remain pending with the delivery owner.

## Implemented candidate scope

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

## Focused evidence

Synthetic, offline fixtures only; no real KNX access. Before production edits,
the owning hook regression failed because same-tick confirmation made two
calls, and the actual mounted App's EN/DE editor cases failed at the missing
Compare button. A following scope-replacement regression, after only the
same-tick guard fix, failed because a pending read lost exclusivity before its
response settled. These are behavioral failures, not missing-API or setup
failures. A separate shared-lease timeout executed no npm/test stage and is not
RED evidence.

The final focused run passed **485 tests in 12 files**, including eight hook
cases, five mounted App comparison cases and all tests in their existing App,
editor, table, adoption and minimally adapted consumer files. Coverage includes
invalid/empty gateways, an exact frozen editor target despite table filter and
selection, shared running/Stop/results, retained modal/draft/focus, late
resolve/reject across project/null transitions, revision invalidation and
separately confirmed adoption followed by Undo. The owning application and
`tsconfig.comparison-fixtures.json` typechecks passed; the latter explicitly
compiled the extended intercepted comparison browser spec.

Browser scenarios were authored for EN/DE at 1440/400 pixels and editor adoption,
but **were not executed** in this implementation track. Chromium, visual review,
production-devices/default browser acceptance, build/package/full gates and
fresh-main integration remain the delivery owner's work. Focused GREEN is not a
feature-completion, compatibility, release or publication claim.
