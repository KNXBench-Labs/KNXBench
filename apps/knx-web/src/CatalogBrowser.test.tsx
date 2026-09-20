/** Tests for CatalogBrowser's install/refresh, diagnostic translation, and keyboard navigation. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  PRODUCT_LANGUAGE_STORAGE_KEY,
  resetProductLanguageForTests,
  useProductLanguage,
} from "./productLanguage";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

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
  window.localStorage.removeItem(PRODUCT_LANGUAGE_STORAGE_KEY);
  resetProductLanguageForTests();
  window.localStorage.removeItem("knx-desktop:ui-language");
  resetUiLanguageForTests();
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

const item2 = {
  id: "cat-2",
  manufacturerId: "M-1",
  name: "Dimmer",
  number: null,
  visibleDescription: null,
  productRefId: "P-2",
  hardware2programRefId: "HP-2",
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

    expect(apiMock.catalogItems).toHaveBeenLastCalledWith("M-2", undefined, null);
    expect(host!.textContent).toContain("Installed: scheme 11");
    expect(host!.textContent).not.toContain("stored, not verified");
    root.unmount();
  });

  // KNOWN_LIMITATIONS.md §85: a `.signature` member is stored, never
  // checked. The server already qualifies its role text; this test pins
  // that the report a person actually reads says so too, in English.
  it("flags an install report containing a signature member as unverified", async () => {
    apiMock.installProductPackage.mockResolvedValueOnce({
      sha256: "abc",
      scheme: 11,
      skipped: false,
      members: [
        { path: "M-0001.signature", role: "Signature (stored, not verified)", sha256: "def", size: 175 },
        { path: "M-0001/Catalog.xml", role: "Catalog", sha256: "ghi", size: 42 },
      ],
      unknown: 0,
      conflicts: 0,
    });
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
    saveUiLanguage(window.localStorage, "de");
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
    window.localStorage.setItem(PRODUCT_LANGUAGE_STORAGE_KEY, "de-DE");
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

    const button = host.querySelector("button")!;
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
