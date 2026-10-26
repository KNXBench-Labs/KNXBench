/** Tests the catalogue results table: headings, sorting, stable selection and keyboard order. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CatalogItem } from "./api";
import { PRODUCT_LANGUAGE_STORAGE_KEY, resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  catalogManufacturers: vi.fn().mockResolvedValue([]),
  catalogItems: vi.fn().mockResolvedValue([]),
  installProductPackage: vi.fn(),
  installLegacyProductDatabase: vi.fn(),
  createDevice: vi.fn(),
  currentProject: vi.fn(),
  catalogBackups: vi.fn().mockResolvedValue({ backups: [] }),
  previewCatalogUpdate: vi.fn(),
  applyCatalogUpdate: vi.fn(),
  restoreCatalogBackup: vi.fn(),
  isCatalogueUpdateStale: () => false,
  previewCatalogCleanup: vi.fn(),
  applyCatalogCleanup: vi.fn(),
  isCatalogueCleanupStale: () => false,
}));
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import CatalogBrowser from "./CatalogBrowser";
import { resetSettingsForTests, setSetting, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  apiMock.createDevice.mockReset();
  apiMock.catalogManufacturers.mockResolvedValue([]);
  apiMock.catalogItems.mockResolvedValue([]);
  resetSettingsForTests();
  resetProductLanguageForTests();
  resetUiLanguageForTests();
});

function make(id: string, over: Partial<CatalogItem> = {}): CatalogItem {
  return { id, manufacturerId: "M-1", name: id, number: null, visibleDescription: null, productRefId: `P-${id}`,
    hardware2programRefId: `HP-${id}`, nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null, ...over };
}
const settle = () => act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
async function render(onCreated = vi.fn(), onWizard?: (item: CatalogItem) => void) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => { root.render(<CatalogBrowser lineId={null} onCreated={onCreated} onClose={vi.fn()} onWizard={onWizard} />); });
  await settle();
  return root;
}
const rows = () => Array.from(host!.querySelectorAll<HTMLElement>("table tbody tr"));
const rowIds = () => rows().map((row) => row.dataset.itemId);
const header = (label: string) => Array.from(host!.querySelectorAll<HTMLElement>("th")).find((th) => th.textContent?.includes(label))!;
const sortButton = (label: string) => header(label).querySelector("button")!;
const search = () => host!.querySelector<HTMLInputElement>('input[type="search"]')!;
const status = () => host!.querySelector<HTMLElement>(".catalog-highlight-status")!;
const press = (key: string) => act(async () => { search().dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })); });
const click = (element: Element) => act(async () => (element as HTMLElement).click());

const catalogue = () => [
  make("c-10", { name: "Dimmer 10", number: "ORD-10", manufacturerId: "M-2" }),
  make("c-2", { name: "dimmer 2", number: "ORD-2", manufacturerId: "M-1" }),
  make("c-a", { name: "Actuator", number: null, manufacturerId: "M-2" }),
];
const manufacturers = [{ id: "M-1", name: "Zeta Elektro" }, { id: "M-2", name: "Alpha Systems" }];

describe("catalogue results table", () => {
  it("is a native table with named column headings and Product ascending as the initial sort", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      const table = host!.querySelector("table")!;
      expect(table, "a native table").not.toBeNull();
      expect(table.hasAttribute("role"), "no role override on the native table").toBe(false);
      expect(Array.from(table.querySelectorAll("thead th")).map((th) => th.getAttribute("scope"))).toEqual(["col", "col", "col", "col"]);
      expect(Array.from(table.querySelectorAll("thead th")).map((th) => th.textContent!.replace(/[↑↓]/g, "").trim()))
        .toEqual(["Product", "Manufacturer", "Order number", "Application"]);
      expect(header("Product").getAttribute("aria-sort")).toBe("ascending");
      for (const other of ["Manufacturer", "Order number", "Application"]) {
        expect(["none", null], other).toContain(header(other).getAttribute("aria-sort"));
      }
      expect(header("Product").textContent).toContain("↑");
      expect(host!.querySelectorAll("th button")).toHaveLength(4);
      expect(rowIds(), "natural order: 2 before 10, case ignored").toEqual(["c-a", "c-2", "c-10"]);
      expect(host!.querySelector('[role="listbox"]'), "no listbox over a native table").toBeNull();
      expect(host!.querySelector('[role="option"]')).toBeNull();
    } finally { await act(async () => root.unmount()); }
  });

  it("shows the resolved manufacturer name, the order number and the application", async () => {
    apiMock.catalogManufacturers.mockResolvedValue(manufacturers);
    apiMock.catalogItems.mockResolvedValue([
      make("c-1", { name: "Switch", number: "ORD-1", manufacturerId: "M-2", details: { applicationProgramId: "APP-1",
        applicationName: "Switch program", applicationVersion: "1.2", catalogSectionId: "SEC-1", sourceNames: ["a.knxprod"] } }),
      make("c-9", { name: "Unknown maker", manufacturerId: "M-404" }),
    ]);
    const root = await render();
    try {
      const cells = (row: HTMLElement) => Array.from(row.querySelectorAll("td")).map((td) => td.textContent ?? "");
      const [first, second] = rows().map(cells);
      expect(first[1]).toBe("Alpha Systems");
      expect(first[2]).toBe("ORD-1");
      expect(first[3]).toContain("Switch program");
      expect(first[3]).toContain("1.2");
      expect(second[1], "an unresolved manufacturer shows its id").toBe("M-404");
    } finally { await act(async () => root.unmount()); }
  });

  it("toggles each column's direction and moves aria-sort, without a new request and without touching the API array", async () => {
    const data = catalogue();
    apiMock.catalogManufacturers.mockResolvedValue(manufacturers);
    apiMock.catalogItems.mockResolvedValue(data);
    const root = await render();
    try {
      const requests = apiMock.catalogItems.mock.calls.length;
      await click(sortButton("Product"));
      expect(header("Product").getAttribute("aria-sort")).toBe("descending");
      expect(header("Product").textContent).toContain("↓");
      expect(rowIds()).toEqual(["c-10", "c-2", "c-a"]);
      await click(sortButton("Order number"));
      expect(header("Order number").getAttribute("aria-sort")).toBe("ascending");
      expect(["none", null]).toContain(header("Product").getAttribute("aria-sort"));
      expect(rowIds(), "empty order numbers stay last").toEqual(["c-2", "c-10", "c-a"]);
      await click(sortButton("Order number"));
      expect(rowIds()).toEqual(["c-10", "c-2", "c-a"]);
      await click(sortButton("Manufacturer"));
      expect(rowIds(), "Alpha Systems (M-2) before Zeta Elektro; ties by id").toEqual(["c-10", "c-a", "c-2"]);
      expect(apiMock.catalogItems.mock.calls.length, "sorting is client-side only").toBe(requests);
      expect(data.map((i) => i.id), "the API array keeps its order").toEqual(["c-10", "c-2", "c-a"]);
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps every row's catalogue id and source inspectable", async () => {
    apiMock.catalogItems.mockResolvedValue([make("M-00FA_CI-7", { name: "Dimmer", details: { applicationProgramId: "APP-7",
      applicationName: "Dim", applicationVersion: "3", catalogSectionId: "SEC-7", sourceNames: ["dimmer-v3.knxprod"] } })]);
    const root = await render();
    try {
      const [row] = rows();
      expect(row.dataset.itemId).toBe("M-00FA_CI-7");
      expect(row.textContent).toContain("M-00FA_CI-7");
      expect(row.textContent).toContain("SEC-7");
      expect(row.textContent).toContain("dimmer-v3.knxprod");
    } finally { await act(async () => root.unmount()); }
  });

  it("selects through an independent native button and keeps the product across sorts", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      const target = rows()[1];
      const button = target.querySelector<HTMLButtonElement>("button")!;
      expect(button.closest("button") === button && button.querySelector("button")).toBeNull();
      expect(button.getAttribute("aria-pressed")).toBe("false");
      await click(button);
      expect(host!.querySelector<HTMLInputElement>('input[aria-label="Device name"]')!.value).toBe("dimmer 2");
      await click(sortButton("Product"));
      expect(rowIds()).toEqual(["c-10", "c-2", "c-a"]);
      const pressed = rows().filter((row) => row.querySelector("button")!.getAttribute("aria-pressed") === "true");
      expect(pressed.map((row) => row.dataset.itemId)).toEqual(["c-2"]);
      expect(pressed[0].classList.contains("selected")).toBe(true);
      expect(host!.querySelector<HTMLInputElement>('input[aria-label="Device name"]')!.value).toBe("dimmer 2");
    } finally { await act(async () => root.unmount()); }
  });

  it("creates the selected product after a re-sort, never a neighbour", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    apiMock.createDevice.mockResolvedValue({ tree: { fixture: true }, diagnostics: [], items: [] });
    const onCreated = vi.fn();
    const root = await render(onCreated);
    try {
      await click(rows()[2].querySelector("button")!); // c-10
      await click(sortButton("Product"));
      await click(sortButton("Order number"));
      const create = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Create")!;
      await click(create);
      expect(apiMock.createDevice).toHaveBeenCalledOnce();
      expect(apiMock.createDevice.mock.calls[0][1]).toBe("c-10");
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the selection through a filter refresh and a catalogue refresh, by id", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await click(rows()[0].querySelector("button")!); // c-a
      apiMock.catalogItems.mockResolvedValue([make("c-new", { name: "Aaa" }), ...catalogue().reverse()]);
      const input = search();
      await act(async () => {
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
        setter.call(input, "a");
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await settle();
      expect(rowIds()).toEqual(["c-new", "c-a", "c-2", "c-10"]);
      const pressed = rows().filter((row) => row.querySelector("button")!.getAttribute("aria-pressed") === "true");
      expect(pressed.map((row) => row.dataset.itemId)).toEqual(["c-a"]);
      expect(apiMock.catalogItems).toHaveBeenLastCalledWith(undefined, "a", null);
    } finally { await act(async () => root.unmount()); }
  });

  it("announces the highlighted visible row in a polite status and uses no invalid active descendant", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const scroll = vi.fn();
    const previous = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollIntoView");
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: scroll });
    const root = await render();
    try {
      await click(sortButton("Product")); // descending: c-10, c-2, c-a
      const input = search();
      expect(input.hasAttribute("aria-activedescendant")).toBe(false);
      expect(input.hasAttribute("role"), "native type=search needs no role").toBe(false);
      expect(input.hasAttribute("aria-controls")).toBe(false);
      expect(host!.querySelector("[aria-activedescendant]")).toBeNull();
      const region = status();
      expect(region.getAttribute("aria-live") ?? "polite").toBe("polite");
      expect(region.textContent, "silent before any keyboard move").toBe("");
      scroll.mockClear();
      await press("ArrowDown");
      expect(status()).toBe(region); // persistent node, so assistive tech sees the change
      expect(region.textContent).toBe("Highlighted: dimmer 2, row 2 of 3");
      const active = host!.querySelector("tr.active") as HTMLElement;
      expect(active.dataset.itemId).toBe("c-2");
      expect(scroll.mock.contexts.at(-1)).toBe(active);
      expect(document.activeElement).toBe(input);
      await press("ArrowDown");
      expect(region.textContent).toBe("Highlighted: Actuator, row 3 of 3");
      await press("ArrowUp");
      expect(region.textContent).toBe("Highlighted: dimmer 2, row 2 of 3");
      await press("Enter");
      expect(host!.querySelector<HTMLInputElement>('input[aria-label="Device name"]')!.value).toBe("dimmer 2");
      expect(apiMock.createDevice, "Enter selects, it does not create").not.toHaveBeenCalled();
      const pressed = rows().filter((row) => row.querySelector("button")!.getAttribute("aria-pressed") === "true");
      expect(pressed.map((row) => row.dataset.itemId)).toEqual(["c-2"]);
    } finally {
      await act(async () => root.unmount());
      if (previous) Object.defineProperty(HTMLElement.prototype, "scrollIntoView", previous);
      else delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
    }
  });

  it("announces positions in the sorted order, not the arrival order, and stays quiet on re-sort or click", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown"); // ascending: c-a, c-2, c-10
      expect(status().textContent).toBe("Highlighted: dimmer 2, row 2 of 3");
      await press("ArrowDown");
      expect(status().textContent).toBe("Highlighted: Dimmer 10, row 3 of 3");
      await click(sortButton("Product")); // descending: c-10 is now row 1
      expect(rowIds()).toEqual(["c-10", "c-2", "c-a"]);
      expect(status().textContent, "a re-sort is not a keyboard move").toBe("Highlighted: Dimmer 10, row 3 of 3");
      await press("ArrowDown");
      expect(status().textContent).toBe("Highlighted: dimmer 2, row 2 of 3");
      await click(rows()[2].querySelector("button")!); // selecting another product
      expect(status().textContent, "selecting another product is not announced").toBe("Highlighted: dimmer 2, row 2 of 3");
    } finally { await act(async () => root.unmount()); }
  });

  it("announces in German under the German UI", async () => {
    saveUiLanguage(settingsStorage, "de");
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown");
      expect(status().textContent).toBe("Hervorgehoben: dimmer 2, Zeile 2 von 3");
    } finally { await act(async () => root.unmount()); }
  });

  it("does not announce while the workspace is hidden", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown");
      expect(status().textContent).not.toBe("");
      await act(async () => root.render(<CatalogBrowser lineId={null} active={false} onCreated={vi.fn()} onClose={vi.fn()} />));
      expect(status().textContent, "an off-screen workspace stays silent").toBe("");
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the highlighted product highlighted when the sort changes", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown"); // c-2 in ascending order
      await click(sortButton("Product"));
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId).toBe("c-2");
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the highlight by id across a same-content refresh with a cloned array", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown"); // c-2, second row
      apiMock.catalogItems.mockResolvedValue(catalogue().map((row) => ({ ...row }))); // fresh array, same rows
      await act(async () => setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de"));
      await settle();
      expect(apiMock.catalogItems).toHaveBeenCalledTimes(2);
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId).toBe("c-2");
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the highlight across a filtered result that retains the item, and across a new sort", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown"); // c-2
      apiMock.catalogItems.mockResolvedValue(catalogue().filter((row) => row.id !== "c-10")); // c-a, c-2
      await act(async () => {
        const input = search();
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "d");
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await settle();
      expect(rowIds()).toEqual(["c-a", "c-2"]);
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId, "c-2 retained at index 1").toBe("c-2");
      await click(sortButton("Order number"));
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId).toBe("c-2");
    } finally { await act(async () => root.unmount()); }
  });

  it("falls back to the first row once the highlighted id is absent, and does not jump back later", async () => {
    apiMock.catalogItems.mockResolvedValue(catalogue());
    const root = await render();
    try {
      await press("ArrowDown"); // c-2
      const change = async (value: string) => act(async () => {
        const input = search();
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      apiMock.catalogItems.mockResolvedValue(catalogue().filter((row) => row.id !== "c-2")); // c-a, c-10
      await change("x");
      await settle();
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId).toBe("c-a");
      apiMock.catalogItems.mockResolvedValue(catalogue()); // c-2 is back
      await change("y");
      await settle();
      expect((host!.querySelector("tr.active") as HTMLElement).dataset.itemId, "the cleared highlight stays cleared").toBe("c-a");
    } finally { await act(async () => root.unmount()); }
  });

  it("marks language fallbacks inside the product cell", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.catalogItems.mockResolvedValue([make("c-1", { name: "Switch", visibleDescription: "Two channels", sourceLanguage: "en-US" })]);
    const root = await render();
    try {
      expect(rows()[0].querySelectorAll("td")[0].querySelectorAll(".language-fallback-badge")).toHaveLength(2);
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the headings and the empty message when nothing matches", async () => {
    const root = await render();
    try {
      expect(host!.querySelectorAll("table tbody tr")).toHaveLength(0);
      expect(host!.querySelector(".search-empty")!.textContent).toBe("No matches.");
      expect(host!.querySelectorAll("th button")).toHaveLength(4);
      expect(search().hasAttribute("aria-activedescendant")).toBe(false);
      expect(status().textContent).toBe("");
    } finally { await act(async () => root.unmount()); }
  });

  it("is German under the German UI, with natural German ordering", async () => {
    saveUiLanguage(settingsStorage, "de");
    apiMock.catalogItems.mockResolvedValue([make("c-1", { name: "Zeit" }), make("c-2", { name: "Äpfel" }), make("c-3", { name: "Apfel" })]);
    const root = await render();
    try {
      expect(Array.from(host!.querySelectorAll("thead th")).map((th) => th.textContent!.replace(/[↑↓]/g, "").trim()))
        .toEqual(["Produkt", "Hersteller", "Bestellnummer", "Anwendung"]);
      expect(rowIds()).toEqual(["c-3", "c-2", "c-1"]);
      expect(header("Produkt").getAttribute("aria-sort")).toBe("ascending");
    } finally { await act(async () => root.unmount()); }
  });
});

describe("catalogue table display text", () => {
  const detail = (applicationName: string | null, applicationVersion: string | null) => ({ applicationProgramId: "A",
    applicationName, applicationVersion, catalogSectionId: null, sourceNames: [] });
  const cells = (id: string) => Array.from(host!.querySelector(`tr[data-item-id="${id}"]`)!.querySelectorAll("td")).map((td) => td.querySelector("button")?.textContent ?? td.textContent ?? "");

  it("shows the id for a null, empty or whitespace product name, never an unnamed button", async () => {
    apiMock.catalogItems.mockResolvedValue([make("n-null", { name: null }), make("n-empty", { name: "" }), make("n-blank", { name: "  \t " })]);
    const root = await render();
    try {
      for (const id of ["n-null", "n-empty", "n-blank"]) expect(cells(id)[0]).toBe(id);
      for (const button of host!.querySelectorAll<HTMLElement>(".catalog-select")) expect(button.textContent!.trim()).not.toBe("");
      await press("ArrowDown");
      expect(status().textContent).toBe("Highlighted: n-empty, row 2 of 3");
    } finally { await act(async () => root.unmount()); }
  });

  it("falls back to the id for an empty manufacturer name and dashes blank order number and application", async () => {
    apiMock.catalogManufacturers.mockResolvedValue([{ id: "M-1", name: "" }, { id: "M-2", name: "   " }]);
    apiMock.catalogItems.mockResolvedValue([
      make("b-1", { manufacturerId: "M-1", number: "   ", details: detail("  ", "2.0") }),
      make("b-2", { manufacturerId: "M-2", number: "", details: detail(null, "3") }),
      make("b-3", { manufacturerId: "M-3", number: "ORD", details: detail(" Prog ", " 1 ") }),
    ]);
    const root = await render();
    try {
      expect(cells("b-1").slice(1)).toEqual(["M-1", "—", "—"]);
      expect(cells("b-2").slice(1)).toEqual(["M-2", "—", "—"]);
      expect(cells("b-3").slice(1)).toEqual(["M-3", "ORD", "Prog 1"]);
    } finally { await act(async () => root.unmount()); }
  });

  it("orders rows by the very text each column shows", async () => {
    apiMock.catalogItems.mockResolvedValue([
      make("z-id", { name: "  " }), make("a-1", { name: "Beta", number: "  " }), make("a-2", { name: "Alpha", number: "7" }),
    ]);
    const root = await render();
    try {
      expect(rows().map((row) => row.querySelector("button")!.textContent)).toEqual(["Alpha", "Beta", "z-id"]);
      await click(sortButton("Order number"));
      expect(rowIds(), "the dash row sorts last").toEqual(["a-2", "a-1", "z-id"]);
    } finally { await act(async () => root.unmount()); }
  });

  it("keeps the raw item for creation: a blank name still pre-fills nothing invented", async () => {
    apiMock.catalogItems.mockResolvedValue([make("n-blank", { name: "  " })]);
    const root = await render();
    try {
      await click(rows()[0]);
      expect(host!.querySelector<HTMLInputElement>('input[aria-label="Device name"]')!.value).toBe("  ");
    } finally { await act(async () => root.unmount()); }
  });
});
