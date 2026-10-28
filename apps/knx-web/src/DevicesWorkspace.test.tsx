/** Device table gestures, preserved view state and asynchronous metadata guards. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import DevicesWorkspace from "./DevicesWorkspace";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceCatalog } from "./deviceList";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { publishSystem2Status, resetSystem2StatusForTests } from "./system2Status";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ deviceCatalog: vi.fn(), compareDevice: vi.fn() }));
const system2 = vi.hoisted(() => ({ getSystem2Status: vi.fn() }));
vi.mock("./system2DownloadApi", async (importOriginal) => ({ ...await importOriginal<typeof import("./system2DownloadApi")>(), ...system2 }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error) }));
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "table-test", snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [], buildings: [], group_addresses: [], group_ranges: [],
    unassigned: [2, 10].map((id) => ({ id, name: `Device ${id}`, address: `1.1.${id}`, description: null, com_object_count: 0 })) }] };
const batch: DeviceCatalog = { schemaVersion: 1, serverIncarnation: "table-test", snapshotRevision: 1, devices: [] };
let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const click = vi.fn(); const catalogue = vi.fn();
beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  apiMock.deviceCatalog.mockReset(); apiMock.deviceCatalog.mockResolvedValue(batch);
  apiMock.compareDevice.mockReset();
  resetSystem2StatusForTests(); system2.getSystem2Status.mockReset(); system2.getSystem2Status.mockResolvedValue(null);
  click.mockClear(); catalogue.mockClear();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals();
  resetSettingsForTests(); resetProductLanguageForTests(); resetUiLanguageForTests(); window.localStorage.clear();
});
async function render(active = true, project = tree) {
  await act(async () => root.render(<div hidden={!active}><DevicesWorkspace tree={project} active={active}
    selection={null} multiSelection={{ kind: "device", ids: new Set([2]) }} onItemClick={click} onCatalogue={catalogue} /></div>));
}
async function query(value: string) {
  const input = host.querySelector<HTMLInputElement>('input[type="search"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true })); });
}
it.each(["en", "de"] as const)("%s compact headings retain full sort names and tooltips", async language => {
  await act(async () => setSetting("uiLanguage", language));
  await render();
  const full = language === "de" ? "Hersteller" : "Manufacturer";
  const header = host.querySelectorAll("thead th")[4];
  const button = header.querySelector("button")!;
  expect(button.getAttribute("aria-label")).toBe(full);
  expect(button.title).toBe(full);
  expect(button.querySelector(".table-column-label")!.textContent).toBe(language === "de" ? "Herst." : "Mfr.");
  expect(header.querySelector("span")!.title).toBe(full);
});
it.each(["en", "de"] as const)("%s compact location, topology and programming labels keep their meaning", async language => {
  await act(async () => setSetting("uiLanguage", language));
  await render();
  const headings = host.querySelectorAll("thead th");
  for (const [index, label, full] of language === "de"
    ? [[3, "Prog.", "Programmierung"], [8, "Topologie", "Installation / KNX-Linie"], [9, "Ort", "Installation / Gebäude / Raum"]] as const
    : [[3, "Prog.", "Programming"], [8, "Topology", "Installation / KNX line"], [9, "Location", "Installation / building / room"]] as const) {
    const button = headings[index].querySelector("button")!;
    expect(button.querySelector(".table-column-label")!.textContent).toBe(label);
    expect(button.getAttribute("aria-label")).toBe(full);
    expect(button.title).toBe(full);
  }
});
it("compares only filtered rows after one gateway confirmation and makes relevant edits Outdated", async () => {
  const project = { ...tree, project_incarnation: 4, comparison_revisions: { 2: 1, 10: 0 } };
  apiMock.compareDevice.mockResolvedValue({ serverIncarnation: "table-test", projectIncarnation: 4, deviceId: 2, comparisonRevision: 1,
    address: "1.1.2", deviceName: "Device 2", programId: "A", written: false, partial: false, mask: 0x0701, manufacturer: 0x83,
    same: true, octets: 1, differingOctets: 0, changes: [], loadStates: [{ machine: "Application", state: "Loaded" }], outdated: false,
    decoded: [{ kind: "raw", label: "", address: 0x4000, project: "21", device: "21", reason: null }] });
  await render(true, project); await query("Device 2");
  expect(host.querySelectorAll(".comparison-tag")).toHaveLength(0);
  const all = [...host.querySelectorAll("button")].find((b) => b.textContent === "Compare all shown")!;
  expect(all).toBeTruthy(); await act(async () => all.click());
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  const gateway = document.querySelector<HTMLInputElement>(".comparison-gateway")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(gateway, "192.0.2.1:3671");
    gateway.dispatchEvent(new Event("input", { bubbles: true })); });
  const confirm = [...document.querySelectorAll("button")].find((b) => b.textContent === "Confirm read-only comparison")!;
  await act(async () => confirm.click());
  expect(apiMock.compareDevice).toHaveBeenCalledExactlyOnceWith("1.1.2", "192.0.2.1:3671",
    { serverIncarnation: "table-test", projectIncarnation: 4, deviceId: 2, comparisonRevision: 1 });
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Synced");
  // The equal raw range is hidden by default (WP-18) and the toggle reveals it.
  expect(document.querySelectorAll(".comparison-decoded tbody tr")).toHaveLength(0);
  expect(document.querySelector(".comparison-hidden")?.textContent).toContain("1 equal row is hidden");
  await act(async () => document.querySelector<HTMLInputElement>(".comparison-filter input")!.click());
  expect(document.querySelector(".comparison-decoded")?.textContent).toContain("21");
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Synced");
  await render(true, { ...project, snapshot_revision: 2, comparison_revisions: { 2: 2, 10: 0 } });
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Outdated");
});

it("hands the server's byte-level changes to the result table, so a reported byte is marked", async () => {
  const project = { ...tree, project_incarnation: 4, comparison_revisions: { 2: 1, 10: 0 } };
  apiMock.compareDevice.mockResolvedValue({ serverIncarnation: "table-test", projectIncarnation: 4, deviceId: 2, comparisonRevision: 1,
    address: "1.1.2", deviceName: "Device 2", programId: "A", written: false, partial: false, mask: 0x0701, manufacturer: 0x83,
    same: false, octets: 2, differingOctets: 1, changes: [{ address: 0x4001, segment: null, device: [0x22], project: [0x30] }],
    loadStates: [{ machine: "Application", state: "Loaded" }], outdated: false,
    decoded: [{ kind: "raw", label: "", address: 0x4000, project: "21 22", device: "21 22", reason: null }] });
  await render(true, project); await query("Device 2");
  await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "Compare all shown")!.click());
  const gateway = document.querySelector<HTMLInputElement>(".comparison-gateway")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(gateway, "192.0.2.1:3671");
    gateway.dispatchEvent(new Event("input", { bubbles: true })); });
  await act(async () => [...document.querySelectorAll("button")].find((b) => b.textContent === "Confirm read-only comparison")!.click());
  expect(host.querySelector(".comparison-tag")?.textContent).toBe("Different");
  expect(document.querySelectorAll(".comparison-decoded tbody tr")).toHaveLength(1);
  const device = document.querySelectorAll(".comparison-decoded tbody tr td")[1];
  expect([...device.querySelectorAll(".comparison-byte")].map((b) => b.getAttribute("data-mark"))).toEqual(["equal", "different"]);
});

it("marks untranslated product text when a product language was requested", async () => {
  await act(async () => setSetting("productLanguage", "de"));
  apiMock.deviceCatalog.mockResolvedValue({ ...batch, devices: [{ id: 2, device: tree.installations[0].unassigned[0],
    productRef: "P-1", resolution: "Resolved", manufacturerId: "M-1", manufacturerName: null,
    productText: "Package product text", orderNumber: null, productTextLanguage: null, productSourceLanguage: "en-US",
    system: { state: "family", mask: 0x07b0, family: "systemB" } }] });
  await render();
  expect(host.querySelector(".language-fallback-badge")?.textContent).toContain("en-US");
});
it("opens a device on ordinary row text clicks without double-activating name buttons", async () => {
  await render();
  await act(async () => host.querySelector<HTMLTableCellElement>("tbody td:last-child")!.click());
  expect(click).toHaveBeenCalledTimes(1);
  expect(click.mock.calls[0].slice(1)).toEqual(["device", 2, { kind: "device", id: 2 }, [2, 10], true]);
  click.mockClear();
  await act(async () => host.querySelector<HTMLButtonElement>("tbody td:nth-child(3) button")!.click());
  expect(click).toHaveBeenCalledTimes(1);
});
it("defers chip nodes in a never-opened table, then keeps them across hidden editor visits", async () => {
  const shown: ProjectTree = { ...tree, installations: tree.installations.map((installation) => ({
    ...installation, unassigned: installation.unassigned.map((device) => ({ ...device,
      pending_programming: { address: true, application: true, parameters: true, group_addresses: true } })),
  })) };
  await render(false, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(0);
  await render(true, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(8);
  const first = host.querySelector(".programming-chips");
  await render(false, shown);
  expect(host.querySelectorAll(".programming-chip")).toHaveLength(8);
  expect(host.querySelector(".programming-chips")).toBe(first);
  await render(true, shown);
  expect(host.querySelector(".programming-chips")).toBe(first);
});

it("does not fetch metadata before the list is opened", async () => {
  await render(false); expect(apiMock.deviceCatalog).not.toHaveBeenCalled();
  await render(); expect(apiMock.deviceCatalog).toHaveBeenCalledTimes(1);
  expect(catalogue).toHaveBeenCalledExactlyOnceWith(batch);
});
it("filters and preserves query, sort, scroll and checkboxes across a hidden editor visit", async () => {
  await render(); await query("Device");
  const name = [...host.querySelectorAll<HTMLButtonElement>("th button")].find((button) => button.textContent === "Name")!;
  await act(async () => name.click()); await act(async () => name.click());
  const scroll = host.querySelector<HTMLElement>(".devices-table-scroll")!; scroll.scrollLeft = 200; scroll.scrollTop = 100;
  await render(false); await render();
  expect(host.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("Device");
  expect(host.querySelector('th[aria-sort="descending"]')?.textContent).toContain("Name");
  expect(host.querySelector(".devices-table-scroll")).toBe(scroll);
  expect(scroll.scrollLeft).toBe(200); expect(scroll.scrollTop).toBe(100);
  expect(host.querySelector<HTMLInputElement>('tbody input[type="checkbox"]')!.checked).toBe(false);
  expect(host.querySelectorAll<HTMLInputElement>('tbody input[type="checkbox"]')[1].checked).toBe(true);
  await query("Device 2"); expect(host.querySelectorAll("tbody tr")).toHaveLength(1);
});
it("passes filtered/sorted render order to the shared multi-selection controller", async () => {
  await render(); await query("Device 10");
  await act(async () => host.querySelector<HTMLButtonElement>("tbody td:nth-child(3) button")!
    .dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true })));
  expect(click.mock.calls[0].slice(1)).toEqual(["device", 10, { kind: "device", id: 10 }, [10], true]);
});
it("keeps project devices visible and explicitly reports a refused metadata read", async () => {
  apiMock.deviceCatalog.mockRejectedValueOnce(new Error("database unavailable"));
  await render(); expect(host.querySelector('[role="alert"]')?.textContent).toContain("database unavailable");
  expect(host.querySelectorAll("tbody tr")).toHaveLength(2);
  expect(host.textContent).toContain("Not resolved");
  // Without metadata the System cell says so too, like its sibling product columns (WP20-01).
  expect([...host.querySelectorAll("tbody tr")].map((row) => row.querySelectorAll("td")[7].textContent)).toEqual(["Not resolved", "Not resolved"]);
});
it("discards a delayed catalogue from a previous project snapshot", async () => {
  let resolve!: (value: DeviceCatalog) => void;
  apiMock.deviceCatalog.mockReturnValueOnce(new Promise<DeviceCatalog>((done) => { resolve = done; }));
  await render(); const newer = { ...tree, snapshot_revision: 2 };
  apiMock.deviceCatalog.mockResolvedValueOnce({ ...batch, snapshotRevision: 2 });
  await render(true, newer);
  await act(async () => resolve(batch));
  expect(catalogue).toHaveBeenCalledTimes(1);
  expect(catalogue.mock.calls[0][0].snapshotRevision).toBe(2);
});
it("discards delayed translations after a product-language switch", async () => {
  let resolve!: (value: DeviceCatalog) => void;
  apiMock.deviceCatalog.mockReturnValueOnce(new Promise<DeviceCatalog>((done) => { resolve = done; }));
  await render(); await act(async () => setSetting("productLanguage", "de"));
  await act(async () => resolve(batch));
  expect(catalogue).toHaveBeenCalledTimes(1);
  expect(apiMock.deviceCatalog.mock.calls.at(-1)?.[0]).toBe("de");
});
it("refuses a correctly shaped batch bound to another server without losing the device list", async () => {
  apiMock.deviceCatalog.mockResolvedValueOnce({ ...batch, serverIncarnation: "other" });
  await render(); expect(catalogue).not.toHaveBeenCalled();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain("project changed");
  expect(host.querySelectorAll("tbody tr")).toHaveLength(2);
});
it("shows pending parts as chips and narrows to pending devices on request (ADR-0129)", async () => {
  const pendingTree: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], unassigned: [
    { ...tree.installations[0].unassigned[0],
      pending_programming: { address: true, application: false, parameters: true, group_addresses: false } },
    { ...tree.installations[0].unassigned[1],
      pending_programming: { address: false, application: false, parameters: false, group_addresses: false } },
    { id: 11, name: "Device 11", address: "1.1.11", description: null, com_object_count: 0 },
  ] }] };
  await render(true, pendingTree);
  const headers = [...host.querySelectorAll("th")].map(th => th.querySelector("button")?.getAttribute("aria-label") ?? th.textContent);
  expect(headers.indexOf("Programming")).toBe(headers.indexOf("Name") + 1);
  const cells = [...host.querySelectorAll("tbody tr")].map((row) => row.querySelector(".programming-chips"));
  expect(cells.map((cell) => cell?.getAttribute("data-state"))).toEqual(["pending", "clean", "unknown"]);
  expect([...cells[0]!.querySelectorAll(".programming-chip")].map((chip) => chip.textContent)).toEqual(["Adr", "Par"]);
  expect(cells[0]!.getAttribute("role")).toBe("img");
  expect(cells[0]!.getAttribute("aria-label")).toBe("Not programmed: Address and Parameters");
  expect(cells[1]!.querySelectorAll(".programming-chip")).toHaveLength(0);
  const pendingOnly = [...host.querySelectorAll<HTMLInputElement>(".devices-toolbar input[type=\"checkbox\"]")]
    .find((input) => input.closest("label")?.textContent?.includes("Pending only"))!;
  await act(async () => pendingOnly.click());
  expect([...host.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-rename-id"))).toEqual(["2"]);
  expect(host.querySelector('[role="status"]')?.textContent).toBe("1 of 3 devices");
  await act(async () => pendingOnly.click());
  expect(host.querySelectorAll("tbody tr")).toHaveLength(3);
});

it.each(["en", "de"] as const)("%s System column shows the declared family, explicit states, sorts and filters (WP-20)", async language => {
  await act(async () => setSetting("uiLanguage", language));
  const unassigned = [2, 3, 4, 5, 6, 10].map((id) => ({ id, name: `Device ${id}`, address: `1.1.${id}`, description: null, com_object_count: 0 }));
  const project: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], unassigned }] };
  const resolved = { productRef: "P-1", resolution: "Resolved" as const, manufacturerId: "M-1", manufacturerName: null,
    productText: null, orderNumber: null, productTextLanguage: null, productSourceLanguage: null };
  const systems: Record<number, DeviceCatalog["devices"][number]["system"]> = {
    2: { state: "family", mask: 0x07b0, family: "systemB" }, 3: { state: "family", mask: 0x0012, family: "system1" },
    4: { state: "unknownMask", mask: 0x0afd }, 5: { state: "noApplicationProgram" }, 6: { state: "family", mask: 0x0300, family: "system300" },
  };
  apiMock.deviceCatalog.mockResolvedValue({ ...batch, devices: [
    ...unassigned.filter((device) => device.id !== 10).map((device) => ({ id: device.id, device, ...resolved, system: systems[device.id] })),
    { id: 10, device: unassigned[5], productRef: null, resolution: "NoReference", manufacturerId: null, manufacturerName: null,
      productText: null, orderNumber: null, productTextLanguage: null, productSourceLanguage: null, system: { state: "noProduct" } },
  ] });
  await render(true, project);
  const header = host.querySelectorAll("thead th")[7];
  const button = header.querySelector("button")!;
  expect(button.getAttribute("aria-label")).toBe("System");
  expect(header.getAttribute("aria-sort")).toBe("none");
  const cells = () => Object.fromEntries([...host.querySelectorAll("tbody tr")]
    .map((row) => [row.getAttribute("data-rename-id"), row.querySelectorAll("td")[7].textContent]));
  const de = language === "de";
  expect(cells()).toEqual({ 2: "System B", 3: "System 1", 4: de ? "Unbekannt (Maske 0AFDh)" : "Unknown (mask 0AFDh)",
    5: de ? "Kein Applikationsprogramm" : "No application program", 6: "System 300", 10: de ? "Keine Produktreferenz" : "No product reference" });
  const first = host.querySelector("tbody tr")!.querySelectorAll("td")[7];
  expect(first.getAttribute("title")).toBe(de ? "Deklarierte Maskenversion 07B0h (Produktdatenbank)" : "Declared mask version 07B0h (product database)");
  await act(async () => button.click());
  expect(header.getAttribute("aria-sort")).toBe("ascending");
  const order = () => [...host.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-rename-id"));
  expect(order()).toEqual(["5", "10", "3", "6", "2", "4"]);
  await query("system b"); expect(order()).toEqual(["2"]);
  await query("0afd"); expect(order()).toEqual(["4"]);
});

tableColumnSmoke([["devices", 13]], async () => { await render(); return { host }; });

// Issue #31: a retained System-2 run explains the pending marks without touching them.
const bound: ProjectTree = { ...tree, project_incarnation: 4 };
const failedRun = {
  schemaVersion: 3, failureDiagnostics: [], downloadResult: "transferCompleteVerificationUnfinished", operationId: "a".repeat(32),
  binding: { serverIncarnation: "table-test", projectIncarnation: "4", projectRevision: "1", deviceId: 2 }, address: "1.1.2", deviceName: "Device 2",
  state: "failed", completedSteps: 26, totalSteps: 27, backupAvailable: true, recoveryPhrase: null, downloadPhrase: null, releasePhrase: null,
  message: null, refusal: null, mutationMayHaveStarted: true, prestateDiffers: false, pointerObservations: [], programVersionObservation: null,
  prestateUnreadableAllocations: [],
};
const markers = () => [...host.querySelectorAll("[data-system2-result]")];
it("reads the retained run once for the whole table and marks only the bound device", async () => {
  system2.getSystem2Status.mockResolvedValue(failedRun);
  await render(true, bound); await act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });
  expect(system2.getSystem2Status).toHaveBeenCalledTimes(1);
  expect(markers()).toHaveLength(1);
  expect(markers()[0].closest("tr")!.textContent).toContain("Device 2");
  expect(markers()[0].getAttribute("data-system2-result")).toBe("transferCompleteVerificationUnfinished");
  await render(true, { ...bound, snapshot_revision: 2 });
  expect(system2.getSystem2Status).toHaveBeenCalledTimes(1);
  expect(markers()[0].getAttribute("data-system2-edited")).toBe("true");
});
it("shows nothing for another project, another server or a failed read", async () => {
  system2.getSystem2Status.mockResolvedValue(failedRun);
  await render(true, { ...bound, project_incarnation: 5 }); await act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });
  expect(markers()).toHaveLength(0);
  await render(true, { ...bound, server_incarnation: "elsewhere" }); await act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });
  expect(markers()).toHaveLength(0);
});
it("shows nothing when the first shared read fails", async () => {
  system2.getSystem2Status.mockRejectedValue(new Error("offline"));
  await render(true, bound); await act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });
  expect(markers()).toHaveLength(0);
});
it("drops a read that began before the panel published a newer status", async () => {
  let release!: (value: unknown) => void;
  system2.getSystem2Status.mockReturnValueOnce(new Promise((resolve) => { release = resolve; }));
  await render(true, bound);
  await act(async () => publishSystem2Status(null));
  await act(async () => { release(failedRun); await Promise.resolve(); });
  expect(markers()).toHaveLength(0);
});
const settle = async () => act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });
it("shows a run that ended after the table was first visited while it was still running (shared lifetime)", async () => {
  const running = { ...failedRun, state: "running", downloadResult: "inProgress" };
  system2.getSystem2Status.mockResolvedValue(running);
  await render(true, bound); await settle();
  expect(markers()).toHaveLength(0);
  // The panel's single poll owner publishes the terminal status: no table request, no poll.
  await act(async () => publishSystem2Status(failedRun as never)); await settle();
  expect(markers()).toHaveLength(1);
  expect(system2.getSystem2Status).toHaveBeenCalledTimes(1);
});
it("replaces an older verified chip as soon as the next run starts", async () => {
  system2.getSystem2Status.mockResolvedValue({ ...failedRun, state: "finished", downloadResult: "verified", mutationMayHaveStarted: true });
  await render(true, bound); await settle();
  expect(markers()[0].getAttribute("data-system2-result")).toBe("verified");
  await act(async () => publishSystem2Status(null)); await settle();
  expect(markers()).toHaveLength(0);
  await act(async () => publishSystem2Status({ ...failedRun, operationId: "b".repeat(32), state: "running", downloadResult: "inProgress" } as never)); await settle();
  expect(markers()).toHaveLength(0);
  await act(async () => publishSystem2Status(failedRun as never)); await settle();
  expect(markers()[0].getAttribute("data-system2-result")).toBe("transferCompleteVerificationUnfinished");
});
it("never shows a recovered device as the requested configuration's success", async () => {
  system2.getSystem2Status.mockResolvedValue({ ...failedRun, state: "restored", restoreAttempted: true, downloadResult: "verified", mutationMayHaveStarted: true });
  await render(true, bound); await settle();
  expect(markers()[0].getAttribute("data-system2-result")).toBe("recoveryRestored");
});
it("the explicit catalogue refresh asks once more for the shared status", async () => {
  await render(true, bound); await settle();
  expect(system2.getSystem2Status).toHaveBeenCalledTimes(1);
  system2.getSystem2Status.mockResolvedValue(failedRun);
  const refresh = [...host.querySelectorAll("button")].find((b) => b.textContent === "Refresh product data")!;
  await act(async () => refresh.click()); await settle();
  expect(system2.getSystem2Status).toHaveBeenCalledTimes(2);
  expect(markers()).toHaveLength(1);
});
it("finds the bound row by its visible System-2 text", async () => {
  system2.getSystem2Status.mockResolvedValue(failedRun);
  await render(true, bound); await settle();
  await query("S2 unverified");
  expect(host.querySelectorAll(".devices-table tbody tr")).toHaveLength(1);
});
it("says so when the shared status cannot be read, instead of silently showing nothing", async () => {
  system2.getSystem2Status.mockRejectedValue(new Error("offline"));
  await render(true, bound); await settle();
  expect(host.querySelector("[data-system2-read-failed]")).not.toBeNull();
  system2.getSystem2Status.mockResolvedValue(null);
  const refresh = [...host.querySelectorAll("button")].find((b) => b.textContent === "Refresh product data")!;
  await act(async () => refresh.click()); await settle();
  expect(host.querySelector("[data-system2-read-failed]")).toBeNull();
});
