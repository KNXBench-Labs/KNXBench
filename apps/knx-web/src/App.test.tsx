// @vitest-environment happy-dom
//
// KNOWN_LIMITATIONS.md #36, part A: the Log tab used to be unreachable
// without an open project, even though `GET /api/log` deliberately works
// with none (a failed import with nothing loaded still leaves a trail).
// These two tests pin the fix: the "Log" button is always enabled, and
// opening it renders `LogPanel` regardless of whether a project is open —
// while, with a project open, the tab still swaps into the same
// `.workspace` slot Inspector/Dashboard use, exactly as before.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { LogEntry } from "./api";
import { PRODUCT_LANGUAGE_STORAGE_KEY, resetProductLanguageForTests, useProductLanguage } from "./productLanguage";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  importProject: vi.fn(),
  getSessionLog: vi.fn().mockResolvedValue([]),
  productLanguages: vi.fn().mockResolvedValue([]),
  deviceDetail: vi.fn(),
  // `Inspector` renders `ParameterPanel` unconditionally once a device's
  // detail has loaded (see `Inspector.tsx`'s own comment on why), and
  // `ParameterPanel` fetches on mount — every test in the T33 describe
  // block below selects a device, so this needs a resolvable default the
  // same way `getSessionLog`/`productLanguages` already get one.
  deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }),
}));

const filePickerMock = vi.hoisted(() => ({
  pickOpenPath: vi.fn(),
  pickSavePath: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("./filePicker", () => ({ ...filePickerMock }));

import App from "./App";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  apiMock.getSessionLog.mockResolvedValue([]);
  apiMock.productLanguages.mockResolvedValue([]);
  apiMock.deviceParameters.mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] });
  window.localStorage.removeItem(PRODUCT_LANGUAGE_STORAGE_KEY);
  resetProductLanguageForTests();
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  document.documentElement.removeAttribute("lang");
  resetUiLanguageForTests();
});

function baseTree(): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    installations: [],
  };
}

// T33: one unassigned device, just enough tree for `ProjectExplorer` to
// render a clickable row without needing a full area/line topology — the
// device-detail-language fetch this exercises doesn't care where in the
// tree the device sits.
function deviceNode(): DeviceNode {
  return { id: 42, name: "Device D", address: null, description: null, com_object_count: 0 };
}

function treeWithDevice(): ProjectTree {
  return {
    ...baseTree(),
    installations: [
      {
        id: 1,
        name: "Installation",
        topology: [],
        buildings: [],
        unassigned: [deviceNode()],
        group_addresses: [],
        group_ranges: [],
      },
    ],
  };
}

function deviceDetailFixture(): DeviceDetail {
  return { id: 42, name: "Device D", description: null, address: null, com_objects: [] };
}

function entry(overrides: Partial<LogEntry>): LogEntry {
  return {
    timestamp: "2026-09-10T12:00:00Z",
    severity: "info",
    source: "open",
    message: "opened project",
    location: null,
    detail: null,
    ...overrides,
  };
}

async function renderApp() {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<App />);
  });
  return root;
}

function findButton(text: string): HTMLButtonElement {
  const button = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === text);
  if (!button) throw new Error(`button "${text}" not found`);
  return button;
}

