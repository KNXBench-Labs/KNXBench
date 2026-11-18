/** Pure column-model helpers for the catalogue and device tables. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { CatalogSortColumn } from "./catalogTableSort";
import type { DeviceSortColumn } from "./deviceList";
import type { MessageKey } from "./i18n";

/** One selectable, sortable column of the catalogue results table. */
export interface CatalogTableColumn {
  id: string;
  sort: CatalogSortColumn;
  label: MessageKey;
  width: number;
  min: number;
}

/** The catalogue table's full declared column set, in display order. */
export const CATALOG_TABLE_COLUMNS: readonly CatalogTableColumn[] = [
  { id: "product", sort: "product", label: "catalogTable.product", width: 220, min: 96 },
  { id: "manufacturer", sort: "manufacturer", label: "catalogTable.manufacturer", width: 140, min: 72 },
  { id: "number", sort: "number", label: "catalogTable.number", width: 120, min: 64 },
  { id: "application", sort: "application", label: "catalogTable.application", width: 150, min: 72 },
  { id: "section", sort: "section", label: "catalogTable.section", width: 140, min: 72 },
  { id: "mask", sort: "mask", label: "catalogTable.mask", width: 90, min: 56 },
  { id: "medium", sort: "medium", label: "catalogTable.medium", width: 110, min: 64 },
];

/** One selectable, sortable column of the project device table. */
export interface DeviceTableColumn {
  id: string;
  sort: DeviceSortColumn | null;
  label: MessageKey;
  width: number;
  min: number;
}

/**
 * The device table's full declared column set, in display order. `select`
 * and `comparison` are fixed action columns: they are not sortable and not
 * part of the chooser, but they are still resizable like every other column.
 */
export const DEVICE_TABLE_COLUMNS: readonly DeviceTableColumn[] = [
  { id: "address", sort: "address", label: "workbench.address", width: 82, min: 48 },
  { id: "name", sort: "name", label: "workbench.name", width: 120, min: 48 },
  { id: "programming", sort: "programming", label: "devices.programming", width: 100, min: 48 },
  { id: "manufacturer", sort: "manufacturer", label: "devices.manufacturer", width: 90, min: 48 },
  { id: "product", sort: "product", label: "devices.product", width: 110, min: 48 },
  { id: "orderNumber", sort: "orderNumber", label: "devices.orderNumber", width: 80, min: 48 },
  { id: "system", sort: "system", label: "devices.system", width: 80, min: 48 },
  { id: "topology", sort: "topology", label: "devices.topology", width: 80, min: 48 },
  { id: "building", sort: "building", label: "devices.building", width: 90, min: 48 },
  { id: "com_object_count", sort: "com_object_count", label: "inspector.communicationObjects", width: 80, min: 48 },
  { id: "description", sort: "description", label: "devices.description", width: 100, min: 48 },
];

/** The fixed selection column, always first and never hideable. */
export const DEVICE_SELECT_COLUMN = { id: "select", width: 48, min: 32 };
/** The fixed comparison column, always last and never hideable. */
export const DEVICE_COMPARISON_COLUMN = { id: "comparison", width: 102, min: 48 };

/** Display-ordered visible subset of a declared column list. */
export function visibleColumns<T extends { id: string }>(columns: readonly T[], visible: Record<string, boolean>): T[] {
  return columns.filter((column) => visible[column.id] !== false);
}
