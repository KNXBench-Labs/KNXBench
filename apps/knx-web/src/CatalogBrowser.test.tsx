// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  catalogManufacturers: vi.fn().mockResolvedValue([]),
  catalogItems: vi.fn().mockResolvedValue([]),
  installProductPackage: vi.fn(),
  createDevice: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import CatalogBrowser from "./CatalogBrowser";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  apiMock.catalogManufacturers.mockResolvedValue([]);
  apiMock.catalogItems.mockResolvedValue([]);
});

const item = {
  id: "cat-1",
  manufacturerId: "M-1",
  name: "Actuator",
  number: null,
  visibleDescription: null,
  productRefId: "P-1",
  hardware2programRefId: "HP-1",
};

async function renderBrowser(onCreated = vi.fn(), onClose = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<CatalogBrowser lineId={null} onCreated={onCreated} onClose={onClose} />);
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
    apiMock.installProductPackage.mockResolvedValueOnce({
      sha256: "abc", scheme: 11, skipped: false, members: [], unknown: 0, conflicts: 0,
    });
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

    expect(apiMock.catalogItems).toHaveBeenLastCalledWith("M-2", undefined);
    expect(host!.textContent).toContain("Installed: scheme 11");
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
});
