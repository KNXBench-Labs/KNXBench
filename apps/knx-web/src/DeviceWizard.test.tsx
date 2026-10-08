/** Tests for the add-device wizard: preview, confirmed create, stale refresh and discard. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CatalogItem, CatalogPreview } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  previewDevices: vi.fn(),
  createDevice: vi.fn(),
  currentProject: vi.fn(),
  catalogManufacturers: vi.fn(),
  catalogItems: vi.fn(),
  installProductPackage: vi.fn(),
}));

// `isPreviewStale` mirrors the real one (status 409 + kind), so the stale
// branch is exercised by the same shape the server sends.
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  isPreviewStale: (e: unknown) => {
    const error = e as { status?: unknown; body?: { kind?: unknown } } | null;
    return error?.status === 409 && error.body?.kind === "catalogPreviewStale";
  },
}));

import DeviceWizard from "./DeviceWizard";

const PRODUCT: CatalogItem = {
  id: "M-0001_H-1_P-1_CI-1", manufacturerId: "M-0001", name: "Switch actuator", number: "SA 4",
  visibleDescription: null, productRefId: null, hardware2programRefId: null,
  nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null,
};

function tree(): ProjectTree {
  return {
    schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "boot-1",
    installations: [{
      id: 0, name: "Installation",
      topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 7, name: "Line", address: 1, devices: [] }] }],
      buildings: [{ id: 30, name: "Kitchen", kind: "Room", children: [], devices: [] }],
      unassigned: [], group_addresses: [], group_ranges: [],
    }],
  };
}

const PREVIEW: CatalogPreview = {
  items: [
    { index: 1, name: "Switch actuator 1", address: "1.1.1" },
    { index: 2, name: "Switch actuator 2", address: "1.1.2" },
  ],
  diagnostics: [],
  installationId: 0,
};

let host: HTMLDivElement | undefined;
let root: ReturnType<typeof createRoot> | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  window.localStorage.clear();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

async function render(props: { product?: CatalogItem; target?: Parameters<typeof DeviceWizard>[0]["target"] } = {}) {
  const onCreated = vi.fn();
  const onClose = vi.fn();
  const onOpenDevice = vi.fn();
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(
      <DeviceWizard tree={tree()} target={props.target ?? { lineId: 7 }} product={props.product}
        onCreated={onCreated} onClose={onClose} onOpenDevice={onOpenDevice} />,
    );
  });
  return { onCreated, onClose, onOpenDevice };
}

function button(name: string): HTMLButtonElement {
  const found = Array.from(document.querySelectorAll<HTMLButtonElement>("button"))
    .find((b) => b.textContent?.trim() === name);
  if (!found) throw new Error(`no button ${name}`);
  return found;
}

async function click(element: HTMLElement) {
  await act(async () => {
    element.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

async function change(select: HTMLSelectElement | HTMLInputElement, value: string) {
  await act(async () => {
    const proto = select instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(select, value);
    select.dispatchEvent(new Event(select instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
  });
}

function labelled(text: string): HTMLSelectElement | HTMLInputElement {
  const label = Array.from(document.querySelectorAll("label"))
    .find((l) => l.querySelector(".settings-field-label")?.textContent === text);
  if (!label) throw new Error(`no field ${text}`);
  return label.querySelector("select, input")!;
}

/** From placement to review with two devices, addresses allocated, in the kitchen. */
async function toReview() {
  await change(labelled("Building part"), "30");
  await click(button("Next"));
  await change(labelled("Quantity"), "2");
  const allocate = Array.from(document.querySelectorAll<HTMLLabelElement>("label.device-wizard-check"))
    .find((l) => l.textContent?.includes("Assign free addresses"))!.querySelector("input")!;
  await click(allocate);
  await click(button("Next"));
}

