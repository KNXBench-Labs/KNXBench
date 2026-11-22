/** Tests for the group-address views' last-download marks (rows, groups, de/en). */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import type { DownloadObjectDifference } from "./bindings/DownloadObjectDifference";
import type { Selection } from "./selection";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { differingAddresses, linkChangeList } from "./groupAddressLastDownload";
import { rawGroupAddress } from "./rawGroupAddress";
import { GA_LAST_DOWNLOAD_KEY } from "./deviceLastDownloads";

const api = vi.hoisted(() => ({ deviceDetail: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)) }));

import GroupAddressTable from "./GroupAddressTable";
import { useMultiSelection } from "./multiSelection";

let host: HTMLDivElement | undefined;
beforeEach(() => { resetSettingsForTests(); settingsStorage.setItem("uiLanguage", "en"); settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, "1"); resetUiLanguageForTests(); });
afterEach(() => { host?.remove(); host = undefined; vi.clearAllMocks(); });

// 1/0/1 = 0x0801, 1/0/2 = 0x0802, 1/0/3 = 0x0803, 2/0/1 = 0x1001.
const link = (device_id: number): GroupAddressNode["links"][number] =>
  ({ device_id, device_name: null, device_address: null, com_object_id: device_id * 10, com_object_number: 10, com_object_name: null, direction: "Send" });
const ga = (id: number, address: string, name: string, range: number | null, devices: number[] = []): GroupAddressNode =>
  ({ id, address, name, range, dpts: [], links: devices.map(link) });
const addresses: GroupAddressNode[] = [
  ga(30, "1/0/1", "Kitchen ceiling", 21, [1]),
  ga(31, "1/0/2", "Kitchen dimming", 21, [1]),
  ga(32, "1/0/3", "Hallway", 20),
  ga(33, "2/0/1", "Unassigned spare", null),
];
function installation(): InstallationNode {
  return { id: 0, name: "Example installation", topology: [
      { id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Line", address: 1,
        devices: [{ id: 1, name: "Fictional dimmer", address: "1.1.1", description: null, com_object_count: 2 }] }] }],
    buildings: [], unassigned: [], group_addresses: addresses,
    group_ranges: [
      { id: 20, name: "Lighting", start: "1/0/0", end: "1/7/255", parent: null },
      { id: 21, name: "Ground floor", start: "1/0/0", end: "1/0/255", parent: 20 },
    ] };
}
const treeOf = (inst: InstallationNode): ProjectTree => ({
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", installations: [inst],
});

const at = "2026-11-12T09:30:00.000Z";
// Device 1's object 10: added link to 1/0/1, removed link to 1/0/2, redirected 2/0/1; object 11: flags only (no address).
const added: DownloadObjectDifference = { number: 10, instance_id: 10, links: [
  { group_address: 0x0801, downloaded: null, current: true }], flags: null };
const removed: DownloadObjectDifference = { number: 10, instance_id: 10, links: [
  { group_address: 0x0802, downloaded: true, current: null }], flags: null };
const redirected: DownloadObjectDifference = { number: 10, instance_id: 10, links: [
  { group_address: 0x1001, downloaded: false, current: true }], flags: null };
const flagsOnly: DownloadObjectDifference = { number: 11, instance_id: 11, links: [], flags: {
  downloaded: { read: true, write: true, transmit: false, update: false, communication: true, read_on_init: null },
  current: { read: false, write: true, transmit: false, update: false, communication: true, read_on_init: null } } };
const usable: ObjectLastDownload = { links: { state: "usable", at }, flags: { state: "usable", at },
  differing: [added, removed, redirected, flagsOnly], instances: { 10: added, 11: flagsOnly },
  differing_objects: 2, no_selected_row: 0 };

function detail(lastDownload?: ObjectLastDownload): DeviceDetail {
  return { id: 1, name: "Fictional dimmer", description: null, address: "1.1.1", com_objects: [],
    product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null }, last_download: lastDownload };
}

