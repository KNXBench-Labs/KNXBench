# WP-27 — Compare button in the device editor

Owner request: 2 November 2026. Recorded only; not claimed, no implementation Go.

## Requested outcome

Next to *Download to device…* in the device editor heading
([`OpenDownloadButton`](../../apps/knx-web/src/OpenDownloadButton.tsx) in
[`Inspector.tsx`](../../apps/knx-web/src/Inspector.tsx)) a *Compare* button
compares the project with the device's current state.

## Existing basis

The read-only device comparison already exists and is started from the
Devices workspace ([`DeviceComparisonDialog`](../../apps/knx-web/src/DeviceComparisonDialog.tsx),
[`useDeviceComparisons`](../../apps/knx-web/src/useDeviceComparisons.ts)). The
button starts that same comparison for this one device and shows its result,
including the existing device → project adoption; no second comparison engine
or result store.
