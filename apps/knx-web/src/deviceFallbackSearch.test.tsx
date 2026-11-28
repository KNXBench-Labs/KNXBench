/** Blank-named devices are found by their fallback label in every raw-name filter (WP-51.3d). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it } from "vitest";
import GroupAddressMatrix from "./GroupAddressMatrix";
import GroupAddressObjectInventoryList from "./GroupAddressObjectInventoryList";
import type { GroupAddressObjectInventory } from "./groupAddressObjectInventory";
import type { GroupAddressMatrixColumn } from "./groupAddressMatrix";
import type { DeviceNode } from "./bindings/DeviceNode";
import { settingsStorage, resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { projectDeviceRows, visibleDeviceRows } from "./deviceList";
import type { ProjectTree } from "./bindings/ProjectTree";
import { translateFor } from "./i18n";
import { deviceLabel } from "./devicePresentation";

let host: HTMLDivElement; let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  resetSettingsForTests(); resetUiLanguageForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); resetSettingsForTests(); resetUiLanguageForTests(); });

const node = (id: number, name: string): DeviceNode => ({ id, name, address: `1.1.${id}`, description: null, com_object_count: 1 });
const inventory = (devices: DeviceNode[]) => ({ devices, entries: new Map(), source: {}, current: new Map(),
  summary: { total: 0, notLoaded: 0, loading: 0, loaded: 0, failed: 0 }, pendingKeys: new Set(), load: () => {} }) as unknown as GroupAddressObjectInventory;
const ids = () => [...host.querySelectorAll("[data-ga-inventory-device]")].map(li => li.getAttribute("data-ga-inventory-device"));

it.each([["en", "unnamed"], ["de", "unbenannt"], ["en", "id 7"], ["de", "id 7"]])("inventory list finds a blank device (%s, %s) and keeps nonblank behaviour", async (language, query) => {
  settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests();
  const devices = [node(7, "  "), node(8, "Dimmer")];
  await act(async () => root.render(<GroupAddressObjectInventoryList inventory={inventory(devices)} query={query} />));
  expect(ids()).toEqual(["7"]);
  await act(async () => root.render(<GroupAddressObjectInventoryList inventory={inventory(devices)} query="dimmer" />));
  expect(ids()).toEqual(["8"]);
  await act(async () => root.render(<GroupAddressObjectInventoryList inventory={inventory(devices)} query="1.1.8" />));
  expect(ids()).toEqual(["8"]);
});

const column = (deviceId: number, deviceName: string | null): GroupAddressMatrixColumn =>
  ({ key: `c${deviceId}`, deviceId, objectId: deviceId * 10, deviceName, deviceAddress: `1.1.${deviceId}`, objectName: "Switch", objectNumber: 0, linked: true });
const choices = () => [...host.querySelectorAll("[data-ga-column]")].map(i => i.getAttribute("data-ga-column"));
async function renderMatrix(query: string) {
  const matrix = { scope: "linked-objects", columns: [column(7, ""), column(8, "Dimmer"), column(9, null)], cells: new Map(), ambiguousObjectIds: [], missingFromInventory: [] };
  const props = { matrix, inventory: inventory([]), columnKeys: null, onColumns: () => {}, columnQuery: query, onColumnQuery: () => {}, rows: [], onLimit: () => {} };
  await act(async () => root.render(<GroupAddressMatrix {...(props as unknown as Parameters<typeof GroupAddressMatrix>[0])} />));
}
it.each([["en", "Unnamed"], ["en", "ID 7"], ["de", "Unbenanntes"], ["de", "ID 7"]])("matrix column filter finds a blank device (%s, %s)", async (language, query) => {
  settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests();
  await renderMatrix(query);
  expect(choices()).toEqual(["c7"]);
});
it("matrix column filter keeps raw-name matching and the unknown-device column", async () => {
  await renderMatrix("dimmer"); expect(choices()).toEqual(["c8"]);
  await renderMatrix(""); expect(choices()).toEqual(["c7", "c8", "c9"]);
  await renderMatrix("Unnamed"); expect(choices()).toEqual(["c7"]);
});

const label = (language: "en" | "de") => (row: { id: number; name: string }) => deviceLabel((k, p) => translateFor(language, k, p), row);
it.each([["en", "Unnamed"], ["en", "ID 2"], ["de", "Unbenanntes"], ["de", "ID 2"]] as const)("device list search finds a blank device (%s, %s)", (language, query) => {
  const tree = { schema_version: 11, errors: 0, warnings: 0, is_modified: false, can_undo: false, can_redo: false,
    group_address_style: "ThreeLevel", server_incarnation: "t", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", unassigned: [node(10, "Device 10")], group_addresses: [], group_ranges: [], buildings: [],
      topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 1, name: "Line", address: 1, devices: [node(2, " ")] }] }] }] } as ProjectTree;
  const rows = projectDeviceRows(tree);
  const found = (q: string, lang: "en" | "de" = language) => visibleDeviceRows(rows, q, { column: "address", descending: false }, lang, { label: label(lang) }).map(r => r.id);
  expect(found(query)).toEqual([2]);
  expect(found("Device 10")).toEqual([10]);
  expect(visibleDeviceRows(rows, query, { column: "address", descending: false }, language)).toEqual([]);
  expect(rows.find(r => r.id === 2)!.name).toBe(" ");
});