function Harness(props: { inst?: InstallationNode; language?: "en" | "de" }) {
  const inst = props.inst ?? installation();
  const [selection, setSelection] = useState<Selection | null>(null);
  const { multiSelection, onItemClick } = useMultiSelection(treeOf(inst), setSelection);
  return <GroupAddressTable installation={inst} tree={treeOf(inst)} selection={selection} multiSelection={multiSelection}
    onItemClick={onItemClick} onTreeUpdate={() => {}} rangeScope={null} />;
}

async function render(lastDownload: ObjectLastDownload | undefined): Promise<Root> {
  api.deviceDetail.mockImplementation(async (id: number) => detail(id === 1 ? lastDownload : undefined));
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => { root.render(<Harness />); });
  // Wait for the device-detail reads to settle.
  for (let i = 0; i < 20; i += 1) await act(async () => { await Promise.resolve(); });
  const expandAll = host.querySelector<HTMLButtonElement>("[data-ga-expand-all]");
  if (expandAll) await act(async () => { expandAll.click(); });
  return root;
}

function row(address: string): HTMLTableRowElement {
  const found = [...host!.querySelectorAll<HTMLTableRowElement>("tbody tr[data-rename-kind='group_address']")].find(tr =>
    tr.querySelector(".mono")?.textContent?.trim().startsWith(address));
  if (!found) throw new Error(`no row for ${address}`);
  return found;
}
const markIn = (el: Element | null) => el?.querySelector(".ga-download-mark") ?? null;
const description = (el: Element) => (el.getAttribute("aria-describedby") ?? "").split(" ").filter(Boolean)
  .map(id => host!.ownerDocument.getElementById(id)?.textContent ?? "").join(" ");

describe("derivation", () => {
  it("maps raw addresses of added, removed and redirected sending links; flags-only differences name no address", () => {
    const map = differingAddresses(usable);
    expect([...map.keys()].sort((a, b) => a - b)).toEqual([0x0801, 0x0802, 0x1001]);
    expect(map.get(0x0801)).toEqual([{ kind: "added", address: 0x0801, sending: true }]);
    expect(map.get(0x0802)).toEqual([{ kind: "removed", address: 0x0802, sending: true }]);
    expect(map.get(0x1001)).toEqual([{ kind: "redirected", address: 0x1001, downloaded: false, current: true }]);
    expect(linkChangeList([])).toEqual([]);
    expect(differingAddresses(undefined).size).toBe(0);
    expect(differingAddresses({ ...usable, links: { state: "absent" } }).size).toBe(0);
    expect(differingAddresses({ ...usable, links: { state: "currentUnreadable" } }).size).toBe(0);
  });

  it("parses projected addresses back to raw values in all three styles", () => {
    expect(rawGroupAddress("1/2/3", "ThreeLevel")).toBe(0x0a03);
    expect(rawGroupAddress("1/515", "TwoLevel")).toBe(0x0a03);
    expect(rawGroupAddress("2563", "Free")).toBe(2563);
    expect(rawGroupAddress("nonsense", "ThreeLevel")).toBeNull();
    expect(rawGroupAddress("1/8/1", "ThreeLevel")).toBeNull();
  });
});

