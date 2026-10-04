/** Tests for CatalogBrowser's install/refresh, diagnostic translation, and keyboard navigation. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  CatalogInstallCount,
  CatalogInstallDiagnostic,
  CatalogInstallReport,
  CatalogUnknownConstruct,
} from "./api";
import { messages as deMessages } from "./messages/de";
import {
  PRODUCT_LANGUAGE_STORAGE_KEY,
  resetProductLanguageForTests,
  useProductLanguage,
} from "./productLanguage";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  catalogManufacturers: vi.fn().mockResolvedValue([]),
  catalogItems: vi.fn().mockResolvedValue([]),
  installProductPackage: vi.fn<(file: File) => Promise<CatalogInstallReport>>(),
  createDevice: vi.fn(),
  currentProject: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import CatalogBrowser from "./CatalogBrowser";
import { resetSettingsForTests, setSetting, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

function installReport(overrides: Partial<CatalogInstallReport> = {}): CatalogInstallReport {
  return {
    sha256: "abc",
    scheme: 11,
    skipped: false,
    members: [],
    unknown: 0,
    conflicts: 0,
    translations: { program: 0, catalog: 0, hardware: 0, master: 0 },
    droppedDatapointTypes: 0,
    facts: null,
    ...overrides,
  };
}

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  // clearAllMocks keeps queued `…Once` values; a test that stops early must
  // not hand them to the next one.
  apiMock.createDevice.mockReset();
  apiMock.currentProject.mockReset();
  apiMock.catalogManufacturers.mockResolvedValue([]);
  apiMock.catalogItems.mockResolvedValue([]);
  resetSettingsForTests();
  resetProductLanguageForTests();
  resetUiLanguageForTests();
});

// ADR-0069: 1–128 ASCII letters, digits, `-`, `_`.
const REQUEST_ID = /^[A-Za-z0-9_-]{1,128}$/;

const item = {
  id: "cat-1",
  manufacturerId: "M-1",
  name: "Actuator",
  number: null,
  visibleDescription: null,
  productRefId: "P-1",
  hardware2programRefId: "HP-1",
};

const item2 = {
  id: "cat-2",
  manufacturerId: "M-1",
  name: "Dimmer",
  number: null,
  visibleDescription: null,
  productRefId: "P-2",
  hardware2programRefId: "HP-2",
};

async function renderBrowser(onCreated = vi.fn(), onClose = vi.fn(), serverIncarnation?: string, lineId: number | null = null) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<CatalogBrowser lineId={lineId} onCreated={onCreated} onClose={onClose}
      serverIncarnation={serverIncarnation} />);
  });
  return { root, onCreated, onClose };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

describe("CatalogBrowser", () => {
  it("keeps keyboard highlight visible without picking or creating the catalog item", async () => {
    apiMock.catalogItems.mockResolvedValue([item, item2]);
    const previous = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollIntoView");
    const scroll = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: scroll });
    const { root } = await renderBrowser();
    try {
      await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
      const input = host!.querySelector<HTMLInputElement>('[role="combobox"]')!;
      scroll.mockClear();
      await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
      const row = host!.querySelector("#catalog-option-1")!;
      expect(scroll).toHaveBeenCalledOnce();
      expect(scroll.mock.contexts[0]).toBe(row);
      expect(row.classList.contains("active")).toBe(true);
      expect(row.getAttribute("aria-selected")).toBe("false");
      expect(document.activeElement).toBe(input);
      expect(host!.querySelector(".catalog-create-row")).toBeNull();
      expect(apiMock.createDevice).not.toHaveBeenCalled();
    } finally {
      await act(async () => root.unmount());
      if (previous) Object.defineProperty(HTMLElement.prototype, "scrollIntoView", previous);
      else delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
    }
  });

  it("offers only supported product packages in the file picker", async () => {
    const { root } = await renderBrowser();
    expect(host!.querySelector<HTMLInputElement>('input[type="file"]')!.accept)
      .toBe(".knxprod,application/zip");
    root.unmount();
  });

  it("keeps a product-package installation failure visible", async () => {
    apiMock.installProductPackage.mockRejectedValueOnce(new Error("encrypted legacy database"));
    const { root } = await renderBrowser();

    const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
    const file = new File(["legacy"], "legacy.vd2");
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file] });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.installProductPackage).toHaveBeenCalledWith(file);
    expect(host!.textContent).toContain("encrypted legacy database");
    root.unmount();
  });

  it("refreshes an installed catalog using the latest selected manufacturer", async () => {
    apiMock.installProductPackage.mockResolvedValueOnce(installReport());
    apiMock.catalogManufacturers.mockResolvedValue([{ id: "M-2", name: "Vendor" }]);
    apiMock.catalogItems.mockResolvedValue([]);
    const { root } = await renderBrowser();

    const select = host!.querySelector<HTMLSelectElement>("select")!;
    await act(async () => {
      select.value = "M-2";
      select.dispatchEvent(new Event("change", { bubbles: true }));
      const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
      Object.defineProperty(input, "files", { value: [new File(["package"], "vendor.knxprod")] });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.catalogItems).toHaveBeenLastCalledWith("M-2", undefined, null);
    expect(host!.textContent).toContain("Installed: scheme 11");
    expect(host!.textContent).not.toContain("stored, not verified");
    root.unmount();
  });

  it("uses a neutral report name and explicit unavailable text for historical facts", async () => {
    apiMock.installProductPackage.mockResolvedValueOnce(installReport({ skipped: true }));
    const { root } = await renderBrowser();
    const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
    await act(async () => {
      Object.defineProperty(input, "files", { value: [new File(["package"], "old.knxprod")] });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });
    const report = host!.querySelector<HTMLElement>(".catalog-report")!;
    expect(report.getAttribute("aria-label")).toBe("Install report");
    expect(report.textContent).toContain("Install facts unavailable for this historical install");
    expect(report.textContent).not.toContain("Measured install facts");
    root.unmount();
  });

  it("localizes every closed install-fact vocabulary without server detail prose", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const categories: Array<[CatalogInstallCount["category"], keyof typeof deMessages]> = [
      ["archive_member", "catalog.installReport.category.archiveMember"],
      ["product", "catalog.installReport.category.product"],
      ["application_program", "catalog.installReport.category.applicationProgram"],
      ["parameter", "catalog.installReport.category.parameter"],
      ["communication_object", "catalog.installReport.category.communicationObject"],
      ["dynamic_node", "catalog.installReport.category.dynamicNode"],
      ["module", "catalog.installReport.category.module"],
      ["baggage_index", "catalog.installReport.category.baggageIndex"],
      ["baggage", "catalog.installReport.category.baggage"],
      ["unknown_construct", "catalog.installReport.category.unknownConstruct"],
      ["master_section", "catalog.installReport.category.masterSection"],
      ["master_subtree", "catalog.installReport.category.masterSubtree"],
      ["datapoint_type", "catalog.installReport.category.datapointType"],
    ];
    const dispositions: Array<[CatalogInstallCount["disposition"], keyof typeof deMessages]> = [
      ["read", "catalog.installReport.disposition.read"],
      ["stored", "catalog.installReport.disposition.stored"],
      ["deduplicated", "catalog.installReport.disposition.deduplicated"],
      ["retained-but-uninterpreted", "catalog.installReport.disposition.retainedButUninterpreted"],
      ["unsupported", "catalog.installReport.disposition.unsupported"],
      ["dropped", "catalog.installReport.disposition.dropped"],
    ];
    const unknowns: Array<[CatalogUnknownConstruct["kind"], keyof typeof deMessages]> = [
      ["Element", "catalog.installReport.unknownKind.element"],
      ["Attribute", "catalog.installReport.unknownKind.attribute"],
    ];
    const diagnostics: Array<[CatalogInstallDiagnostic["kind"], keyof typeof deMessages]> = [
      ["unsupported-master-section", "catalog.installReport.diagnosticKind.unsupportedMasterSection"],
      ["unsupported-master-subtree", "catalog.installReport.diagnosticKind.unsupportedMasterSubtree"],
      ["unresolved-baggage-declaration", "catalog.installReport.diagnosticKind.unresolvedBaggageDeclaration"],
      ["undeclared-baggage-payload", "catalog.installReport.diagnosticKind.undeclaredBaggagePayload"],
    ];
    apiMock.installProductPackage.mockResolvedValueOnce(installReport({
      facts: {
        counts: categories.map(([category], index) => ({
          category,
          disposition: dispositions[index % dispositions.length][0],
          count: index + 1,
        })),
        unknownConstructs: unknowns.map(([kind], index) => ({
          xpath: `/KNX/X${index}`,
          kind,
          name: `future-${index}`,
          occurrences: index + 1,
          sample: null,
        })),
        unknownOccurrences: 3,
        diagnostics: diagnostics.map(([kind], index) => ({
          kind,
          archivePath: kind.includes("baggage") ? "M-0001/Baggages.xml" : "knx_master.xml",
          xmlPath: kind.includes("baggage") ? "/KNX/ManufacturerData/Manufacturer/Baggages/Baggage" : "/KNX/MasterData/Future",
          detail: `server English detail ${index}`,
          occurrences: index + 1,
        })),
      },
    }));
    const { root } = await renderBrowser();
    const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
    await act(async () => {
      Object.defineProperty(input, "files", { value: [new File(["package"], "measured.knxprod")] });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });
    categories.forEach(([, categoryKey], index) => {
      const dispositionKey = dispositions[index % dispositions.length][1];
      expect(host!.textContent).toContain(
        `${deMessages[categoryKey]} / ${deMessages[dispositionKey]}: ${index + 1}`,
      );
    });
    unknowns.forEach(([, key], index) => {
      expect(host!.textContent).toContain(`${deMessages[key]} future-${index}`);
    });
    diagnostics.forEach(([, key]) => {
      expect(host!.textContent).toContain(deMessages[key]);
    });
    expect(host!.textContent).not.toContain("server English detail");
    root.unmount();
  });

  // Signature bytes are stored but deliberately not cryptographically checked.
  // The server already qualifies its role text; this test pins
  // that the report a person actually reads says so too, in English.
  it("flags an install report containing a signature member as unverified", async () => {
    apiMock.installProductPackage.mockResolvedValueOnce(installReport({
      members: [
        { path: "M-0001.signature", role: "Signature (stored, not verified)", sha256: "def", size: 175 },
        { path: "M-0001/Catalog.xml", role: "Catalog", sha256: "ghi", size: 42 },
      ],
    }));
    const { root } = await renderBrowser();

    const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
    await act(async () => {
      Object.defineProperty(input, "files", { value: [new File(["package"], "signed.knxprod")] });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(host!.textContent).toContain("1 signature member stored, not verified");
    expect(host!.textContent).toContain("this application cannot check it");
    root.unmount();
  });

  it("previews bounded quantities and reports diagnostics for each created device", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockResolvedValue({
      tree: { installations: [] },
      diagnostics: [{ kind: "programlessProduct", catalogItemId: "cat-1" }],
      items: [1, 2, 3].map((index) => ({
        index, deviceId: index, name: `Actuator ${index}`,
        diagnostics: [{ kind: "programlessProduct", catalogItemId: "cat-1" }],
      })),
    });
    const { root, onCreated, onClose } = await renderBrowser();
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
    await act(async () => host!.querySelector<HTMLElement>(".search-result")!.click());
    const quantity = host!.querySelector<HTMLInputElement>('input[aria-label="Quantity"]')!;
    expect(quantity).toBeTruthy();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => { setter.call(quantity, "0"); quantity.dispatchEvent(new Event("input", { bubbles: true })); });
    expect(host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!.disabled).toBe(true);
    await act(async () => { setter.call(quantity, "3"); quantity.dispatchEvent(new Event("input", { bubbles: true })); });
    expect(host!.textContent).toContain("Actuator 1");
    expect(host!.textContent).toContain("Actuator 3");
    expect(host!.textContent).toContain("Physical addresses remain unassigned");
    await act(async () => host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!.click());
    expect(apiMock.createDevice).toHaveBeenCalledWith(null, "cat-1", "Actuator", 3, expect.stringMatching(REQUEST_ID),
      { allocateAddresses: false, uniqueNames: false });
    expect(onCreated).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("Actuator 3");
    expect(host!.querySelectorAll(".catalog-created-item")).toHaveLength(3);
    root.unmount();
  });

  it("does not claim batch success or retry when an older server ignores the quantity", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockResolvedValue({ tree: { installations: [] }, diagnostics: [] });
    const { root, onCreated, onClose } = await renderBrowser();
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
    await act(async () => host!.querySelector<HTMLElement>(".search-result")!.click());
    const quantity = host!.querySelector<HTMLInputElement>('input[aria-label="Quantity"]')!;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => { setter.call(quantity, "3"); quantity.dispatchEvent(new Event("input", { bubbles: true })); });
    await act(async () => host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!.click());
    expect(onCreated).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("The server did not confirm every requested device");
    expect(host!.textContent).toContain("No automatic retry was made");
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  async function pickAndSetQuantity(value: string) {
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
    await act(async () => host!.querySelector<HTMLElement>(".search-result")!.click());
    const quantity = host!.querySelector<HTMLInputElement>('input[aria-label="Quantity"]')!;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => { setter.call(quantity, value); quantity.dispatchEvent(new Event("input", { bubbles: true })); });
  }

  const createButton = () => host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!;
  const retryButton = () => host!.querySelector<HTMLButtonElement>(".catalog-retry");

  it("never sends a second batch with a new requestId after a network error with unknown commit status", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(new TypeError("connection lost"));
    const { root, onCreated, onClose } = await renderBrowser();
    await pickAndSetQuantity("3");
    await act(async () => createButton().click());
    expect(onCreated).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("Could not confirm whether the server added the devices");
    expect(host!.querySelector(".catalog-create-row")?.textContent).toContain("Done");
    await act(async () => host!.querySelector<HTMLElement>(".search-result")!.click());
    expect(host!.textContent).toContain("Could not confirm whether the server added the devices");
    expect(host!.querySelector(".catalog-create-row")?.textContent).toContain("Done");
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  // DATA-03 / ADR-0069: one requestId per submit makes a resend after a lost
  // response safe, as long as the same server process still holds the record.
  it("sends a fresh requestId with each catalog submit", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockResolvedValue({ tree: { installations: [] }, diagnostics: [], replayed: false });
    const first = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("1");
    await act(async () => createButton().click());
    first.root.unmount();
    host?.remove();
    const second = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("1");
    await act(async () => createButton().click());
    const ids = apiMock.createDevice.mock.calls.map((call) => call[4]);
    expect(ids).toHaveLength(2);
    for (const id of ids) expect(id).toMatch(REQUEST_ID);
    expect(ids[0]).not.toBe(ids[1]);
    second.root.unmount();
  });

  it("retries a lost batch with the same requestId and accepts a replayed outcome once", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    const created = [1, 2, 3].map((index) => ({ index, deviceId: index, name: `Actuator ${index}`, diagnostics: [] }));
    apiMock.createDevice
      .mockRejectedValueOnce(new TypeError("connection lost"))
      .mockResolvedValueOnce({ tree: { installations: [] }, diagnostics: [], items: created, replayed: true });
    apiMock.currentProject.mockResolvedValue({ installations: [], server_incarnation: "inc-1" });
    const { root, onCreated } = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("3");
    await act(async () => createButton().click());
    expect(retryButton()).toBeTruthy();
    expect(host!.textContent).toContain("cannot add the devices twice");
    await act(async () => retryButton()!.click());
    expect(apiMock.createDevice).toHaveBeenCalledTimes(2);
    const [firstCall, retryCall] = apiMock.createDevice.mock.calls;
    expect(retryCall).toEqual(firstCall);
    expect(firstCall[4]).toMatch(REQUEST_ID);
    expect(onCreated).toHaveBeenCalledTimes(1);
    expect(host!.querySelectorAll(".catalog-created-item")).toHaveLength(3);
    expect(host!.textContent).not.toContain("Could not confirm whether the server added the devices");
    expect(retryButton()).toBeNull();
    root.unmount();
  });

  it("treats a lost single-device response as unconfirmed and offers the same safe retry", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(Object.assign(new Error("Internal Server Error"), { status: 500 }));
    const { root } = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("1");
    await act(async () => createButton().click());
    expect(host!.textContent).toContain("Could not confirm whether the server added the devices");
    expect(retryButton()).toBeTruthy();
    expect(host!.querySelector(".catalog-create-row")?.textContent).toContain("Done");
    root.unmount();
  });

  it("refuses to retry once the server has restarted and forgotten the request", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(new TypeError("connection lost"));
    apiMock.currentProject.mockResolvedValue({ installations: [], server_incarnation: "inc-2" });
    const { root, onCreated } = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("3");
    await act(async () => createButton().click());
    await act(async () => retryButton()!.click());
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    expect(onCreated).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("The server has restarted");
    expect(retryButton()).toBeNull();
    root.unmount();
  });

  it("offers no retry when the server identity is unknown", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(new TypeError("connection lost"));
    const { root } = await renderBrowser();
    await pickAndSetQuantity("3");
    await act(async () => createButton().click());
    expect(host!.textContent).toContain("Could not confirm whether the server added the devices");
    expect(retryButton()).toBeNull();
    root.unmount();
  });

  it("keeps the retry available when the server cannot be asked yet", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(new TypeError("connection lost"));
    apiMock.currentProject.mockRejectedValue(new TypeError("still offline"));
    const { root } = await renderBrowser(vi.fn(), vi.fn(), "inc-1");
    await pickAndSetQuantity("3");
    await act(async () => createButton().click());
    await act(async () => retryButton()!.click());
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    expect(host!.textContent).toContain("still offline");
    expect(retryButton()).toBeTruthy();
    root.unmount();
  });

  const allocateBox = () => host!.querySelector<HTMLInputElement>('input[type="checkbox"][name="allocateAddresses"]')!;
  const uniqueBox = () => host!.querySelector<HTMLInputElement>('input[type="checkbox"][name="uniqueNames"]')!;

  // MODEL-04: both options are opt-in; addresses can only be taken from a line.
  it("offers address allocation only when a target line is selected", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    const unassigned = await renderBrowser();
    await pickAndSetQuantity("2");
    expect(allocateBox().disabled).toBe(true);
    expect(allocateBox().checked).toBe(false);
    expect(uniqueBox().disabled).toBe(false);
    expect(host!.textContent).toContain("Addresses can only be assigned on a target line");
    unassigned.root.unmount();
    host?.remove();
    const onLine = await renderBrowser(vi.fn(), vi.fn(), "inc-1", 7);
    await pickAndSetQuantity("2");
    expect(allocateBox().disabled).toBe(false);
    expect(allocateBox().checked).toBe(false);
    expect(uniqueBox().checked).toBe(false);
    onLine.root.unmount();
  });

  it("sends the chosen options and shows each allocated address", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockResolvedValue({
      tree: { installations: [] }, diagnostics: [], replayed: false,
      items: [["1.1.2", "Actuator 3"], ["1.1.4", "Actuator 4"], ["1.1.5", "Actuator 5"]].map(([address, deviceName], i) => ({
        index: i + 1, deviceId: i + 1, name: deviceName, address, diagnostics: [],
      })),
    });
    const { root } = await renderBrowser(vi.fn(), vi.fn(), "inc-1", 7);
    await pickAndSetQuantity("3");
    await act(async () => { allocateBox().click(); uniqueBox().click(); });
    expect(host!.textContent).toContain("Free addresses on the line are assigned in order");
    expect(host!.textContent).toContain("Names already in the project are skipped");
    await act(async () => createButton().click());
    expect(apiMock.createDevice).toHaveBeenCalledWith(7, "cat-1", "Actuator", 3, expect.stringMatching(REQUEST_ID),
      { allocateAddresses: true, uniqueNames: true });
    const created = Array.from(host!.querySelectorAll(".catalog-created-item")).map((li) => li.textContent);
    expect(created).toHaveLength(3);
    expect(created[0]).toContain("Actuator 3");
    expect(created[0]).toContain("1.1.2");
    expect(created[2]).toContain("1.1.5");
    root.unmount();
  });

  it("shows a refused allocation as an ordinary error without a retry offer", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice.mockRejectedValue(Object.assign(new Error("line 1.1 has 2 free device addresses, 3 requested"), { status: 400 }));
    const { root, onCreated } = await renderBrowser(vi.fn(), vi.fn(), "inc-1", 7);
    await pickAndSetQuantity("3");
    await act(async () => allocateBox().click());
    await act(async () => createButton().click());
    expect(host!.querySelector(".field-error")?.textContent).toContain("line 1.1 has 2 free device addresses, 3 requested");
    expect(retryButton()).toBeNull();
    expect(onCreated).not.toHaveBeenCalled();
    expect(createButton().disabled).toBe(false);
    root.unmount();
  });

  it("resends the chosen options unchanged on a safe retry", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    apiMock.createDevice
      .mockRejectedValueOnce(new TypeError("connection lost"))
      .mockResolvedValueOnce({ tree: { installations: [] }, diagnostics: [], items: [], replayed: true });
    apiMock.currentProject.mockResolvedValue({ installations: [], server_incarnation: "inc-1" });
    const { root } = await renderBrowser(vi.fn(), vi.fn(), "inc-1", 7);
    await pickAndSetQuantity("2");
    await act(async () => allocateBox().click());
    await act(async () => createButton().click());
    await act(async () => retryButton()!.click());
    const [first, retried] = apiMock.createDevice.mock.calls;
    expect(first[5]).toEqual({ allocateAddresses: true, uniqueNames: false });
    expect(retried).toEqual(first);
    root.unmount();
  });

  it("locks a successful diagnostic create and offers Done instead of another Create", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    const create = deferred<{ tree: { installations: never[] }; diagnostics: [{ kind: "programlessProduct"; catalogItemId: string }] }>();
    apiMock.createDevice.mockReturnValue(create.promise);
    const { root, onCreated, onClose } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const result = host!.querySelector<HTMLElement>(".search-result")!;
    await act(async () => result.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const button = host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    expect(button.disabled).toBe(true);

    await act(async () => {
      create.resolve({
        tree: { installations: [] },
        diagnostics: [{ kind: "programlessProduct", catalogItemId: "cat-1" }],
      });
      await create.promise;
    });
    expect(onCreated).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("Device created with diagnostics.");
    expect(host!.querySelector(".catalog-create-row")!.textContent).toContain("Done");
    expect(host!.textContent).not.toContain("Create");

    await act(async () => {
      host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      );
    });
    expect(onClose).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  // Task 5, D4 exception: a known `CreationDiagnostic.kind` is composed
  // client-side from its structured fields, in the active UI language —
  // never the server's English `detail` prose.
  it("composes a translated sentence for a known CreationDiagnostic kind, in German too", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    const create = deferred<{
      tree: { installations: never[] };
      diagnostics: [{ kind: "ambiguousDpt"; refId: string; alternatives: string[]; detail: string }];
    }>();
    apiMock.createDevice.mockReturnValue(create.promise);
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const result = host!.querySelector<HTMLElement>(".search-result")!;
    await act(async () => result.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const button = host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      create.resolve({
        tree: { installations: [] },
        diagnostics: [
          {
            kind: "ambiguousDpt",
            refId: "CO-7",
            alternatives: ["9.001", "9.002"],
            detail: "no DPT could be inferred for CO-7; alternatives: 9.001, 9.002",
          },
        ],
      });
      await create.promise;
    });

    const diagnosticText = host!.querySelector(".catalog-diagnostics li")!.textContent!;
    expect(diagnosticText).toContain("CO-7");
    expect(diagnosticText).toContain("9.001, 9.002");
    expect(diagnosticText).not.toBe("no DPT could be inferred for CO-7; alternatives: 9.001, 9.002");
    // German wording, not a copy of the server's English `detail`.
    expect(diagnosticText).toContain("konnte kein DPT ermittelt werden");
    root.unmount();
  });

  // Task 5, D4 exception: a `kind` this build has never heard of (a future
  // server variant) must still show *something* — the server's own
  // `detail` sentence, verbatim, rather than a blank diagnostic line.
  it("falls back to the server's detail verbatim for an unknown CreationDiagnostic kind", async () => {
    apiMock.catalogItems.mockResolvedValue([item]);
    const create = deferred<{
      tree: { installations: never[] };
      diagnostics: [{ kind: string; detail: string }];
    }>();
    apiMock.createDevice.mockReturnValue(create.promise);
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const result = host!.querySelector<HTMLElement>(".search-result")!;
    await act(async () => result.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const button = host!.querySelector<HTMLButtonElement>(".catalog-create-row button")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      create.resolve({
        tree: { installations: [] },
        diagnostics: [
          { kind: "somethingFutureAndUnknown", detail: "a brand new diagnostic kind this build cannot name" },
        ],
      });
      await create.promise;
    });

    expect(host!.querySelector(".catalog-diagnostics li")!.textContent).toBe(
      "a brand new diagnostic kind this build cannot name",
    );
    root.unmount();
  });

  // Regression test for KNOWN_LIMITATIONS.md §20: the catalog result list
  // had no keyboard path at all, so a keyboard-only user could not reach
  // it. No mouse event appears anywhere in this test.
  it("ArrowDown then Enter on the search input selects the second catalog item", async () => {
    apiMock.catalogItems.mockResolvedValue([item, item2]);
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const input = host!.querySelector<HTMLInputElement>('input[placeholder="Search catalog items…"]')!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    });
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    });

    const nameInput = host!.querySelector<HTMLInputElement>('input[placeholder="Device name"]')!;
    expect(nameInput.value).toBe("Dimmer");
    root.unmount();
  });

  it("ArrowUp at the top of the list stays at the top", async () => {
    apiMock.catalogItems.mockResolvedValue([item, item2]);
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const input = host!.querySelector<HTMLInputElement>('input[placeholder="Search catalog items…"]')!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });

    expect(input.getAttribute("aria-activedescendant")).toBe("catalog-option-0");
    root.unmount();
  });

  // T32 Task 4: the catalog browser must forward the active product
  // language to `api.catalogItems`, exactly the way `ParameterPanel.test.
  // tsx`'s "sends the active product language" tests already prove for the
  // parameter panel's own fetch.
  it("sends the active product language when fetching catalog items", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de-DE");
    apiMock.catalogItems.mockResolvedValue([]);
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });

    expect(apiMock.catalogItems).toHaveBeenLastCalledWith(undefined, undefined, "de-DE");
    root.unmount();
  });

  // The effect's dependency array must include `language`: changing the
  // setting while the browser is already open must refetch, not leave the
  // list showing the previously selected language's names. Mounts a
  // "Writer" alongside `CatalogBrowser`, the same `useProductLanguage()`
  // reader/writer shape `productLanguage.test.tsx`'s own "a writer's
  // change reaches an already-mounted reader" test uses — `CatalogBrowser`
  // itself has no UI to change the setting, so this is the only way to
  // flip it while it's mounted.
  it("refetches catalog items when the product language changes while open", async () => {
    function Writer() {
      const [, setLanguage] = useProductLanguage();
      return (
        <button type="button" onClick={() => setLanguage("fr-FR")}>
          set fr-FR
        </button>
      );
    }

    apiMock.catalogItems.mockResolvedValue([]);
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(
        <>
          <CatalogBrowser lineId={null} onCreated={vi.fn()} onClose={vi.fn()} />
          <Writer />
        </>,
      );
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    expect(apiMock.catalogItems).toHaveBeenLastCalledWith(undefined, undefined, null);

    const button = Array.from(host.querySelectorAll("button")).find(
      (candidate) => candidate.textContent === "set fr-FR",
    )!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 250));
    });

    expect(apiMock.catalogItems).toHaveBeenLastCalledWith(undefined, undefined, "fr-FR");
    root.unmount();
  });

  it("names the highlighted row via the input's aria-activedescendant", async () => {
    apiMock.catalogItems.mockResolvedValue([item, item2]);
    const { root } = await renderBrowser();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    const input = host!.querySelector<HTMLInputElement>('input[placeholder="Search catalog items…"]')!;
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-haspopup")).toBe("listbox");
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    });

    expect(input.getAttribute("aria-activedescendant")).toBe("catalog-option-1");
    const highlighted = host!.querySelector("#catalog-option-1")!;
    expect(highlighted.textContent).toContain("Dimmer");
    root.unmount();
  });
});