describe("DeviceWizard (ADR-0093)", () => {
  it("previews on the server and creates exactly the previewed devices in one request", async () => {
    apiMock.previewDevices.mockResolvedValue(PREVIEW);
    apiMock.createDevice.mockResolvedValue({
      tree: tree(), diagnostics: [], replayed: false,
      items: [
        { index: 1, deviceId: 40, name: "Switch actuator 1", address: "1.1.1", diagnostics: [] },
        { index: 2, deviceId: 41, name: "Switch actuator 2", address: "1.1.2", diagnostics: [] },
      ],
    });
    const { onCreated, onOpenDevice, onClose } = await render({ product: PRODUCT });
    // Opened from the catalog: product step skipped, line pre-selected.
    expect(document.querySelector(".project-wizard-step-title")?.textContent).toContain("Placement");
    expect(labelled("Line").value).toBe("7");
    await toReview();

    expect(apiMock.previewDevices).toHaveBeenCalledExactlyOnceWith({
      lineId: 7, catalogItemId: PRODUCT.id, name: "Switch actuator", quantity: 2,
      allocateAddresses: true, uniqueNames: false, installationId: 0, buildingPartId: 30,
    });
    expect(document.body.textContent).toContain("Switch actuator 2");
    expect(document.body.textContent).toContain("1.1.2");

    await click(button("Create 2 devices"));
    expect(apiMock.createDevice).toHaveBeenCalledTimes(1);
    const [lineId, itemId, name, quantity, requestId, options] = apiMock.createDevice.mock.calls[0];
    expect([lineId, itemId, name, quantity]).toEqual([7, PRODUCT.id, "Switch actuator", 2]);
    expect(requestId).toMatch(/^[A-Za-z0-9_-]{1,128}$/);
    expect(options).toEqual({
      allocateAddresses: true, uniqueNames: false, installationId: 0, buildingPartId: 30,
      expected: [{ name: "Switch actuator 1", address: "1.1.1" }, { name: "Switch actuator 2", address: "1.1.2" }],
    });
    expect(onCreated).toHaveBeenCalledTimes(1);
    expect(document.body.textContent).toContain("2 devices created.");

    await click(button("Open device"));
    expect(onOpenDevice).toHaveBeenCalledExactlyOnceWith(40);
    expect(onClose).toHaveBeenCalled();
  });

  it("refreshes the preview after a 409 stale refusal and creates nothing", async () => {
    apiMock.previewDevices.mockResolvedValueOnce(PREVIEW).mockResolvedValueOnce({
      ...PREVIEW,
      items: [
        { index: 1, name: "Switch actuator 1", address: "1.1.2" },
        { index: 2, name: "Switch actuator 2", address: "1.1.3" },
      ],
    });
    const stale = Object.assign(new Error("preview stale"), { status: 409, body: { kind: "catalogPreviewStale" } });
    apiMock.createDevice.mockRejectedValue(stale);
    const { onCreated } = await render({ product: PRODUCT });
    await toReview();
    await click(button("Create 2 devices"));

    expect(onCreated).not.toHaveBeenCalled();
    expect(apiMock.previewDevices).toHaveBeenCalledTimes(2);
    expect(document.body.textContent).toContain("The project changed since this preview");
    expect(document.body.textContent).toContain("1.1.3");
    // Still on review, ready to confirm the new preview.
    expect(button("Create 2 devices").disabled).toBe(false);
  });

  it("disables address allocation without a line and sends no line", async () => {
    apiMock.previewDevices.mockResolvedValue({ items: [{ index: 1, name: "Switch actuator", address: null }], diagnostics: [], installationId: 0 });
    await render({ product: PRODUCT, target: { buildingPartId: 30 } });
    expect(labelled("Line").value).toBe("");
    expect(labelled("Building part").value).toBe("30");
    await click(button("Next"));
    const allocate = Array.from(document.querySelectorAll<HTMLLabelElement>("label.device-wizard-check"))
      .find((l) => l.textContent?.includes("Assign free addresses"))!.querySelector("input")!;
    expect(allocate.disabled).toBe(true);
    await click(button("Next"));
    expect(apiMock.previewDevices).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({
      lineId: null, allocateAddresses: false, buildingPartId: 30,
    }));
    expect(document.body.textContent).toContain("no address");
  });

  it("picks a product in step one and asks before discarding a choice", async () => {
    apiMock.catalogManufacturers.mockResolvedValue([]);
    apiMock.catalogItems.mockResolvedValue([PRODUCT]);
    const { onClose } = await render();
    expect(button("Next").disabled).toBe(true);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    await click(button("Switch actuatorSA 4"));
    expect(button("Next").disabled).toBe(false);
    await click(button("Cancel"));
    expect(onClose).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain("Discard your entries?");
    await click(button("Keep editing"));
    expect(document.body.textContent).not.toContain("Discard your entries?");
    await click(button("Cancel"));
    await click(button("Discard"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes at once when nothing was chosen yet", async () => {
    apiMock.catalogManufacturers.mockResolvedValue([]);
    apiMock.catalogItems.mockResolvedValue([]);
    const { onClose } = await render();
    await click(button("Cancel"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