describe("App — Log tab reachability (KNOWN_LIMITATIONS.md #36, part A)", () => {
  it("is reachable with no project open: the Log button is enabled and clicking it renders the panel", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ message: "a failed import with nothing open" }),
    ]);
    const root = await renderApp();

    const logButton = findButton("Log");
    expect(logButton.disabled).toBe(false);

    await act(async () => {
      logButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    // GET /api/log resolves asynchronously inside LogPanel's own effect.
    await act(async () => {});

    expect(host!.querySelector(".log-panel")).not.toBeNull();
    expect(host!.textContent).toContain("a failed import with nothing open");
    // No project is open, so the Explorer has nothing to show — the Log
    // tab is the only thing in the workspace slot, per the brief.
    expect(host!.querySelector(".project-explorer")).toBeNull();

    root.unmount();
  });

  it("with a project open, the Log tab still swaps into the same slot as Inspector/Dashboard", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    apiMock.getSessionLog.mockResolvedValue([entry({ message: "opened ok" })]);
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    expect(host!.querySelector(".project-explorer")).not.toBeNull();

    const logButton = findButton("Log");
    expect(logButton.disabled).toBe(false);
    await act(async () => {
      logButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    expect(host!.querySelector(".log-panel")).not.toBeNull();
    expect(host!.querySelector(".project-explorer")).not.toBeNull();
    expect(host!.textContent).toContain("opened ok");

    // Closing the Log tab returns to the Dashboard (no selection yet), as
    // before.
    await act(async () => {
      logButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".log-panel")).toBeNull();

    root.unmount();
  });
});

