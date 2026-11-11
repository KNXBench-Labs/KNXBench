/** Pending-programming sidebar list: order, chips, empty/unknown, selection and download jump. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { PendingProgramming } from "./bindings/PendingProgramming";

const locale = vi.hoisted(() => ({ value: "en" as "en" | "de" }));
vi.mock("./uiLanguage", () => ({ useUiLanguage: () => [locale.value] }));
vi.mock("./i18n", async (importOriginal) => {
  const original = await importOriginal<typeof import("./i18n")>();
  const translators = { en: original.translateFor.bind(null, "en"), de: original.translateFor.bind(null, "de") };
  return { ...original, useTranslate: () => translators[locale.value] };
});
import PendingProgrammingPanel, { PENDING_PANEL_LIMIT } from "./PendingProgrammingPanel";
import { DownloadNavigationProvider } from "./OpenDownloadButton";
import { pendingDevicesSummary } from "./pendingDevices";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

const state = (address: boolean, application: boolean, parameters: boolean, groupAddresses: boolean): PendingProgramming =>
  ({ address, application, parameters, group_addresses: groupAddresses });
const CLEAN = state(false, false, false, false);
const device = (id: number, address: string | null, name: string, pending?: PendingProgramming): DeviceNode =>
  ({ id, name, address, description: null, com_object_count: 0, ...(pending ? { pending_programming: pending } : {}) });

function tree(lineDevices: DeviceNode[], unassigned: DeviceNode[] = []): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "pending-panel", project_incarnation: 1, snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Line", address: 1, devices: lineDevices }] }], unassigned,
      buildings: [], group_ranges: [], group_addresses: [] }] };
}

const FIXTURE = tree([
  device(1, "1.1.10", "Dimmer ten", state(false, false, true, false)),
  device(2, "1.1.2", "Clean blind", CLEAN),
  device(3, "1.1.9", "Sensor nine", state(true, true, true, true)),
  device(4, "1.1.3", "Old server device"),
  device(5, "1.1.4", "Push button", state(true, false, true, false)),
], [device(6, null, "Unaddressed actuator", state(true, true, false, false))]);

let host: HTMLDivElement; let root: Root;
beforeEach(() => { locale.value = "en"; host = document.createElement("div"); document.body.append(host); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

async function render(project: ProjectTree, options: { selected?: number | null; onSelect?: (id: number) => void; onOpen?: (id: number) => void; onOpenAll?: () => void } = {}) {
  const summary = pendingDevicesSummary(project, locale.value);
  const panel = <PendingProgrammingPanel summary={summary} selectedDeviceId={options.selected ?? null} onSelect={options.onSelect ?? (() => {})} />;
  await act(async () => root.render(options.onOpen
    ? <DownloadNavigationProvider onOpen={options.onOpen} onOpenAll={options.onOpenAll}>{panel}</DownloadNavigationProvider> : panel));
}
const rows = () => [...host.querySelectorAll<HTMLLIElement>(".pending-programming-row")];
const rowOf = (id: number) => host.querySelector<HTMLLIElement>(`.pending-programming-row[data-device-id="${id}"]`)!;
const chipTexts = (id: number) => [...rowOf(id).querySelectorAll(".programming-chip")].map((chip) => chip.textContent);

it("lists only devices with a pending part, in the Devices table's address order, with their chips", async () => {
  await render(FIXTURE);
  // No address sorts first, as in the table; 1.1.10 after 1.1.9 (numeric collation).
  expect(rows().map((row) => row.dataset.deviceId)).toEqual(["6", "5", "3", "1"]);
  expect(rowOf(5).querySelector(".pending-programming-address")!.textContent).toBe("1.1.4");
  expect(rowOf(5).textContent).toContain("Push button");
  expect(chipTexts(5)).toEqual([en["programming.part.address.short"], en["programming.part.parameters.short"]]);
  expect(chipTexts(3)).toHaveLength(4);
  expect(rowOf(5).querySelector(".programming-chips")!.getAttribute("aria-label")).toBe("Not programmed: Address and Parameters");
  // A clean device and one whose state is not reported are never listed; the latter is counted honestly.
  expect(host.textContent).not.toContain("Clean blind");
  expect(host.textContent).not.toContain("Old server device");
  expect(host.querySelector(".pending-programming-unknown")!.textContent).toBe("Programming state not reported for 1 device.");
  expect(host.querySelector(".pending-programming-empty")).toBeNull();
});

it("says nothing is pending, and never guesses for unreported devices", async () => {
  await render(tree([device(1, "1.1.1", "Clean", CLEAN), device(2, "1.1.2", "Old a"), device(3, "1.1.3", "Old b")]));
  expect(rows()).toHaveLength(0);
  expect(host.querySelector(".pending-programming-empty")!.textContent).toBe(en["pendingPanel.empty"]);
  expect(host.querySelector(".pending-programming-unknown")!.textContent).toBe("Programming state not reported for 2 devices.");
  await render(tree([device(1, "1.1.1", "Clean", CLEAN)]));
  expect(host.querySelector(".pending-programming-unknown")).toBeNull();
});

it("selects a device by click, marks the selected row and does nothing else", async () => {
  const onSelect = vi.fn(); const onOpen = vi.fn();
  await render(FIXTURE, { selected: 3, onSelect, onOpen });
  expect(rowOf(3).querySelector(".pending-programming-open")!.getAttribute("aria-current")).toBe("true");
  expect(rowOf(5).querySelector(".pending-programming-open")!.hasAttribute("aria-current")).toBe(false);
  await act(async () => rowOf(5).querySelector<HTMLButtonElement>(".pending-programming-open")!.click());
  expect(onSelect).toHaveBeenCalledExactlyOnceWith(5);
  expect(onOpen).not.toHaveBeenCalled();
});

it("hands the device id to the workbench's download navigation, and only there", async () => {
  const onSelect = vi.fn(); const onOpen = vi.fn();
  await render(FIXTURE, { onSelect, onOpen });
  const download = rowOf(3).querySelector<HTMLButtonElement>(".pending-programming-download")!;
  expect(download.textContent).toBe(en["pendingPanel.download"]);
  expect(download.getAttribute("aria-label")).toBe("Download to 1.1.9 Sensor nine…");
  await act(async () => download.click());
  expect(onOpen).toHaveBeenCalledExactlyOnceWith(3);
  expect(onSelect).not.toHaveBeenCalled();
  // Without an individual address there is nothing to download to: disabled, with the reason.
  const blocked = rowOf(6).querySelector<HTMLButtonElement>(".pending-programming-download")!;
  expect(blocked.disabled).toBe(true);
  expect(host.querySelector(`#${CSS.escape(blocked.getAttribute("aria-describedby")!)}`)!.textContent).toBe(en["devices.downloadNeedsAddress"]);
});

it("offers no download button outside the workbench (no navigation provider)", async () => {
  await render(FIXTURE);
  expect(rows()).toHaveLength(4);
  expect(host.querySelector(".pending-programming-download")).toBeNull();
});

it("renders a bounded number of rows with an explicit count and Show all", async () => {
  const many = Array.from({ length: 120 }, (_, index) => device(index + 1, `1.1.${index + 1}`, `Device ${index + 1}`, state(false, false, true, false)));
  await render(tree(many));
  expect(rows()).toHaveLength(PENDING_PANEL_LIMIT);
  expect(rows().at(-1)!.dataset.deviceId).toBe(String(PENDING_PANEL_LIMIT));
  const more = host.querySelector(".pending-programming-more")!;
  expect(more.textContent).toContain(`Showing ${PENDING_PANEL_LIMIT} of 120`);
  await act(async () => more.querySelector<HTMLButtonElement>("button")!.click());
  expect(rows()).toHaveLength(120);
  expect(host.querySelector(".pending-programming-more")).toBeNull();
});

it("speaks German, with the glossary's programmieren for the device write", async () => {
  locale.value = "de";
  await render(FIXTURE, { onOpen: vi.fn() });
  const download = rowOf(3).querySelector<HTMLButtonElement>(".pending-programming-download")!;
  expect(download.textContent).toBe("Programmieren…");
  expect(download.getAttribute("aria-label")).toBe("1.1.9 Sensor nine programmieren…");
  expect(chipTexts(5)).toEqual([de["programming.part.address.short"], de["programming.part.parameters.short"]]);
  expect(host.querySelector(".pending-programming-unknown")!.textContent).toBe("Programmierstand für 1 Gerät nicht gemeldet.");
  expect(host.querySelector(".pending-programming-list")!.getAttribute("aria-label")).toBe("Geräte mit ausstehender Programmierung");
  await render(tree([]));
  expect(host.querySelector(".pending-programming-empty")!.textContent).toBe("Nichts ausstehend");
});

it("summarises a large project in one pass and keeps the table's pending filter", () => {
  const many = Array.from({ length: 5000 }, (_, index) => device(index + 1, `1.${Math.floor(index / 250) + 1}.${(index % 250) + 1}`,
    `Device ${index + 1}`, index % 3 === 0 ? undefined : index % 3 === 1 ? CLEAN : state(false, true, false, false)));
  const summary = pendingDevicesSummary(tree(many), "en");
  expect(summary.devices).toHaveLength(1666);
  expect(summary.unknown).toBe(1667);
  expect(summary.devices[0].address).toBe("1.1.3");
});

it("offers Download all… only with pending devices and a workbench, and it only hands off", async () => {
  const onOpenAll = vi.fn();
  await render(FIXTURE, { onOpen: () => {}, onOpenAll });
  const all = host.querySelector<HTMLButtonElement>("[data-pending-download-all]")!;
  expect(all.textContent).toBe(en["pendingPanel.downloadAll"]);
  await act(async () => all.click());
  expect(onOpenAll).toHaveBeenCalledTimes(1);
  locale.value = "de";
  await render(FIXTURE, { onOpen: () => {}, onOpenAll });
  expect(host.querySelector("[data-pending-download-all]")?.textContent).toBe(de["pendingPanel.downloadAll"]);
  await render(tree([device(1, "1.1.2", "Clean blind", CLEAN)]), { onOpen: () => {}, onOpenAll });
  expect(host.querySelector("[data-pending-download-all]")).toBeNull();
  await render(FIXTURE);
  expect(host.querySelector("[data-pending-download-all]")).toBeNull();
});