describe("tree view marks", () => {
  it("marks exactly the differing addresses, with dot, caption and unchanged accessible names", async () => {
    const root = await render(usable);
    for (const [address, key] of [["1/0/1", "gaWorkspace.lastDownload.row"], ["1/0/2", "gaWorkspace.lastDownload.row"], ["2/0/1", "gaWorkspace.lastDownload.row"]] as const) {
      const r = row(address);
      expect(r.dataset.differsFromDownload, address).toBe("true");
      const mark = markIn(r);
      expect(mark?.getAttribute("aria-hidden")).toBe("true");
      expect(mark?.getAttribute("title")).toBe(en[key]);
      expect(r.querySelector(".ga-download-note")?.textContent).toBe(en[key]);
      const button = r.querySelector<HTMLButtonElement>("[data-ga-select]")!;
      expect(button.getAttribute("aria-label")).toBeNull();
      expect(description(button)).toBe(en[key]);
    }
    const clean = row("1/0/3");
    expect(clean.dataset.differsFromDownload).toBeUndefined();
    expect(markIn(clean)).toBeNull();
    expect(clean.querySelector(".ga-download-note")).toBeNull();
    expect(clean.querySelector("[data-ga-select]")!.hasAttribute("aria-describedby")).toBe(false);
    await act(async () => root.unmount());
  });

  it("marks the groups holding differing rows (including the ancestor), also while collapsed", async () => {
    const root = await render(usable);
    const expand = (key: string) => host!.querySelector<HTMLButtonElement>(`[data-ga-expand="${key}"]`)!;
    const lighting = expand("range:20:0"), floor = expand("range:21:0");
    expect(lighting.dataset.differsFromDownload).toBe("true");
    expect(floor.dataset.differsFromDownload).toBe("true");
    expect(description(lighting)).toBe(en["gaWorkspace.lastDownload.group"]);
    expect(markIn(lighting)).not.toBeNull();
    // Collapse both: the ancestor still shows that it holds a differing address.
    await act(async () => { lighting.click(); });
    await act(async () => { floor.click(); });
    expect(lighting.dataset.differsFromDownload).toBe("true");
    expect(markIn(lighting)).not.toBeNull();
    // The unassigned pseudo-group holds 2/0/1 and differs too; nothing else does.
    const unassigned = expand("unassigned");
    expect(unassigned.dataset.differsFromDownload).toBe("true");
    await act(async () => root.unmount());
  });

  it("summary counts differing addresses, hidden ones and the compare time; static text", async () => {
    const root = await render(usable);
    const summary = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
    expect(summary.getAttribute("data-links")).toBe("usable");
    expect(summary.getAttribute("data-differs")).toBe("true");
    expect(summary.querySelector(".ga-last-download-dot")).not.toBeNull();
    const time = summary.textContent!.match(/\(([^)]+)\)/)![1];
    expect(summary.textContent).toBe(`${en["gaWorkspace.lastDownload.differs.other"].replace("{count}", "3")} ${
      en["comTable.lastDownload.links.usable"].replace("{time}", time)}`);
    // A query that hides every differing address adds the hidden count.
    const input = host!.querySelector<HTMLInputElement>("input.address-filter")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "hallway");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    const filtered = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
    expect(filtered.textContent).toContain(en["gaWorkspace.lastDownload.hidden.other"].replace("{count}", "3"));
    await act(async () => root.unmount());
  });

  it("no baseline (absent) shows the state sentence and no marks; empty usable comparison says nothing differs", async () => {
    let root = await render({ ...usable, links: { state: "absent" } });
    let summary = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
    expect(summary.getAttribute("data-links")).toBe("absent");
    expect(summary.getAttribute("data-differs")).toBeNull();
    expect(summary.textContent).toBe(en["comTable.lastDownload.links.absent"]);
    expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
    await act(async () => root.unmount());

    root = await render({ ...usable, differing: [], instances: {}, differing_objects: 0 });
    summary = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
    expect(summary.getAttribute("data-differs")).toBeNull();
    expect(summary.querySelector(".ga-last-download-dot")).toBeNull();
    const time = summary.textContent!.match(/\(([^)]+)\)/)![1];
    expect(summary.textContent).toBe(`${en["gaWorkspace.lastDownload.noneDiffer"]} ${
      en["comTable.lastDownload.links.usable"].replace("{time}", time)}`);
    expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
    await act(async () => root.unmount());
  });

  it("opt-out: without the preference no device detail is read and nothing is marked", async () => {
    settingsStorage.removeItem(GA_LAST_DOWNLOAD_KEY);
    const root = await render(usable);
    expect(api.deviceDetail).not.toHaveBeenCalled();
    expect(host!.querySelector(".ga-last-download")).toBeNull();
    expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
    await act(async () => root.unmount());
  });

  it("an older server without last_download and failed reads give no marks and say nothing was compared", async () => {
    let root = await render(undefined);
    expect(host!.querySelector(".ga-last-download")!.textContent).toBe(
      `${en["gaWorkspace.lastDownload.coverage.one"].replace("{compared}", "0").replace("{total}", "1")} ${
        en["gaWorkspace.lastDownload.notCompared"].replace("{list}", en["gaWorkspace.lastDownload.reason.failed"].replace("{count}", "1"))}`);
    expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
    await act(async () => root.unmount());

    api.deviceDetail.mockRejectedValue(new Error("boom"));
    host = document.createElement("div");
    document.body.appendChild(host);
    const root2 = createRoot(host);
    await act(async () => { root2.render(<Harness />); });
    for (let i = 0; i < 20; i += 1) await act(async () => { await Promise.resolve(); });
    const summary = host!.querySelector(".ga-last-download")!;
    expect(summary.getAttribute("data-coverage")).toBe("partial");
    expect(summary.textContent).not.toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
    await act(async () => root2.unmount());
  });

  it("German copy for row, group and summary", async () => {
    settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
    const root = await render(usable);
    expect(markIn(row("1/0/1"))?.getAttribute("title")).toBe(de["gaWorkspace.lastDownload.row"]);
    expect(row("1/0/1").querySelector(".ga-download-note")?.textContent).toBe(de["gaWorkspace.lastDownload.row"]);
    const lighting = host!.querySelector<HTMLButtonElement>(`[data-ga-expand="range:20:0"]`)!;
    expect(description(lighting)).toBe(de["gaWorkspace.lastDownload.group"]);
    const summary = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
    expect(summary.textContent).toContain(de["gaWorkspace.lastDownload.differs.other"].replace("{count}", "3"));
    await act(async () => root.unmount());
  });

  it("unusable links states other than absent also mark nothing and report their sentence", async () => {
    for (const state of ["otherProgram", "productUnavailable", "currentUnreadable"] as const) {
      const root = await render({ ...usable, links: { state } });
      const summary = host!.querySelector<HTMLParagraphElement>(".ga-last-download")!;
      expect(summary.getAttribute("data-links")).toBe(state);
      expect(summary.textContent).toBe(en[`comTable.lastDownload.links.${state}` as keyof typeof en]);
      expect(host!.querySelectorAll(".ga-download-mark").length).toBe(0);
      await act(async () => root.unmount());
    }
  });
});

