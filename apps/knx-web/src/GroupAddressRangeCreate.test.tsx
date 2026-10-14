/** Issue #20: a range heading creates a group address in that range, typed or next free. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ createGroupAddress: vi.fn(), unlinkComObject: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)) }));

import GroupAddressTable from "./GroupAddressTable";

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests(); resetUiLanguageForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.clearAllMocks(); });

// Main 20 → middles 21 and 22; 44 is a tiny full range; one address without range.
const installation = (): InstallationNode => ({
  id: 1, name: "Example", topology: [], buildings: [], unassigned: [],
  group_ranges: [
    { id: 20, parent: null, name: "Lighting", start: "1/0/0", end: "1/7/255" },
    { id: 21, parent: 20, name: "Ground floor", start: "1/0/0", end: "1/0/255" },
    { id: 22, parent: 20, name: "First floor", start: "1/1/0", end: "1/1/255" },
    { id: 44, parent: null, name: "Tiny", start: "4/0/1", end: "4/0/2" },
  ],
  group_addresses: [
    { id: 30, name: "Alpha", address: "1/0/1", range: 21, dpts: [], links: [] },
    { id: 31, name: "Beta", address: "1/0/2", range: 21, dpts: [], links: [] },
    { id: 32, name: "Gamma", address: "1/1/1", range: 22, dpts: [], links: [] },
    { id: 45, name: "T1", address: "4/0/1", range: 44, dpts: [], links: [] },
    { id: 46, name: "T2", address: "4/0/2", range: 44, dpts: [], links: [] },
    { id: 60, name: "Loose", address: "3/0/1", range: null, dpts: [], links: [] },
  ],
});
const tree = (input: InstallationNode): ProjectTree => ({ schema_version: 15, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [input],
  view_key: "view-1" });

let updates: ProjectTree[] = [];
async function show() {
  updates = [];
  const input = installation();
  await act(async () => root.render(<GroupAddressTable installation={input} tree={tree(input)}
    selection={null} multiSelection={null} onItemClick={() => {}} onTreeUpdate={(next) => updates.push(next)}
    rangeScope={null} projectContextKey="srv:1" />));
}
const click = async (element: Element | null) => {
  expect(element).not.toBeNull();
  await act(async () => (element as HTMLElement).click());
};
async function type(input: HTMLInputElement | null, value: string) {
  expect(input).not.toBeNull();
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
const dialog = () => host.querySelector<HTMLElement>(".ga-new-in-range");
const addButton = (range: number) => host.querySelector<HTMLButtonElement>(`[data-ga-range-add="${range}"]`);
const nameInput = () => dialog()!.querySelector<HTMLInputElement>(".ga-new-in-range-name input");
const createButton = () => dialog()!.querySelector<HTMLButtonElement>(".ga-new-in-range-create");

it("puts a + on every range heading and none on the addresses without range", async () => {
  await show();
  // The tree opens its main ranges, so every range heading is visible.
  const ranges = [...host.querySelectorAll<HTMLButtonElement>("[data-ga-range-add]")].map(b => Number(b.dataset.gaRangeAdd));
  expect(ranges.sort((a, b) => a - b)).toEqual([20, 21, 22, 44]);
  expect(host.querySelector('[data-ga-expand="unassigned"]')!.closest("tr")!.querySelector("[data-ga-range-add]")).toBeNull();
  expect(addButton(21)!.getAttribute("aria-label")).toBe("New group address in Ground floor");
});

it("creates the next free address in the range and opens the range", async () => {
  await show();
  await click(addButton(20));
  expect(host.querySelectorAll(".ga-new-in-range").length).toBe(1);
  // Main range 1/0/0–1/7/255 holds 1/0/1, 1/0/2 and 1/1/1 through its middles: first gap is 1/0/3.
  expect(dialog()!.querySelector("[data-ga-next-free]")!.textContent).toBe("1/0/3");
  expect(dialog()!.querySelector<HTMLInputElement>('input[value="next"]')!.checked).toBe(true);
  expect(document.activeElement).toBe(nameInput());
  expect(createButton()!.disabled).toBe(true);
  const created = tree(installation());
  apiMock.createGroupAddress.mockResolvedValue(created);
  await type(nameInput(), "Kitchen");
  await click(createButton());
  expect(apiMock.createGroupAddress).toHaveBeenCalledWith("Kitchen", "1/0/3", 20);
  expect(updates).toEqual([created]);
  expect(dialog()).toBeNull();
});

it("opens a closed middle range after creating in it", async () => {
  await show();
  await click(host.querySelector("[data-ga-expand-all]"));
  await click(host.querySelector('[data-ga-expand="range:21:0"]')); // close Ground floor
  expect(host.querySelector('[data-ga-expand="range:21:0"]')!.getAttribute("aria-expanded")).toBe("false");
  await click(addButton(21));
  expect(dialog()!.querySelector("[data-ga-next-free]")!.textContent).toBe("1/0/3");
  apiMock.createGroupAddress.mockResolvedValue(tree(installation()));
  await type(nameInput(), "Hall");
  await click(createButton());
  expect(apiMock.createGroupAddress).toHaveBeenCalledWith("Hall", "1/0/3", 21);
  expect(host.querySelector('[data-ga-expand="range:21:0"]')!.getAttribute("aria-expanded")).toBe("true");
});

it("sends a typed address in the canonical spelling", async () => {
  await show();
  await click(host.querySelector("[data-ga-expand-all]"));
  await click(addButton(22));
  await type(dialog()!.querySelector<HTMLInputElement>(".ga-new-in-range-manual"), " 1/1/9 ");
  expect(dialog()!.querySelector<HTMLInputElement>('input[value="manual"]')!.checked).toBe(true);
  apiMock.createGroupAddress.mockResolvedValue(tree(installation()));
  await type(nameInput(), "Stairs");
  await click(createButton());
  expect(apiMock.createGroupAddress).toHaveBeenCalledWith("Stairs", "1/1/9", 22);
});

it("keeps the dialog and the typed name when the server refuses", async () => {
  await show();
  await click(host.querySelector("[data-ga-expand-all]"));
  await click(addButton(21));
  apiMock.createGroupAddress.mockRejectedValue(new Error("group address 1/0/3 already used"));
  await type(nameInput(), "Kitchen");
  await click(createButton());
  expect(dialog()!.querySelector('[role="alert"]')!.textContent).toBe("group address 1/0/3 already used");
  expect(nameInput()!.value).toBe("Kitchen");
  expect(updates).toEqual([]);
});

it("says a full range is full and leaves manual entry", async () => {
  await show();
  await click(addButton(44));
  expect(dialog()!.textContent).toContain("this range is full");
  expect(dialog()!.querySelector<HTMLInputElement>('input[value="next"]')!.disabled).toBe(true);
  expect(dialog()!.querySelector<HTMLInputElement>('input[value="manual"]')!.checked).toBe(true);
  expect(dialog()!.querySelector("[data-ga-next-free]")).toBeNull();
});

it("cancels without creating anything", async () => {
  await show();
  await click(addButton(20));
  await type(nameInput(), "Never");
  await click([...dialog()!.querySelectorAll("button")].find(b => b.textContent === "Cancel")!);
  expect(dialog()).toBeNull();
  expect(apiMock.createGroupAddress).not.toHaveBeenCalled();
});

it("speaks German", async () => {
  setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
  await show();
  expect(addButton(20)!.getAttribute("aria-label")).toBe("Neue Gruppenadresse in Lighting");
  await click(addButton(20));
  expect(dialog()!.textContent).toContain("Nächste freie Adresse:");
  expect(dialog()!.textContent).toContain("Adresse eingeben:");
  expect(createButton()!.textContent).toBe("Anlegen");
});
