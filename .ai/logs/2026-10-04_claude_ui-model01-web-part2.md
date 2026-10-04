# 2026-10-04 — MODEL-01 web half, part 2: Inspector, bulk toolbar, CSV

Agent: Claude, goal-ui.md owner session, under the Web lock taken for MODEL-01
(`babdbdc1`); part 1 was published as `0dd9add8`. The status-docs lock was
already released, so the ledger change is in `docs/status/LEDGER.md` directly.

## Rules mirrored from the core (ADR-0070)

- Id-addressed commands act in the one installation holding the entity; ids
  in several installations are refused.
- `LinkComObject` (`command.rs:3196-3212`): a device with a topology
  installation links only to group addresses of that installation; a device
  placed nowhere may link to any; one placed in two installations is refused.
- CSV import: the server binds the installation into the confirmation token
  (`domain.rs:1258-1262`), so a preview cannot confirm another installation.

## Change

- `Inspector.tsx`: gates use `owningInstallation` / `deviceInstallation`. The
  line, part, area, parent and link lists come from the owning installation;
  `linkableGroupAddresses` mirrors `LinkComObject`. The message key is now
  `inspector.restrictedToOneInstallation` (en/de).
- `treeUtils.ts`: `findDeviceLine` / `findDeviceBuildingPart` scoped to the
  device's installation.
- `BulkActionToolbar.tsx`: targets from the single owning installation of the
  selection, otherwise the hint `bulkAction.oneInstallation`.
- `GroupAddressCsvButtons.tsx` + `api.ts`: an installation choice when there
  are several; `installationId` on export, preview and confirmation.
- `e2e/installations-fixture.tsx`: Explorer clicks now drive the Inspector
  selection.

## Evidence

- RED: 7 Inspector cases, 1 DeviceWorkspace link-list case (the list showed
  installation 1's address), 5 CSV/bulk/api cases; the guard "no choice with a
  single installation" already held.
- GREEN surfaced 6 older tests that pinned the first-installation-only rule.
  Each was read: all used a uniquely owned later-installation entity (the
  genuinely ambiguous-id tests still pass unchanged). The 5 Inspector tests
  were converted to assert the new rule, and the `findDeviceBuildingPart`
  fixture got a topology placement plus a new later-installation case. One
  converted test queued a `…Once` answer for both rename mocks and leaked into
  the next test; it now queues only the one it consumes.
- Controls: the line-rename e2e fails against the previous `Inspector.tsx`.
  7 mutants caught by name: GA delete first-only, device line list, line area
  list, part parent list, links from every installation, CSV confirmation
  without installation, bulk move across installations.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,155 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, four repository gates (headers 465 ok, ceiling 157; anchors 442 ok) plus `check-ledger` (185 rows), tsc, Vitest 1,803/100 files, complete intercepted Chromium suite 104/104, whitespace; source frozen.