describe("columns and atlas views", () => {
  const setView = async (view: string) => {
    const select = host!.querySelector<HTMLSelectElement>("select[data-ga-view]")!;
    expect(select).not.toBeNull();
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, view);
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    for (let i = 0; i < 5; i += 1) await act(async () => { await Promise.resolve(); });
  };

  it("columns: differing group buttons carry the dot and description; rows via the flat table", async () => {
    const root = await render(usable);
    await setView("columns");
    expect(host!.querySelector("[data-ga-columns]")).not.toBeNull();
    const buttons = [...host!.querySelectorAll<HTMLButtonElement>(".ga-column [data-ga-scope]")];
    const keys = buttons.map(b => b.dataset.gaScope);
    expect(keys).toContain("range:20:0");
    const lighting = buttons.find(b => b.dataset.gaScope === "range:20:0")!;
    // Drill into Lighting: the next column offers Ground floor.
    await act(async () => { lighting.click(); });
    for (let i = 0; i < 5; i += 1) await act(async () => { await Promise.resolve(); });
    const all = [...host!.querySelectorAll<HTMLButtonElement>(".ga-column [data-ga-scope]")];
    const floor = all.find(b => b.dataset.gaScope === "range:21:0")!;
    expect(floor.dataset.differsFromDownload).toBe("true");
    expect(lighting.dataset.differsFromDownload).toBe("true");
    expect(description(floor)).toBe(en["gaWorkspace.lastDownload.group"]);
    expect(markIn(floor)).not.toBeNull();
    expect(row("1/0/1").dataset.differsFromDownload).toBe("true");
    expect(row("1/0/3").dataset.differsFromDownload).toBeUndefined();
    await act(async () => root.unmount());
  });

  it("atlas: range cards and address cards carry marks and descriptions", async () => {
    const root = await render(usable);
    await setView("atlas");
    // Drill into Lighting: its card list shows the child range; scope to it.
    const lighting = host!.querySelector<HTMLButtonElement>(`[data-ga-scope="range:20:0"]`)!;
    expect(lighting.dataset.differsFromDownload).toBe("true");
    expect(description(lighting)).toBe(en["gaWorkspace.lastDownload.group"]);
    await act(async () => { lighting.click(); });
    const floor = host!.querySelector<HTMLButtonElement>(`[data-ga-scope="range:21:0"]`)!;
    expect(floor.dataset.differsFromDownload).toBe("true");
    await act(async () => { floor.click(); });
    const card = [...host!.querySelectorAll<HTMLButtonElement>("[data-ga-atlas] [data-ga-select]")]
      .find(b => b.querySelector(".mono")?.textContent?.trim() === "1/0/1")!;
    expect(card.closest("li")!.dataset.differsFromDownload).toBe("true");
    expect(markIn(card)).not.toBeNull();
    expect(description(card)).toBe(en["gaWorkspace.lastDownload.row"]);
    const clean = [...host!.querySelectorAll<HTMLButtonElement>("[data-ga-atlas] [data-ga-select]")]
      .find(b => b.querySelector(".mono")?.textContent?.trim() === "2/0/1");
    // 2/0/1 is unassigned and not in the Ground floor scope; scope back to all to check it.
    const allButton = host!.querySelector<HTMLButtonElement>(".ga-breadcrumb button");
    if (allButton) await act(async () => { allButton.click(); });
    await act(async () => { host!.querySelector<HTMLButtonElement>(`[data-ga-scope="unassigned"]`)!.click(); });
    const spare = [...host!.querySelectorAll<HTMLButtonElement>("[data-ga-atlas] [data-ga-select]")]
      .find(b => b.querySelector(".mono")?.textContent?.trim() === "2/0/1")!;
    expect(spare.closest("li")!.dataset.differsFromDownload).toBe("true");
    void clean;
    await act(async () => root.unmount());
  });
});