// T33 Task 3: the Inspector's device-detail fetch forwards the active
// product language, exactly the way `ParameterPanel.test.tsx`'s "sends the
// active product language" tests already prove for the parameter panel's
// own fetch — and, since `selectEntity`/`handleTreeUpdate` are ordinary
// event handlers rather than an effect, a dedicated effect in `App.tsx`
// covers the "language changes while a device stays selected" case
// (`CatalogBrowser.test.tsx`'s "refetches ... when open" test drives that
// same scenario for its own fetch the same way).
describe("App — device-detail fetch carries the product language (T33)", () => {
  async function openProjectWithDevice() {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(treeWithDevice());
    apiMock.deviceDetail.mockResolvedValue(deviceDetailFixture());
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
    return root;
  }

  function deviceLabel(): HTMLElement {
    const label = Array.from(host!.querySelectorAll<HTMLElement>(".tree-label")).find(
      (el) => el.textContent === "Device D",
    );
    if (!label) throw new Error('tree-label "Device D" not found');
    return label;
  }

  it("passes no product language to deviceDetail when none is set", async () => {
    const root = await openProjectWithDevice();

    await act(async () => {
      deviceLabel().dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });
    // `productLanguage` is `null` (nothing in `localStorage`) — this pins
    // the value `App` hands to `api.deviceDetail` for that case; whether
    // `languageQuery(null)` then turns it into a bare URL with no
    // `?language=` is `api.ts`'s own concern, not this component's.
    expect(apiMock.deviceDetail).toHaveBeenLastCalledWith(42, null);

    root.unmount();
  });

  it("carries the stored language once a device is selected", async () => {
    window.localStorage.setItem(PRODUCT_LANGUAGE_STORAGE_KEY, "de-DE");
    const root = await openProjectWithDevice();

    await act(async () => {
      deviceLabel().dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.deviceDetail).toHaveBeenLastCalledWith(42, "de-DE");

    root.unmount();
  });

  it("refetches the selected device's detail when the language changes underneath it", async () => {
    // `App` has no UI of its own that changes `productLanguage` in this
    // test's reach (the real control lives in `SettingsPanel`, several
    // props away) — a sibling `Writer` reading/writing the same
    // `useProductLanguage()` store is the same trick
    // `CatalogBrowser.test.tsx`'s equivalent test uses to flip the setting
    // out from under an already-mounted consumer.
    function Writer() {
      const [, setLanguage] = useProductLanguage();
      return (
        <button type="button" onClick={() => setLanguage("fr-FR")}>
          set fr-FR
        </button>
      );
    }

    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(treeWithDevice());
    apiMock.deviceDetail.mockResolvedValue(deviceDetailFixture());
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(
        <>
          <App />
          <Writer />
        </>,
      );
    });

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    await act(async () => {
      deviceLabel().dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.deviceDetail).toHaveBeenLastCalledWith(42, null);

    const writerButton = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.textContent === "set fr-FR",
    )!;
    await act(async () => {
      writerButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    // Same device id, new language — the selection never changed, only
    // the setting did, which is exactly the case `selectEntity` and
    // `handleTreeUpdate` don't cover on their own.
    expect(apiMock.deviceDetail).toHaveBeenLastCalledWith(42, "fr-FR");

    root.unmount();
  });

  // Fix round 1 regression test: two language changes in quick succession
  // race each other, not just the selection. The selection-identity guard
  // `selectEntity`/`handleTreeUpdate` already carry is always satisfied
  // here — the selection never moves across a language change — so on its
  // own it cannot tell the de-DE reply and the fr-FR reply apart. Only a
  // per-request generation counter (`languageRequestIdRef` in `App.tsx`)
  // can, and only by discarding whichever reply is no longer current
  // rather than whichever happens to have started first.
  it("keeps the newer language's detail even when the older language's response resolves later", async () => {
    function Writer() {
      const [, setLanguage] = useProductLanguage();
      return (
        <>
          <button type="button" onClick={() => setLanguage("de-DE")}>
            set de-DE
          </button>
          <button type="button" onClick={() => setLanguage("fr-FR")}>
            set fr-FR
          </button>
        </>
      );
    }

    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(treeWithDevice());
    // The initial selection's own fetch (language `null`) resolves right
    // away. The two language-change fetches that follow are held open by
    // the test — deliberately released out of request order below — so
    // one call queues to each of the two `mockImplementationOnce`s in the
    // order `App` is expected to issue them: de-DE first, fr-FR second.
    let resolveDe: ((detail: DeviceDetail) => void) | undefined;
    let resolveFr: ((detail: DeviceDetail) => void) | undefined;
    apiMock.deviceDetail
      .mockResolvedValueOnce(deviceDetailFixture())
      .mockImplementationOnce(
        () =>
          new Promise<DeviceDetail>((resolve) => {
            resolveDe = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<DeviceDetail>((resolve) => {
            resolveFr = resolve;
          }),
      );

    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(
        <>
          <App />
          <Writer />
        </>,
      );
    });

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    await act(async () => {
      deviceLabel().dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    const deButton = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.textContent === "set de-DE",
    )!;
    const frButton = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.textContent === "set fr-FR",
    )!;

    // Two rapid language changes: de-DE requested first, fr-FR second —
    // both requests are now in flight, with de-DE's `languageRequestIdRef`
    // generation the older of the two.
    await act(async () => {
      deButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      frButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.deviceDetail).toHaveBeenCalledTimes(3);
    expect(resolveDe).toBeDefined();
    expect(resolveFr).toBeDefined();

    // Resolve out of order: the OLDER request (de-DE) answers LAST.
    await act(async () => {
      resolveFr!({ ...deviceDetailFixture(), name: "Device D (fr-FR)" });
      await Promise.resolve();
    });
    await act(async () => {
      resolveDe!({ ...deviceDetailFixture(), name: "Device D (de-DE)" });
      await Promise.resolve();
    });

    // Without `languageRequestIdRef`, the de-DE reply — delivered last —
    // would silently overwrite the fr-FR detail already on screen, even
    // though `productLanguage` has been "fr-FR" the whole time.
    expect(host!.textContent).toContain("Device D (fr-FR)");
    expect(host!.textContent).not.toContain("Device D (de-DE)");

    root.unmount();
  });
});

// T25 task 3: before this change, `useUiLanguage()`'s `document
// .documentElement.lang`-setting effect only ran once `SettingsPanel`
// mounted, since it was the sole caller of the hook — leaving
// `index.html`'s static `lang="en"` in place for a whole session that
// never opens Settings. `App` now calls `useTranslate()` (which calls
// `useUiLanguage()` internally) for its own toolbar labels, unconditional
// on every render, closing that gap.
describe("App — <html lang> reflects the UI language without opening Settings", () => {
  it("sets lang=\"en\" on a fresh mount, before Settings is ever opened", async () => {
    const root = await renderApp();

    expect(document.documentElement.getAttribute("lang")).toBe("en");
    expect(host!.querySelector(".settings-panel")).toBeNull();

    root.unmount();
  });
});
