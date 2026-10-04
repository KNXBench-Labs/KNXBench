# 2026-10-04 — MODEL-02 web half: explicit placement repair

Agent: Claude, goal-ui.md owner session. Web lock taken for this package
(`e66474cb`).

## Contract (ADR-0071, server half already published)

`RepairDevicePlacement { device, keep: Line(id) | Unassigned(installation) }`
keeps the first occurrence in the chosen slot and removes every other
occurrence in every installation. `RepairLineOwner { line, keep: AreaId }`
keeps the first reference in the chosen area. Both refuse `RepairNotNeeded`
and `RepairKeepNotPresent`; the kept area must be in the line's installation.
Undo restores the exact imported lists. Routes: `POST /api/repair/device-placement`
(`deviceId` + exactly one of `keepLineId` / `keepUnassignedInstallationId`),
`POST /api/repair/line-owner` (`lineId`, `keepAreaId`).

## Change

- `api.ts`: `repairDevicePlacement(deviceId, keep)`, `repairLineOwner(lineId, keepAreaId)`.
- `treeUtils.ts`: `devicePlacementSlots`, which counts per line instance so a
  line shown under two areas is one slot.
- `Inspector.tsx`: `DevicePlacementRepair` in the device Inspector, and
  `LineOwnerRepair` under the duplicate-id alert for lines. The line repair is
  offered only for the same line in at least two areas of one installation.
  The stale `canDelete` comment from MODEL-01 is corrected.
- Messages en/de for the conflict, slot labels and buttons.
- `e2e/installations-fixture.tsx`: a device selection loads its detail through
  the real `api.deviceDetail`; the Inspector runs `propertiesOnly`.

## Evidence

- RED: 9 failing cases (treeUtils 3, api 1, Inspector 5); the guards "no
  repair for a device placed once" (en/de) and "no repair across
  installations" already held.
- GREEN: full Vitest green, including the existing duplicate-id tests that
  require no buttons for two different lines sharing an id.
- Mutation, first run: the identity check and the one-installation check
  survived. That was a real gap (buttons would appear where they must not),
  so the two missing tests were added and the mutants re-run. Final: 7 caught
  by name, including a refusal mutant; one mutant was renamed to what it
  actually changes (repair result not applied).
- `e2e/repair.e2e.ts`: line-owner and device-placement repair in Chromium;
  both fail against the previous `Inspector.tsx` (negative control, first run).
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,155 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, five repository gates (headers 466 ok, ceiling 157; anchors 442 ok; ledger 185 rows), tsc, Vitest 1,817/100 files, complete intercepted Chromium suite 106/106, whitespace; source frozen.

## Boundaries

Duplicate ids are not renumbered; ambiguous building-part or group-range
placement has no repair command; a line listed twice by the same area is not
offered (it is not reached through two areas). Offline only, no KNX or bus
contact.