// Two devices, both linking addresses; replies controlled per device.
describe("summary coverage", () => {
  const twoDevices = (): InstallationNode => {
    const inst = installation();
    inst.topology[0]!.lines[0]!.devices.push({ id: 2, name: "Fictional switch", address: "1.1.2", description: null, com_object_count: 1 });
    inst.group_addresses = [
      ga(30, "1/0/1", "Kitchen ceiling", 21, [1, 2]), ga(31, "1/0/2", "Kitchen dimming", 21, [1]),
      ga(32, "1/0/3", "Hallway", 20, [2]), ga(33, "2/0/1", "Unassigned spare", null)];
    return inst;
  };
  const noneDiffer: ObjectLastDownload = { ...usable, differing: [], instances: {}, differing_objects: 0 };
  const settle = async () => { for (let i = 0; i < 20; i += 1) await act(async () => { await Promise.resolve(); }); };
  const summaryText = () => host!.querySelector(".ga-last-download")?.textContent ?? null;
  const reason = (key: string, count = 1) => en[`gaWorkspace.lastDownload.reason.${key}` as keyof typeof en].replace("{count}", String(count));
  const mount = async () => {
    host = document.createElement("div"); document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => { root.render(<Harness inst={twoDevices()} />); });
    return root;
  };
  const deferred = () => { let resolve!: (d: DeviceDetail) => void; let reject!: (e: unknown) => void;
    const promise = new Promise<DeviceDetail>((res, rej) => { resolve = res; reject = rej; }); return { promise, resolve, reject }; };
  const detailOf = (id: number, lastDownload: ObjectLastDownload): DeviceDetail => ({ ...detail(lastDownload), id });

  it("one usable no-difference device beside an absent one is not an all-clear", async () => {
    api.deviceDetail.mockImplementation(async (id: number) => detailOf(id, id === 1 ? noneDiffer : { ...noneDiffer, links: { state: "absent" } }));
    const root = await mount(); await settle();
    const summary = host!.querySelector(".ga-last-download")!;
    expect(summary.getAttribute("data-coverage")).toBe("partial");
    expect(summary.getAttribute("data-compared")).toBe("1");
    expect(summary.getAttribute("data-total")).toBe("2");
    expect(summary.textContent).toContain(en["gaWorkspace.lastDownload.coverage.other"].replace("{compared}", "1").replace("{total}", "2"));
    expect(summary.textContent).toContain(reason("absent"));
    expect(summary.textContent).toContain(en["gaWorkspace.lastDownload.noneDifferCompared"]);
    expect(summary.textContent).not.toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    await act(async () => root.unmount());
  });

  it("every device usable and clean says so installation-wide", async () => {
    api.deviceDetail.mockImplementation(async (id: number) => detailOf(id, noneDiffer));
    const root = await mount(); await settle();
    const summary = host!.querySelector(".ga-last-download")!;
    expect(summary.getAttribute("data-coverage")).toBe("complete");
    expect(summary.textContent).toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    expect(summary.textContent).not.toContain(en["gaWorkspace.lastDownload.coverage.other"].replace("{compared}", "2").replace("{total}", "2"));
    await act(async () => root.unmount());
  });

  it("a delayed reply keeps the summary partial until it arrives, then completes it", async () => {
    const slow = deferred();
    api.deviceDetail.mockImplementation((id: number) => id === 1 ? Promise.resolve(detailOf(1, noneDiffer)) : slow.promise);
    const root = await mount(); await settle();
    expect(summaryText()).toContain(reason("loading"));
    expect(summaryText()).not.toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    await act(async () => { slow.resolve(detailOf(2, noneDiffer)); }); await settle();
    expect(host!.querySelector(".ga-last-download")!.getAttribute("data-coverage")).toBe("complete");
    expect(summaryText()).toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    await act(async () => root.unmount());
  });

  it("a failed read stays partial and names the failure; other states are told apart", async () => {
    api.deviceDetail.mockImplementation(async (id: number) => {
      if (id === 2) throw new Error("boom");
      return detailOf(1, { ...noneDiffer, links: { state: "otherProgram" } });
    });
    let root = await mount(); await settle();
    expect(summaryText()).toContain(reason("failed"));
    expect(summaryText()).toContain(reason("otherProgram"));
    expect(summaryText()).toContain(en["gaWorkspace.lastDownload.coverage.other"].replace("{compared}", "0").replace("{total}", "2"));
    expect(summaryText()).not.toContain(en["gaWorkspace.lastDownload.noneDiffer"]);
    await act(async () => root.unmount());
    host!.remove();
    api.deviceDetail.mockImplementation(async (id: number) =>
      detailOf(id, { ...noneDiffer, links: { state: id === 1 ? "productUnavailable" : "currentUnreadable" } }));
    root = await mount(); await settle();
    expect(summaryText()).toContain(reason("productUnavailable"));
    expect(summaryText()).toContain(reason("currentUnreadable"));
    await act(async () => root.unmount());
  });

  it("marks from the compared device stay while the other is unknown; a device linked in several rows counts once", async () => {
    api.deviceDetail.mockImplementation(async (id: number) =>
      id === 1 ? detailOf(1, usable) : detailOf(2, { ...usable, links: { state: "absent" } }));
    const root = await mount(); await settle();
    const summary = host!.querySelector(".ga-last-download")!;
    expect(summary.getAttribute("data-total")).toBe("2"); // device 1 links two addresses, device 2 two, both counted once
    expect(summary.textContent).toContain(en["gaWorkspace.lastDownload.differs.other"].replace("{count}", "3"));
    expect(summary.getAttribute("data-differs")).toBe("true");
    await act(async () => root.unmount());
  });

  it("German coverage copy", async () => {
    settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
    api.deviceDetail.mockImplementation(async (id: number) => detailOf(id, id === 1 ? noneDiffer : { ...noneDiffer, links: { state: "absent" } }));
    const root = await mount(); await settle();
    expect(summaryText()).toContain(de["gaWorkspace.lastDownload.coverage.other"].replace("{compared}", "1").replace("{total}", "2"));
    expect(summaryText()).toContain(de["gaWorkspace.lastDownload.reason.absent"].replace("{count}", "1"));
    await act(async () => root.unmount());
  });
});
