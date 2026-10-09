/** Tests for App's log-tab reachability, language-aware device fetches, and document language. */
// @vitest-environment happy-dom
//
// KNOWN_LIMITATIONS.md #36, part A: the Log tab used to be unreachable
// without an open project, even though `GET /api/log` deliberately works
// with none (a failed import with nothing loaded still leaves a trail).
// These two tests pin the fix: the "Log" button is always enabled, and
// opening it renders `LogPanel` regardless of whether a project is open —
// while, with a project open, the tab still swaps into the same
// `.workspace` slot Inspector/Dashboard use, exactly as before.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { version as packageVersion } from "../package.json";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { LogEntry } from "./api";
import { messages as enMessages } from "./messages/en";
import { messages as deMessages } from "./messages/de";
import { PRODUCT_LANGUAGE_STORAGE_KEY, resetProductLanguageForTests, useProductLanguage } from "./productLanguage";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  importProject: vi.fn(),
  // ADR-0094 (L3): Settings asks whether a legacy password is remembered.
  legacyPasswordStatus: vi.fn().mockResolvedValue({ available: true, remembered: false, problem: null }),
  forgetLegacyPassword: vi.fn(),
  // F14: `openNativeProject`'s own function, distinct from `importProject`
  // above — the "shows the banner" test below pins that the `.knxdb`
  // button reaches this one and not its ETS-import sibling.
  openProject: vi.fn(),
  // The welcome screen's third button (the from-scratch launcher) calls
  // this through `NewProjectDialog`; every other test here renders that
  // dialog not at all, so an unconfigured `vi.fn()` is enough for them.
  newProject: vi.fn(),
  saveProject: vi.fn().mockResolvedValue(undefined),
  saveProjectAs: vi.fn().mockResolvedValue(undefined),
  getSessionLog: vi.fn().mockResolvedValue([]),
  productLanguages: vi.fn().mockResolvedValue([]),
  deviceDetail: vi.fn(),
  deviceCatalog: vi.fn().mockResolvedValue({ schemaVersion: 1, serverIncarnation: null, snapshotRevision: null, devices: [] }),
  // Only the "edit-triggered refetch races a language reply" regression
  // test below drives this — it needs `handleTreeUpdate`'s own fetch to
  // fire from a real command, and `undo` is the cheapest one on the
  // toolbar.
  undo: vi.fn(),
  redo: vi.fn(),
  // The File menu's Compare entry (stage 4, item 4's keyboard walk).
  diffProject: vi.fn(),
  // The File menu's documentation entry opens a dialog that previews on
  // mount; a promise that never settles keeps that dialog inert.
  previewDocumentation: vi.fn(() => new Promise(() => {})),
  // `Inspector` renders `ParameterPanel` unconditionally once a device's
  // detail has loaded (see `Inspector.tsx`'s own comment on why), and
  // `ParameterPanel` fetches on mount — every test in the T33 describe
  // block below selects a device, so this needs a resolvable default the
  // same way `getSessionLog`/`productLanguages` already get one.
  deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }),
  // `BusMonitorPanel` asks for the current session on mount (see its
  // reattach effect); the palette-reachability tests below render it, and
  // a 404 is the "no session yet" answer that leaves the connect form up.
  pollBusTelegrams: vi.fn().mockRejectedValue(new Error("no session")),
  errorStatus: vi.fn().mockReturnValue(404),
  // ADR-0023's progress poll. `null` is the honest default for a server
  // that has loaded nothing, and the answer every test here wants except
  // the two that drive a load on purpose.
  loadProgress: vi.fn().mockResolvedValue(null),
  currentProject: vi.fn(),
  // `CatalogBrowser` fires both of these on mount. The help tests below
  // open it for real (it is the third dialog F1 has to replace), and an
  // unconfigured `vi.fn()` returns `undefined`, on which the component
  // promptly calls `.then`.
  catalogManufacturers: vi.fn().mockResolvedValue([]),
  catalogItems: vi.fn().mockResolvedValue([]),
  // T28/F5: `AboutDialog` asks the server which build this is the moment
  // it mounts, rather than reading a constant the frontend would have to
  // remember to bump.
  serverVersion: vi.fn().mockResolvedValue({ version: "0.0.0-test" }),
  // T25: `App` starts one interface search in the background on mount.
  // An empty result is the honest default for a test machine with no KNX
  // installation on its network, and it keeps the search out of every
  // test here that is about something else.
  discoverBusInterfaces: vi.fn().mockResolvedValue({ interfaces: [] }),
  moveDeviceToLine: vi.fn(),
  moveDeviceToBuildingPart: vi.fn(),
  setDeviceDescription: vi.fn(),
}));

const filePickerMock = vi.hoisted(() => ({
  pickOpenPath: vi.fn(),
  pickSavePath: vi.fn(),
  // T28/F4: `quit.ts`'s `canQuit()` is `isTauri()` and nothing else, and
  // `App` asks it on every render to decide whether the File menu carries
  // a Quit item. This suite replaces `./filePicker` wholesale, so the
  // export has to be here or every render in the file throws. `false` is
  // the default because these tests are a browser, not the desktop shell;
  // the T28 block below flips it for the two tests that care.
  isTauri: vi.fn(() => false),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  isUnsavedChangesConflict: (error: unknown) =>
    error instanceof Error && (error as Error & { status?: number }).status === 409,
  // Same predicate as `api.ts`'s; App only reaches it after a failed load.
  isUnsavedProjectConflict: (error: unknown) => {
    const { status, body } = (error ?? {}) as { status?: unknown; body?: { kind?: unknown } | null };
    return status === 409 && body?.kind === "projectUnsavedChanges";
  },
}));

vi.mock("./filePicker", () => ({ ...filePickerMock }));

// T28/F4: `quit.ts` imports this lazily, so only the tests that actually
// press Quit ever reach it — but the mock has to be declared up here all
// the same, and a real `getCurrentWindow()` outside the shell throws.
// §132: `onCloseRequested` records the handler App registers, so a test can
// play the window manager's × by calling `tauriCloseRequested`.
const tauriWindowMock = vi.hoisted(() => {
  const mock = {
    destroy: vi.fn().mockResolvedValue(undefined),
    // Never called: `close()` would re-emit close-requested and reopen the
    // dialog "discard" just dismissed (§132), so `quitApp()` uses `destroy()`.
    close: vi.fn().mockResolvedValue(undefined),
    closeHandler: null as null | ((event: { preventDefault: () => void }) => void),
    onCloseRequested: vi.fn(async (handler: (event: { preventDefault: () => void }) => void) => {
      mock.closeHandler = handler;
      return () => {
        if (mock.closeHandler === handler) mock.closeHandler = null;
      };
    }),
  };
  return mock;
});

/** Plays the window manager's close; returns whether the window may close. */
function tauriCloseRequested(): boolean {
  let prevented = false;
  tauriWindowMock.closeHandler?.({ preventDefault: () => { prevented = true; } });
  return !prevented;
}
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => tauriWindowMock }));

import App from "./App";
import type { SessionControls } from "./session";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { resetSettingsForTests, setSetting, settingsStorage } from "./settingsStore";

// F9's client half: every load generates its own token via
// `crypto.randomUUID()` before the POST. Pinning it to a fixed value
// here lets each test below say, plainly, which snapshots are ours and
// which belong to a stranger.
const OWN_CLIENT_TOKEN = "11111111-1111-1111-1111-111111111111";
vi.spyOn(crypto, "randomUUID").mockReturnValue(OWN_CLIENT_TOKEN);

let host: HTMLDivElement | undefined;

beforeEach(() => {
  // Startup jokes intentionally depend on local wall-clock time. Freeze only
  // Date (not timers) so unrelated App assertions behave the same at noon,
  // midnight, and on CI runners in another timezone.
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-02-03T12:00:00Z"));
  // Most `/api/` calls in this file go through the mocked `./api`; two
  // modules talk HTTP by themselves: the achievement tracker (ADR-0089) and
  // the settings record's server-confirmed reads (`settingsStore.ts`, used
  // by the Settings panel). happy-dom's real `fetch` would reach whatever
  // listens on localhost:3000, so every request is refused here — the same
  // answer as an unreachable server, which both handle — and any other URL
  // is recorded as a leak.
  unexpectedFetches.length = 0;
  vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (!EXPECTED_SELF_FETCHES.some((path) => url.includes(path))) unexpectedFetches.push(url);
    throw new TypeError(`no network in unit tests: ${url}`);
  }));
});

const unexpectedFetches: string[] = [];
const EXPECTED_SELF_FETCHES = ["/api/achievements", "/api/settings"];

afterEach(() => {
  vi.unstubAllGlobals();
  host?.remove();
  host = undefined;
  // App publishes accepted server lifetimes to the same origin-wide record
  // companion windows consume. A test's synthetic process identity must not
  // become the next test's persisted browser history.
  window.localStorage.clear();
  vi.clearAllMocks();
  apiMock.getSessionLog.mockResolvedValue([]);
  apiMock.productLanguages.mockResolvedValue([]);
  apiMock.deviceParameters.mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] });
  // `clearAllMocks` keeps implementations, including a queued
  // `mockResolvedValueOnce`, so the §96 test's first-snapshot answer
  // would otherwise leak to whoever runs next.
  apiMock.loadProgress.mockReset();
  apiMock.loadProgress.mockResolvedValue(null);
  apiMock.currentProject.mockReset();
  resetSettingsForTests();
  resetProductLanguageForTests();
  document.documentElement.style.removeProperty("--app-ui-scale");
  document.documentElement.removeAttribute("lang");
  document.title = "";
  resetUiLanguageForTests();
  vi.useRealTimers();
  // Last, so a failure here cannot skip the cleanup above.
  expect(unexpectedFetches, "App must not reach the network outside the mocked api").toEqual([]);
});

function baseTree(): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [],
  };
}

function treeAt(tree: ProjectTree, snapshotRevision: number): ProjectTree {
  return { ...tree, snapshot_revision: snapshotRevision } as ProjectTree;
}

function treeFromProcess(
  tree: ProjectTree,
  serverIncarnation: string,
  snapshotRevision: number,
): ProjectTree {
  return {
    ...tree,
    server_incarnation: serverIncarnation,
    snapshot_revision: snapshotRevision,
  } as ProjectTree;
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

function treeWithDragTargets(): ProjectTree {
  const device = { ...deviceNode(), id: 1, name: "Device A" };
  return {
    ...baseTree(),
    installations: [{
      id: 1,
      name: "Installation",
      topology: [{
        id: 10,
        name: "Area A",
        address: 1,
        lines: [{ id: 11, name: "Line A", address: 1, devices: [device] }],
      }],
      buildings: [{ id: 501, name: "Room A", kind: "Room", children: [], devices: [] }],
      unassigned: [],
      group_addresses: [],
      group_ranges: [],
    }],
  };
}

class TestDataTransfer {
  private readonly values = new Map<string, string>();
  dropEffect: DataTransfer["dropEffect"] = "none";
  effectAllowed: DataTransfer["effectAllowed"] = "uninitialized";
  get types(): string[] { return [...this.values.keys()]; }
  setData(format: string, data: string): void { this.values.set(format, data); }
  getData(format: string): string { return this.values.get(format) ?? ""; }
}

function deviceDetailFixture(): DeviceDetail {
  return { id: 42, name: "Device D", description: null, address: null, com_objects: [],
    product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };
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

async function renderApp(manifestVersion?: string, session?: SessionControls) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<App manifestVersion={manifestVersion} session={session} />);
  });
  return root;
}

describe("App manifest version", () => {
  it("uses package.json for the footer and document title by default", async () => {
    const root = await renderApp();

    expect(host!.querySelector(".workbench-status > span:last-child")?.textContent)
      .toBe(`v${packageVersion}`);
    expect(document.title).toBe(`KNXBench ${packageVersion}`);

    await act(async () => root.unmount());
  });

  it("shows the injected manifest version at the right edge of the footer", async () => {
    const root = await renderApp("98.76.54-test");

    expect(host!.querySelector(".workbench-status > span:last-child")?.textContent)
      .toBe("v98.76.54-test");

    await act(async () => root.unmount());
  });

  it("sets the document title from the injected manifest version", async () => {
    const root = await renderApp("98.76.54-test");

    expect(document.title).toBe("KNXBench 98.76.54-test");

    await act(async () => root.unmount());
  });
});

describe("App — ISSUE-04 last-saved status", () => {
  it("uses the selected UI language and advances only after a successful save", async () => {
    saveUiLanguage(settingsStorage, "de");
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxdb");
    const first = "2026-02-03T10:20:00Z";
    const second = "2026-02-03T11:45:00Z";
    const dirty = treeAt({ ...baseTree(), is_modified: true, last_saved_at: first }, 1);
    apiMock.openProject.mockResolvedValueOnce(dirty);
    const root = await renderApp();
    const status = () => host!.querySelector(".workbench-status-saved")?.textContent;
    const germanTime = (iso: string) => new Intl.DateTimeFormat("de", {
      dateStyle: "short", timeStyle: "medium",
    }).format(new Date(iso));

    await act(async () => {
      host!.querySelector<HTMLButtonElement>('[aria-labelledby="welcome-native-title"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(status()).toContain(germanTime(first));

    apiMock.saveProject.mockRejectedValueOnce(new Error("disk full"));
    await act(async () => {
      findButton("Speichern").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(status()).toContain(germanTime(first));
    expect(status()).not.toContain(germanTime(second));

    apiMock.currentProject.mockResolvedValueOnce(treeAt({ ...dirty, is_modified: false, last_saved_at: second }, 2));
    await act(async () => {
      findButton("Speichern").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(status()).toContain(germanTime(second));
    expect(status()).not.toContain(germanTime(first));
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
  });
});

/** Answers F1's open-over-unsaved-edits dialog with "discard". */
async function confirmReplaceDiscard() {
  const discard = Array.from(document.querySelectorAll<HTMLButtonElement>(".replace-confirm button"))
    .find((b) => b.textContent === enMessages["replace.discard"]);
  if (!discard) throw new Error("the replace-confirm dialog is not open");
  await act(async () => discard.click());
  await act(async () => {});
}

function findButton(text: string): HTMLButtonElement {
  const button = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === text);
  if (!button) throw new Error(`button "${text}" not found`);
  return button;
}

describe("App — bounded workbench zoom", () => {
  function zoomKey(key: string, target: EventTarget = window): KeyboardEvent {
    const event = new KeyboardEvent("keydown", { key, ctrlKey: true, bubbles: true, cancelable: true });
    target.dispatchEvent(event);
    return event;
  }

  it("uses Ctrl+Plus, Ctrl+Minus and Ctrl+0 without stealing editable-field keys", async () => {
    const root = await renderApp();
    await act(async () => { zoomKey("+"); });
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1.1");
    await act(async () => { zoomKey("-"); });
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1");
    await act(async () => { zoomKey("+"); zoomKey("0"); });
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1");
    const input = document.createElement("input");
    host!.append(input);
    input.focus();
    let editableEvent!: KeyboardEvent;
    await act(async () => { editableEvent = zoomKey("+", input); });
    expect(editableEvent.defaultPrevented).toBe(false);
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1");
    input.remove();
    await act(async () => root.unmount());
  });

  it("clamps persisted and repeatedly changed zoom to safe bounds", async () => {
    setSetting("uiScale", 20);
    const root = await renderApp();
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1.5");
    for (let i = 0; i < 12; i++) await act(async () => { zoomKey("-"); });
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("0.8");
    await act(async () => { zoomKey("0"); });
    expect(document.documentElement.style.getPropertyValue("--app-ui-scale")).toBe("1");
    await act(async () => root.unmount());
  });
});

describe("App — remembered workbench geometry", () => {
  it("stacks the inspector before enlarged saved widths can clip it", async () => {
    const original = Object.getOwnPropertyDescriptor(window, "innerWidth")!;
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 1280 });
    try {
      setSetting("navigationPaneWidth", 480);
      setSetting("inspectorPaneWidth", 700);
      setSetting("uiScale", 1.5);
      const root = await renderApp();
      expect(host!.querySelector(".workbench")?.classList.contains("workbench--stacked-inspector")).toBe(true);
      await act(async () => root.unmount());
    } finally {
      Object.defineProperty(window, "innerWidth", original);
    }
  });

  it("keeps a resized navigation pane across hide/show and remount; clamps both persisted widths", async () => {
    setSetting("navigationPaneWidth", 10000);
    setSetting("inspectorPaneWidth", -1);
    let root = await renderApp();
    const navigation = () => host!.querySelector<HTMLElement>(".workbench-pane-left")!;
    const inspector = () => host!.querySelector<HTMLElement>(".workbench-pane-right")!;
    expect(navigation().style.width).toBe("480px");
    expect(inspector().style.width).toBe("280px");
    const resizer = navigation().querySelector<HTMLElement>('[role="separator"]')!;
    await act(async () => resizer.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })));
    expect(navigation().style.width).toBe("200px");
    await act(async () => findButton("Navigation").click());
    expect(host!.querySelector(".workbench-pane-left")).toBeNull();
    await act(async () => findButton("Navigation").click());
    expect(navigation().style.width).toBe("200px");
    await act(async () => root.unmount());
    host?.remove();
    root = await renderApp();
    expect(navigation().style.width).toBe("200px");
    expect(inspector().style.width).toBe("280px");
    await act(async () => root.unmount());
  });
});

function treeLabel(text: string): HTMLElement {
  const label = Array.from(host!.querySelectorAll<HTMLElement>(".tree-label"))
    .find((candidate) => candidate.textContent === text);
  if (!label) throw new Error(`tree label "${text}" not found`);
  return label;
}

async function dragTreeLabel(sourceText: string, targetText: string): Promise<void> {
  const transfer = new TestDataTransfer();
  for (const [text, type] of [
    [sourceText, "dragstart"],
    [targetText, "dragover"],
    [targetText, "drop"],
  ] as const) {
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: transfer });
    await act(async () => treeLabel(text).dispatchEvent(event));
  }
}

async function openDragProject(): Promise<ReturnType<typeof createRoot>> {
  filePickerMock.pickOpenPath.mockResolvedValue("/tmp/drag.knxproj");
  apiMock.importProject.mockResolvedValue(treeWithDragTargets());
  const root = await renderApp();
  await act(async () => {
    findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  await act(async () => {});
  return root;
}

describe("App — drag-and-drop announcement", () => {
  it("announces a successful line drop through the existing status toast", async () => {
    apiMock.moveDeviceToLine.mockResolvedValueOnce(treeWithDragTargets());
    const root = await openDragProject();

    await dragTreeLabel("Device A", "Line 1: Line A");

    expect([...host!.querySelectorAll('[role="status"]')]
      .some((toast) => toast.textContent?.includes("Device A moved to line Line 1: Line A."))).toBe(true);
    await act(async () => root.unmount());
  });

  it("announces a successful building-part drop in the active German UI", async () => {
    apiMock.moveDeviceToBuildingPart.mockResolvedValueOnce(treeWithDragTargets());
    const root = await openDragProject();
    await act(async () => setSetting("uiLanguage", "de"));

    await dragTreeLabel("Device A", "Room A (Raum)");

    expect([...host!.querySelectorAll('[role="status"]')]
      .some((toast) => toast.textContent?.includes("Device A wurde nach Room A verschoben."))).toBe(true);
    await act(async () => root.unmount());
  });

  it("reports a rejected drop as an alert and keeps the old tree visible", async () => {
    apiMock.moveDeviceToLine.mockRejectedValueOnce(new Error("move refused"));
    const root = await openDragProject();

    await dragTreeLabel("Device A", "Line 1: Line A");

    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("move refused");
    expect(treeLabel("Device A")).toBeTruthy();
    await act(async () => root.unmount());
  });
});

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

describe("App — catalog is a persistent centre workspace (ISSUE-07)", () => {
  it("keeps the chosen product and search while switching to Overview and back", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    apiMock.catalogItems.mockResolvedValue([{ id: "cat-1", manufacturerId: "M-1", name: "Actuator",
      number: null, visibleDescription: null, productRefId: "P-1", hardware2programRefId: "HP-1" }]);
    const root = await renderApp();
    await act(async () => findButton("Open project…").click());
    await act(async () => findButton(enMessages["workbench.catalog"]).click());
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });

    const workspace = host!.querySelector<HTMLElement>(".workbench-center .catalog-workspace");
    expect(workspace).not.toBeNull();
    expect(workspace!.closest('[role="dialog"]')).toBeNull();
    const search = workspace!.querySelector<HTMLInputElement>('input[role="combobox"]')!;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(search, "Act");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => workspace!.querySelector<HTMLElement>(".search-result")!.click());
    expect(workspace!.querySelector<HTMLInputElement>(".catalog-create-row input")?.value).toBe("Actuator");

    await act(async () => findButton(enMessages["workbench.overview"]).click());
    expect(workspace!.hidden).toBe(true);
    await act(async () => findButton(enMessages["workbench.catalog"]).click());
    expect(workspace!.hidden).toBe(false);
    expect(workspace!.querySelector<HTMLInputElement>('input[role="combobox"]')?.value).toBe("Act");
    expect(workspace!.querySelector<HTMLInputElement>(".catalog-create-row input")?.value).toBe("Actuator");
    await act(async () => root.unmount());
  });

  it("forgets a previous project's catalog selection when another project loads", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    apiMock.catalogItems.mockResolvedValue([{ id: "cat-1", manufacturerId: "M-1", name: "Actuator",
      number: null, visibleDescription: null, productRefId: "P-1", hardware2programRefId: "HP-1" }]);
    const root = await renderApp();
    await act(async () => findButton("Open project…").click());
    await act(async () => findButton(enMessages["workbench.catalog"]).click());
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 250)); });
    await act(async () => host!.querySelector<HTMLElement>(".catalog-workspace .search-result")!.click());
    expect(host!.querySelector(".catalog-create-row")).not.toBeNull();

    await act(async () => findButton("Open project…").click());
    await act(async () => findButton(enMessages["workbench.catalog"]).click());
    expect(host!.querySelector(".catalog-create-row")).toBeNull();
    expect(host!.querySelector<HTMLInputElement>('.catalog-workspace input[role="combobox"]')?.value).toBe("");
    await act(async () => root.unmount());
  });

  it("shows the catalogue immediately on narrow screens instead of below a full-height navigation", async () => {
    const originalWidth = window.innerWidth;
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 400 });
    try {
      const root = await renderApp();
      await act(async () => findButton(enMessages["workbench.catalog"]).click());
      expect(findButton(enMessages["workbench.navigation"]).getAttribute("aria-expanded")).toBe("false");
      expect(host!.querySelector<HTMLElement>(".catalog-workspace")?.hidden).toBe(false);
      await act(async () => root.unmount());
    } finally {
      Object.defineProperty(window, "innerWidth", { configurable: true, value: originalWidth });
    }
  });
});

// Stage 5 audit: the workbench shell moved the Log and Bus monitor
// buttons into the left `ResizablePane`, which the Navigation toggle can
// collapse — at which point the panels had no entry point at all, since
// neither had a command. These tests drive the palette with the pane
// collapsed, which is the state the toolbar-button tests above cannot
// reach.
describe("App — diagnostic panels survive a collapsed navigation pane", () => {
  function clickButton(button: HTMLButtonElement) {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  }

  function paletteOption(label: string): HTMLElement {
    const option = Array.from(host!.querySelectorAll<HTMLElement>("li[role=option]")).find(
      (li) => li.textContent === label,
    );
    if (!option) throw new Error(`palette option "${label}" not found`);
    return option;
  }

  async function collapseNavigationAndOpen(label: string) {
    await act(async () => clickButton(findButton("Navigation")));
    expect(host!.querySelector(".diagnostic-navigation")).toBeNull();
    await act(async () => clickButton(findButton("Commands… (Ctrl+Shift+P)")));
    await act(async () => {
      paletteOption(label).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
  }

  it("reaches the Log from the command palette", async () => {
    apiMock.getSessionLog.mockResolvedValue([entry({ message: "still reachable" })]);
    const root = await renderApp();
    await collapseNavigationAndOpen("Log");
    expect(host!.querySelector(".log-panel")).not.toBeNull();
    expect(host!.textContent).toContain("still reachable");
    root.unmount();
  });

  it("reaches the Bus monitor from the command palette", async () => {
    const root = await renderApp();
    await collapseNavigationAndOpen("Bus monitor");
    expect(host!.querySelector(".bus-monitor-panel")).not.toBeNull();
    root.unmount();
  });

  it("reaches Settings from the command palette", async () => {
    const root = await renderApp();
    await collapseNavigationAndOpen("Settings");
    expect(host!.querySelector(".settings-panel")).not.toBeNull();
    // ADR-0094 (L3): the real app wires the remembered legacy password in.
    expect(host!.querySelector(".settings-section-legacy-password")).not.toBeNull();
    expect(apiMock.legacyPasswordStatus).toHaveBeenCalled();
    root.unmount();
  });
});

describe("App — search reveal request", () => {
  // A tree click must remain a local navigation decision: only the external
  // search pick is allowed to reopen a branch the user has just collapsed.
  it("reveals a search selection but leaves an ordinary explorer selection without a new reveal", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/search.knxproj");
    apiMock.importProject.mockResolvedValue(treeWithDevice());
    apiMock.deviceDetail.mockResolvedValueOnce(deviceDetailFixture());
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    const unassignedToggle = () => host!.querySelector<HTMLButtonElement>(
      '.tree-toggle[aria-label="Unassigned"]',
    )!;
    await act(async () => unassignedToggle().click());
    expect(() => treeLabel("Device D")).toThrow('tree label "Device D" not found');

    // Selecting Project is an ordinary explorer selection and must not
    // countermand the manual collapse.
    await act(async () => treeLabel("Project").click());
    expect(() => treeLabel("Device D")).toThrow('tree label "Device D" not found');

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", {
        key: "k", ctrlKey: true, bubbles: true, cancelable: true,
      }));
    });
    const input = host!.querySelector<HTMLInputElement>(".search-panel input")!;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "Device D");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    const result = Array.from(host!.querySelectorAll<HTMLElement>('[role="option"]'))
      .find((option) => option.textContent === "Device D")!;
    await act(async () => result.click());
    expect(treeLabel("Device D")).toBeTruthy();

    await act(async () => unassignedToggle().click());
    expect(() => treeLabel("Device D")).toThrow('tree label "Device D" not found');
    await act(async () => treeLabel("Project").click());
    expect(() => treeLabel("Device D")).toThrow('tree label "Device D" not found');

    // The request has completed at the selected row, so remounting the
    // navigator starts at its ordinary default-open state but cannot replay
    // the old scroll. (The row being present alone is not evidence here.)
    const previous = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollIntoView");
    const scrollIntoView = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: scrollIntoView,
    });
    await act(async () => findButton("Navigation").click());
    expect(host!.querySelector(".project-explorer")).toBeNull();
    await act(async () => findButton("Navigation").click());
    expect(treeLabel("Device D")).toBeTruthy();
    expect(scrollIntoView).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    if (previous) Object.defineProperty(HTMLElement.prototype, "scrollIntoView", previous);
    else delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
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
describe("App — U13 device selection identity", () => {
  it("never exposes editors for a detail response whose ID differs from the selection", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxproj");
    apiMock.importProject.mockResolvedValue(treeWithDevice());
    apiMock.deviceDetail.mockReset();
    apiMock.deviceDetail.mockResolvedValue({ ...deviceDetailFixture(), id: 99 });
    const root = await renderApp();
    try {
      await act(async () => findButton("Open project…").click());
      await act(async () => treeLabel("Device D").click());
      expect(host!.querySelector(".workbench-pane-right .inspector input")).toBeNull();
      expect(host!.querySelector(".device-workspace")).toBeNull();
    } finally {
      await act(async () => root.unmount());
    }
  });

  it.each(["pending", "failed"])("removes the previous device's editors while the next detail is %s", async (outcome) => {
    const tree = treeWithDevice();
    tree.installations[0].unassigned.push({ ...deviceNode(), id: 43, name: "Device B" });
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxproj");
    apiMock.importProject.mockResolvedValue(tree);
    let resolve!: (detail: DeviceDetail) => void;
    let reject!: (error: Error) => void;
    apiMock.deviceDetail.mockReset();
    apiMock.deviceDetail.mockResolvedValueOnce(deviceDetailFixture())
      .mockImplementationOnce(() => new Promise<DeviceDetail>((yes, no) => { resolve = yes; reject = no; }));
    const root = await renderApp();
    try {
      await act(async () => findButton("Open project…").click());
      await act(async () => treeLabel("Device D").click());
      expect(host!.querySelector(".workbench-pane-right .inspector input")).not.toBeNull();
      expect(host!.querySelector(".device-workspace")).not.toBeNull();

      await act(async () => treeLabel("Device B").click());
      expect(apiMock.deviceDetail).toHaveBeenLastCalledWith(43, null);
      expect(host!.querySelector(".workbench-pane-right .inspector input")).toBeNull();
      expect(host!.querySelector(".device-workspace")).toBeNull();
      expect(apiMock.setDeviceDescription).not.toHaveBeenCalled();

      await act(async () => {
        if (outcome === "failed") reject(new Error("detail unavailable"));
        else resolve({ ...deviceDetailFixture(), id: 43, name: "Device B" });
      });
      const title = host!.querySelector(".workbench-pane-right .inspector h2");
      if (outcome === "failed") {
        expect(title).toBeNull();
        expect(host!.textContent).toContain("detail unavailable");
      } else {
        expect(title?.textContent).toBe("Device B");
      }
    } finally {
      await act(async () => root.unmount());
    }
  });
});

describe("App — U13 parameter snapshot invalidation", () => {
  it("refreshes parameters after Undo and Redo without changing the selected device", async () => {
    const snapshot = (revision: number, canRedo: boolean) => treeAt({ ...treeWithDevice(), can_undo: true, can_redo: canRedo }, revision);
    const parameters = (value: string) => ({
      programId: "PROG-1", tree: null, stale: [], diagnostics: [],
      sections: [{ scope: null, fields: [{ etsId: "P1", name: "Field A", text: null, kind: "Number",
        value, valueSource: "Stored", editable: true, min: "0", max: "10", enumOptions: [],
        displayOrder: null, access: null, writeEtsId: "P1" }] }],
    });
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxproj");
    apiMock.importProject.mockResolvedValue(snapshot(1, false));
    apiMock.deviceDetail.mockReset();
    apiMock.deviceDetail.mockResolvedValue(deviceDetailFixture());
    apiMock.deviceParameters.mockReset();
    apiMock.deviceParameters.mockResolvedValueOnce(parameters("6"))
      .mockResolvedValueOnce(parameters("5")).mockResolvedValueOnce(parameters("6"));
    apiMock.undo.mockResolvedValue(snapshot(2, true));
    apiMock.redo.mockResolvedValue(snapshot(3, false));
    const root = await renderApp();
    try {
      await act(async () => findButton("Open project…").click());
      await act(async () => treeLabel("Device D").click());
      const value = () => host!.querySelector<HTMLInputElement>(".parameter-field input")?.value;
      expect(value()).toBe("6");
      await act(async () => findButton("Undo").click());
      expect(value()).toBe("5");
      await act(async () => findButton("Redo").click());
      expect(value()).toBe("6");
      expect(apiMock.deviceParameters).toHaveBeenCalledTimes(3);
      expect(apiMock.deviceParameters.mock.calls.every(([id]) => id === 42)).toBe(true);
    } finally {
      await act(async () => root.unmount());
    }
  });
});

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
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de-DE");
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
  // per-request generation counter (`deviceDetailRequestIdRef` in `App.tsx`)
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
    // both requests are now in flight, with de-DE's `deviceDetailRequestIdRef`
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

    // Without `deviceDetailRequestIdRef`, the de-DE reply — delivered last —
    // would silently overwrite the fr-FR detail already on screen, even
    // though `productLanguage` has been "fr-FR" the whole time.
    expect(host!.textContent).toContain("Device D (fr-FR)");
    expect(host!.textContent).not.toContain("Device D (de-DE)");

    root.unmount();
  });

  // Task 3 regression test: the counter above only closed the
  // language-vs-language gap — it was bumped by the language effect alone,
  // so a stale language reply could still land after, and overwrite, a
  // *later* edit-triggered refetch from `handleTreeUpdate` (Undo/Redo/any
  // command), since that site's selection-identity check is satisfied too
  // (the selection never moves for either kind of request). Sharing one
  // `deviceDetailRequestIdRef` across all three `api.deviceDetail` call
  // sites closes that gap as well.
  it("keeps an edit-triggered refetch issued after a language change, even when the older language reply resolves later", async () => {
    function Writer() {
      const [, setLanguage] = useProductLanguage();
      return (
        <button type="button" onClick={() => setLanguage("de-DE")}>
          set de-DE
        </button>
      );
    }

    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    // `can_undo: true` from the start keeps the toolbar's Undo button
    // enabled without needing a real command to flip it first.
    apiMock.importProject.mockResolvedValue({ ...treeWithDevice(), can_undo: true });
    apiMock.undo.mockResolvedValue({ ...treeWithDevice(), can_undo: true });

    // Call order: (1) the initial selection's own fetch, resolved right
    // away; (2) the language-change effect's fetch, held open; (3) Undo's
    // `handleTreeUpdate` refetch, issued after (2) while it is still
    // in flight, also held open.
    let resolveLang: ((detail: DeviceDetail) => void) | undefined;
    let resolveEdit: ((detail: DeviceDetail) => void) | undefined;
    apiMock.deviceDetail
      .mockResolvedValueOnce(deviceDetailFixture())
      .mockImplementationOnce(
        () =>
          new Promise<DeviceDetail>((resolve) => {
            resolveLang = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<DeviceDetail>((resolve) => {
            resolveEdit = resolve;
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

    const langButton = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.textContent === "set de-DE",
    )!;
    await act(async () => {
      langButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.deviceDetail).toHaveBeenCalledTimes(2);
    expect(resolveLang).toBeDefined();

    // The edit-triggered refetch, issued after the still-in-flight
    // language request.
    await act(async () => {
      findButton("Undo").dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.deviceDetail).toHaveBeenCalledTimes(3);
    expect(resolveEdit).toBeDefined();

    // Resolve out of order: the NEWER request (the edit refetch) answers
    // first, the OLDER request (the language change) answers last.
    await act(async () => {
      resolveEdit!({ ...deviceDetailFixture(), name: "Device D (edit)" });
      await Promise.resolve();
    });
    await act(async () => {
      resolveLang!({ ...deviceDetailFixture(), name: "Device D (de-DE)" });
      await Promise.resolve();
    });

    // Without a request id shared across all three call sites, the stale
    // de-DE reply would silently overwrite the newer edit result, since
    // `handleTreeUpdate`'s selection-identity check alone is satisfied
    // here too.
    expect(host!.textContent).toContain("Device D (edit)");
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

it("leaves native text undo to the focused input", async () => {
  host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  await act(async () => root.render(<App />));
  const input = document.createElement("input"); host.append(input); input.focus();
  const event = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true, cancelable: true });
  await act(async () => input.dispatchEvent(event));
  expect(event.defaultPrevented).toBe(false);
  expect(apiMock.undo).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

// Stage 4, items 4 and 5: the File menu and everything inside it — export,
// CSV, documentation, compare — reachable and operable by keyboard alone.
// `<summary>` is keyboard-activated by Enter/Space, which the platform
// delivers as a click on the focused summary, so that is what these tests
// dispatch.
describe("App — the File menu by keyboard alone", () => {
  async function openFileMenu() {
    const summary = host!.querySelector<HTMLElement>(".file-menu summary")!;
    summary.focus();
    expect(document.activeElement).toBe(summary);
    await act(async () => summary.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    return summary;
  }

  it("opens from the keyboard, exposes every file entry in the tab order, and Escape returns focus", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const summary = await openFileMenu();
    const menu = host!.querySelector<HTMLElement>(".file-menu")!;
    expect(menu.hasAttribute("open")).toBe(true);

    // Every entry is a real button with no negative tabindex, so a Tab walk
    // from the summary reaches all of them in the order they are read.
    const entries = [...menu.querySelectorAll<HTMLButtonElement>(".file-menu-content button")];
    expect(entries.map((b) => b.textContent)).toEqual([
      "New project…",
      "Open project…",
      "Open (.knxdb)…",
      "Save As…",
      "Export project…",
      "Export group addresses (CSV)…",
      "Import group addresses (CSV)…",
      "Export documentation…",
      "Compare with…",
      "Debug report…",
      "Analyze support gaps…",
      // ADR-0084: the first-run guide, reopened on request.
      "Show introduction…",
      // ADR-0089: present while achievements are switched on (the default).
      "Achievements…",
      // T28/F5. No "Quit" after it: `isTauri()` is mocked `false` here,
      // and a browser tab cannot close itself.
      "About KNXBench…",
    ]);
    expect(entries.every((b) => b.tabIndex >= 0)).toBe(true);
    // Nothing in the menu is keyboard-dead: every entry here works on a
    // freshly imported project. (The one that used to be disabled without a
    // `.knxdb` path was the `.knxproj` export, withdrawn by ADR-0028.)
    expect(entries.filter((b) => b.disabled).map((b) => b.textContent)).toEqual([]);

    await act(async () => {
      entries[0].dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(menu.hasAttribute("open")).toBe(false);
    expect(document.activeElement).toBe(summary);

    await act(async () => root.unmount());
  });

  // ADR-0089: "off means off" — no menu entry while achievements are switched off.
  it("leaves the achievements entry out of the File menu while achievements are switched off", async () => {
    settingsStorage.setItem("achievementsEnabled", "false");
    const root = await renderApp();
    await openFileMenu();
    const entries = [...host!.querySelectorAll<HTMLButtonElement>(".file-menu-content button")].map((b) => b.textContent);
    expect(entries).toContain("Show introduction…");
    expect(entries).not.toContain("Achievements…");
    await act(async () => root.unmount());
  });

  it("closes the comparison report on Escape before the menu, and restores focus at each step", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    apiMock.diffProject.mockResolvedValue({ infoChanges: [], installations: [] });
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const summary = await openFileMenu();
    const menu = host!.querySelector<HTMLElement>(".file-menu")!;
    const compare = findButton("Compare with…");
    await act(async () => compare.dispatchEvent(new MouseEvent("click", { bubbles: true })));

    const panel = host!.querySelector<HTMLElement>(".project-diff-panel")!;
    expect(panel.textContent).toContain("No differences found.");
    // Focus moves into the report, so the next Escape has somewhere to land.
    expect(document.activeElement).toBe(panel);

    await act(async () => {
      panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    // Innermost first: the report is gone, the menu is still open, and focus
    // sits back on the entry that opened the report.
    expect(host!.querySelector(".project-diff-panel")).toBeNull();
    expect(menu.hasAttribute("open")).toBe(true);
    expect(document.activeElement).toBe(compare);

    await act(async () => {
      compare.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(menu.hasAttribute("open")).toBe(false);
    expect(document.activeElement).toBe(summary);

    await act(async () => root.unmount());
  });
});

describe("App — browser project export", () => {
  it("labels the local export differently from Save As in both languages", () => {
    expect(enMessages["toolbar.exportProject"]).toBe("Export project…");
    expect(deMessages["toolbar.exportProject"]).toBe("Projekt exportieren…");
    expect(enMessages["toolbar.exportProject"]).not.toBe(enMessages["toolbar.saveAs"]);
    expect(deMessages["toolbar.exportProject"]).not.toBe(deMessages["toolbar.saveAs"]);
  });

  it("shows a localized export command, disables it without a project, and uses native navigation", async () => {
    let anchor: HTMLAnchorElement | undefined;
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      anchor = this;
    });
    const root = await renderApp();
    const exportButton = findButton("Export project…");

    expect(exportButton.disabled).toBe(true);

    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(exportButton.disabled).toBe(false);

    await act(async () => {
      exportButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(anchor?.href).toMatch(/\/api\/project\/download$/);
    expect(anchor?.download).toBe("project.knxdb");

    click.mockRestore();
    await act(async () => root.unmount());
  });

  it("omits the browser export command inside the Tauri shell", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    const root = await renderApp();

    expect(host!.textContent).not.toContain("Export project…");

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });
});

// The gap this closes: `POST /api/project/new` has worked since
// 2026-09-08, and until now the UI had no caller for it at all — both
// welcome-screen buttons required the user to already own a file. These
// tests pin the third one, which is the only route to a project for
// someone whose first act is installing a device from the product
// catalogue.
describe("App — starting a project from scratch", () => {
  it("explains the three distinct empty-workspace routes as accessible cards", async () => {
    const root = await renderApp();

    const actions = [...host!.querySelectorAll<HTMLButtonElement>(".welcome-actions button")];
    expect(actions.map((button) => document.getElementById(button.getAttribute("aria-labelledby")!)?.textContent)).toEqual([
      "New project…",
      "Open KNXBench project",
      "Import ETS project",
    ]);
    expect(actions.map((button) => document.getElementById(button.getAttribute("aria-describedby")!)?.textContent)).toEqual([
      "Start an empty project and build its structure here.",
      "Continue a saved KNXBench .knxdb project.",
      "Read an ETS .knxproj archive. Save your work as .knxdb later.",
    ]);
    expect(actions.every((button) => button.getAttribute("type") === "button")).toBe(true);
    expect(actions[0].classList.contains("primary-action")).toBe(true);
    expect(host!.querySelector(".welcome-workspace h1")?.id).toBe("welcome-title");
    // At narrow widths the center stacks before the explorer. DOM order
    // must match that visual order so Tab reaches the cards before the
    // navigation buttons that are below them, not the other way around.
    expect(host!.querySelector(".workbench")?.classList.contains("workbench--welcome")).toBe(true);
    expect(host!.querySelector(".workbench-body")?.firstElementChild?.classList.contains("workbench-center")).toBe(true);

    // The card itself, not the File menu, must keep the native and ETS
    // file routes separate. Cancel the picker so the welcome stays open.
    filePickerMock.pickOpenPath.mockResolvedValue(null);
    await act(async () => actions[1].click());
    await act(async () => actions[2].click());
    expect(filePickerMock.pickOpenPath.mock.calls.map(([filters]) => filters[0].extensions)).toEqual([
      ["knxdb"],
      ["knxproj"],
    ]);

    await act(async () => root.unmount());
  });

  it("translates the welcome route names and their format explanations", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const root = await renderApp();
    const actions = [...host!.querySelectorAll<HTMLButtonElement>(".welcome-actions button")];

    expect(actions.map((button) => document.getElementById(button.getAttribute("aria-labelledby")!)?.textContent)).toEqual([
      "Neues Projekt…",
      "KNXBench-Projekt öffnen",
      "ETS-Projekt importieren",
    ]);
    expect(actions.map((button) => document.getElementById(button.getAttribute("aria-describedby")!)?.textContent)).toEqual([
      "Ein leeres Projekt anlegen und seine Struktur hier aufbauen.",
      "Ein gespeichertes KNXBench-Projekt (.knxdb) weiterbearbeiten.",
      "Ein ETS-Archiv (.knxproj) einlesen. Änderungen später als .knxdb speichern.",
    ]);

    await act(async () => root.unmount());
  });

  it("opens the creation dialog from the welcome screen and swaps in the workbench once it returns a tree", async () => {
    apiMock.newProject.mockResolvedValue(baseTree());
    const root = await renderApp();

    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector('[role="dialog"]')).not.toBeNull();
    // Opening the dialog alone asks the server for nothing.
    expect(apiMock.newProject).not.toHaveBeenCalled();

    await act(async () => {
      host!.querySelector("form")!.dispatchEvent(
        new Event("submit", { bubbles: true, cancelable: true }),
      );
    });

    expect(apiMock.newProject).toHaveBeenCalledTimes(1);
    expect(apiMock.newProject.mock.calls[0][0].discardChanges).toBe(false);
    // The welcome screen is gone behind the wizard's "created" page
    // (ADR-0093), which closes on Done.
    expect(host!.querySelector(".welcome-workspace")).toBeNull();
    expect(host!.querySelector(".workbench")?.classList.contains("workbench--welcome")).toBe(false);
    expect(host!.querySelector('[role="dialog"]')?.textContent).toContain("Project created");
    await act(async () => {
      findButton("Done").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector('[role="dialog"]')).toBeNull();

    await act(async () => root.unmount());
  });

  it("opens the add-device wizard from the project wizard's Add devices now", async () => {
    apiMock.newProject.mockResolvedValue(baseTree());
    const root = await renderApp();
    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      host!.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });
    await act(async () => {
      findButton("Add devices now").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    const dialogs = Array.from(host!.querySelectorAll('[role="dialog"]'));
    expect(dialogs).toHaveLength(1);
    expect(dialogs[0].querySelector("#device-wizard-title")?.textContent).toBe("Add device");

    await act(async () => root.unmount());
  });

  it("leaves a fresh project with no file behind it, so Save has to ask where", async () => {
    apiMock.importProject.mockResolvedValue(baseTree());
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    filePickerMock.pickSavePath.mockResolvedValue(null);
    apiMock.newProject.mockResolvedValue(baseTree());
    const root = await renderApp();

    // Open something first, so a `store_path` could plausibly be lingering
    // — an ETS import does not set one, which is why the new project has
    // to clear the flag rather than merely not set it.
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      host!.querySelector("form")!.dispatchEvent(
        new Event("submit", { bubbles: true, cancelable: true }),
      );
    });

    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    // Save As, not a silent overwrite of whatever was open before. The
    // mock has no `saveProject` at all, so the plain-save path would have
    // thrown before ever reaching the picker.
    expect(filePickerMock.pickSavePath).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
  });

  it("reaches the same dialog from the command palette with no project open", async () => {
    const root = await renderApp();

    await act(async () => {
      findButton("Commands… (Ctrl+Shift+P)").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(host!.querySelector('[role="dialog"]')).not.toBeNull();
    expect(host!.querySelector("input[aria-label=\"Project name\"]")).not.toBeNull();

    await act(async () => root.unmount());
  });
});

// T37 / ADR-0023. The banner itself is covered in
// `LoadProgressBanner.test.tsx`; what belongs here is the wiring: one load
// at a time, feedback for its whole duration, and a failure that leaves
// both the banner and whatever was already open in place.
describe("App — project load progress", () => {
  it("shows the banner for the whole load and refuses a second one while it runs", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    let finish: (tree: ProjectTree) => void = () => {};
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((resolve) => { finish = resolve; }));
    apiMock.loadProgress.mockResolvedValue({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "parseTopology",
      completed: null, total: null, status: "running", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(host!.querySelector(".load-progress")).not.toBeNull();
    expect(host!.textContent).toContain("villa.knxproj");
    expect(host!.textContent).toContain("Parsing the topology");
    expect(findButton("Open project…").disabled).toBe(true);
    expect(findButton("Open (.knxdb)…").disabled).toBe(true);

    // A second click while the first load is still running must not reach
    // the server at all — the `409` is the backstop, not the mechanism.
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.importProject).toHaveBeenCalledTimes(1);

    await act(async () => {
      finish(baseTree());
    });
    expect(host!.querySelector(".load-progress")).toBeNull();
    const toasts = host!.querySelectorAll<HTMLElement>(".toast--fun[role=\"status\"]");
    expect(toasts).toHaveLength(1);
    expect(toasts[0].querySelector(".toast-body")?.textContent).toBe("Loaded villa.knxproj.");

    await act(async () => root.unmount());
  });

  // F14: the native `.knxdb` button binds to `openNativeProject`, which
  // must call `api.openProject` — a mutation swapping it for
  // `api.importProject` (the ETS-import path `pickProject` uses) passed
  // every other gate, because nothing here ever clicked this button.
  it("opens a .knxdb file through api.openProject and announces it in the active locale", async () => {
    setSetting("uiLanguage", "de");
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxdb");
    apiMock.openProject.mockResolvedValue(baseTree());
    const root = await renderApp();

    await act(async () => {
      findButton("Öffnen (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(apiMock.openProject).toHaveBeenCalledTimes(1);
    expect(apiMock.importProject).not.toHaveBeenCalled();
    const toasts = host!.querySelectorAll<HTMLElement>(".toast--fun[role=\"status\"]");
    expect(toasts).toHaveLength(1);
    expect(toasts[0].querySelector(".toast-body")?.textContent).toBe("villa.knxdb wurde geladen.");

    await act(async () => root.unmount());
  });

  it("keeps the banner after a failure, naming the phase the load died in", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    apiMock.importProject.mockRejectedValue(new Error("invalid Zip archive"));
    apiMock.loadProgress.mockResolvedValue({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "openContainer",
      completed: null, total: null, status: "failed", error: "invalid Zip archive",
      clientToken: OWN_CLIENT_TOKEN,
    });
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const banner = host!.querySelector(".load-progress")!;
    expect(banner.getAttribute("data-failed")).toBe("true");
    expect(banner.textContent).toContain("Could not load villa.knxproj");
    expect(banner.textContent).toContain("Opening the archive");
    // And the load is over: the buttons are usable again.
    expect(findButton("Open project…").disabled).toBe(false);

    await act(async () => root.unmount());
  });
});

// T37 fix round 1, finding F1. Every one of these four failures used to
// leave a banner claiming a load was still running — three of them showing
// a *different* operation's phase under our file name. The rule they all
// pin: when a load is over, the banner says so, whatever the server's
// snapshot happens to be. A moving bar for nothing is the exact defect
// this feature exists to avoid.
describe("App — a failed load never renders a running banner", () => {
  // The banner is a failure, not a fiction: no progressbar at all, so no
  // indeterminate shuttle, and no phase borrowed from another operation.
  function expectFailedBanner(foreignPhrases: string[]) {
    const banner = host!.querySelector(".load-progress")!;
    expect(banner, "the banner must stay up to report the failure").not.toBeNull();
    expect(banner.getAttribute("data-failed")).toBe("true");
    expect(banner.querySelector('[role="progressbar"]')).toBeNull();
    expect(banner.querySelector('[data-indeterminate="true"]')).toBeNull();
    expect(host!.querySelector('.toast--fun[role="status"]')).toBeNull();
    expect(banner.textContent).toContain("Could not load villa.knxproj");
    for (const phrase of foreignPhrases) {
      expect(banner.textContent, `banner must not borrow "${phrase}"`).not.toContain(phrase);
    }
  }

  async function clickOpen() {
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
  }

  it("probe A: a pre-flight rejection does not inherit the previous operation's phase", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    // The server refused before it began an operation, so the newest
    // snapshot is still the *previous*, finished load — a native open of
    // a different file, sitting on its final phase.
    apiMock.loadProgress.mockResolvedValue({
      operationId: 3, kind: "open", source: "older.knxdb", phase: "buildProjectTree",
      completed: null, total: null, status: "succeeded", error: null,
      clientToken: "99999999-9999-9999-9999-999999999999",
    });
    apiMock.importProject.mockRejectedValue(new Error("path is outside the data directory"));
    const root = await renderApp();

    await clickOpen();

    expectFailedBanner(["Building the project tree", "older.knxdb"]);
    expect(host!.querySelector(".load-progress")!.textContent).toContain("path is outside the data directory");
    expect(apiMock.currentProject).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });

  it("probe B: an unreachable server leaves a failure, not a permanent 'Starting…'", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    // Nothing answers: not the poll, not the snapshot fetched after the
    // POST died.
    apiMock.loadProgress.mockRejectedValue(new Error("Failed to fetch"));
    apiMock.importProject.mockRejectedValue(new Error("Failed to fetch"));
    const root = await renderApp();

    await clickOpen();

    expectFailedBanner([]);
    expect(host!.querySelector(".load-progress")!.textContent).toContain("Failed to fetch");
    await act(async () => root.unmount());
  });

  it("probe C: a 409 shows our failure, never the holder's operation", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    // Another client holds the slot: its operation is genuinely running,
    // and it is genuinely not ours — it carries a token this load never
    // generated.
    apiMock.loadProgress.mockResolvedValue({
      operationId: 7, kind: "open", source: "someone-elses.knxdb", phase: "loadStoredProject",
      completed: null, total: null, status: "running", error: null,
      clientToken: "77777777-7777-7777-7777-777777777777",
    });
    apiMock.importProject.mockRejectedValue(new Error("a project load is already running (operation 7)"));
    const root = await renderApp();

    await clickOpen();

    expectFailedBanner(["Reading the stored project", "someone-elses.knxdb"]);
    expect(host!.querySelector(".load-progress")!.textContent).toContain("already running");
    await act(async () => root.unmount());
  });

  it("recovers an owned successful load whose response was lost (§96)", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    apiMock.loadProgress.mockResolvedValueOnce({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "parseTopology",
      completed: null, total: null, status: "running", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    // The operation finished on the server; only its answer was lost.
    apiMock.loadProgress.mockResolvedValue({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "buildProjectTree",
      completed: null, total: null, status: "succeeded", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    apiMock.currentProject.mockResolvedValue({ ...treeWithDevice(), has_store_path: false });
    let fail: (error: Error) => void = () => {};
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
    const root = await renderApp();

    await clickOpen();
    // The poll has adopted operation 1 by now and the banner is running.
    expect(host!.querySelector(".load-progress")!.getAttribute("data-failed")).toBeNull();
    await act(async () => {
      fail(new Error("connection closed"));
    });

    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
    expect(host!.textContent).toContain("Device D");
    expect(host!.querySelector(".load-progress")).toBeNull();
    expect(host!.querySelector('[role="alert"]')).toBeNull();
    const toasts = host!.querySelectorAll<HTMLElement>(".toast--fun[role=\"status\"]");
    expect(toasts).toHaveLength(1);
    expect(toasts[0].querySelector(".toast-body")?.textContent).toBe("Recovered the current project.");
    await act(async () => root.unmount());
  });

  it("recovers an owned successful native open with its stored path intact (§96)", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxdb");
    apiMock.loadProgress.mockResolvedValue({
      operationId: 2, kind: "open", source: "villa.knxdb", phase: "buildProjectTree",
      completed: null, total: null, status: "succeeded", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    apiMock.currentProject.mockResolvedValue({ ...treeWithDevice(), has_store_path: true });
    apiMock.openProject.mockRejectedValueOnce(new Error("connection closed"));
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    // Once to recover the lost open response, then once more to consume the
    // authoritative clean tree established by Save.
    expect(apiMock.currentProject).toHaveBeenCalledTimes(2);
    expect(apiMock.saveProject).toHaveBeenCalledTimes(1);
    expect(filePickerMock.pickSavePath).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });

  it.each([true, false])("recovery uses current save metadata when another project replaces the owned load (saved=%s)", async (saved) => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/old.knxproj");
    apiMock.loadProgress.mockResolvedValue({
      operationId: 3, kind: saved ? "import" : "open", source: "old.knxproj",
      phase: "buildProjectTree", completed: null, total: null,
      status: "succeeded", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    let recover!: (value: unknown) => void;
    apiMock.currentProject.mockReturnValue(new Promise((resolve) => { recover = resolve; }));
    apiMock.importProject.mockRejectedValue(new Error("connection closed"));
    filePickerMock.pickSavePath.mockResolvedValue(null);
    const root = await renderApp();
    await clickOpen();
    // The GET completes after another client has replaced the project. Its
    // save state is deliberately opposite to the completed operation's kind.
    await act(async () => { recover({ ...treeWithDevice(), has_store_path: saved }); });
    expect(host!.querySelector(".load-progress")).toBeNull();
    const status = host!.querySelector('.toast--fun[role="status"]');
    expect(status?.textContent).toContain("Recovered the current project.");
    expect(status?.textContent).not.toContain("old.knxproj");
    await act(async () => { findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true })); });
    expect(apiMock.saveProject).toHaveBeenCalledTimes(saved ? 1 : 0);
    expect(filePickerMock.pickSavePath).toHaveBeenCalledTimes(saved ? 0 : 1);
    await act(async () => root.unmount());
  });

  it("does not recover a succeeded snapshot with an empty ownership token", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    apiMock.loadProgress.mockResolvedValue({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "buildProjectTree",
      completed: null, total: null, status: "succeeded", error: null, clientToken: null,
    });
    apiMock.importProject.mockRejectedValue(new Error("connection closed"));
    const root = await renderApp();

    await clickOpen();

    expectFailedBanner([]);
    expect(apiMock.currentProject).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("connection closed");
    await act(async () => root.unmount());
  });

  it("keeps the recovery GET failure when an owned successful load cannot be retrieved", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    apiMock.loadProgress.mockResolvedValue({
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "buildProjectTree",
      completed: null, total: null, status: "succeeded", error: null, clientToken: OWN_CLIENT_TOKEN,
    });
    apiMock.currentProject.mockRejectedValue(new Error("current project unavailable"));
    apiMock.importProject.mockRejectedValue(new Error("connection closed"));
    const root = await renderApp();

    await clickOpen();

    expectFailedBanner([]);
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("current project unavailable");
    expect(host!.querySelector(".load-progress")?.textContent).toContain("current project unavailable");
    await act(async () => root.unmount());
  });

  // Fix round 2, F8 (re-review "probe D"; carried forward under the fix
  // round 3 token model). A foreign operation starts before our own POST
  // is refused. Rounds 1 and 2 needed a baseline read and a `source`
  // match to rule this out and still missed the same-basename case (F9);
  // under the token, `ownsOperation` needs nothing but the id this load
  // never sent to the stranger's operation — no pre-flight read at all.
  it("a foreign operation starting before our POST is refused is not ours (F8)", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    let fail: (error: Error) => void = () => {};
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
    apiMock.loadProgress.mockResolvedValue({
      operationId: 2, kind: "open", source: "someone-elses.knxdb", phase: "loadStoredProject",
      completed: null, total: null, status: "running", error: null,
      clientToken: "22222222-2222-2222-2222-222222222222",
    });
    const root = await renderApp();

    await clickOpen();

    // Give the immediate poll a chance to run and try to adopt the
    // foreign operation before our own POST is rejected.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      fail(new Error("a project load is already running (operation 2)"));
    });

    expectFailedBanner(["Reading the stored project", "someone-elses.knxdb"]);
    await act(async () => root.unmount());
  });

  // F9: three straight rounds of a server-side heuristic (no filter, an
  // id-only filter, an id-and-source filter) all had the same blind spot
  // — two clients loading files with the same base name from different
  // directories. This is that exact scenario: the stranger's operation
  // fails first, under the same file name we are loading, and the banner
  // must still show *our* error, never theirs.
  it("F9: a same-basename stranger's failure is never rendered as ours", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    let fail: (error: Error) => void = () => {};
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
    apiMock.loadProgress.mockResolvedValue({
      operationId: 9,
      kind: "import",
      source: "villa.knxproj",
      phase: "openContainer",
      completed: null,
      total: null,
      status: "failed",
      error: "disk on the other machine is full",
      clientToken: "33333333-3333-3333-3333-333333333333",
    });
    const root = await renderApp();

    await clickOpen();

    // The stranger's operation is already sitting there, failed, under
    // the same source name, by the time our own POST is refused.
    await act(async () => {
      fail(new Error("a project load is already running (operation 9)"));
    });

    expectFailedBanner(["disk on the other machine is full"]);
    expect(host!.querySelector(".load-progress")!.textContent).toContain("already running");
    await act(async () => root.unmount());
  });

  // Fix round 6, F-C. The poll interval is still armed while `runLoad`'s
  // catch awaits its final snapshot, and the `cancelled` latch is closed
  // later still, by React's effect cleanup. A poll whose fetch lands in
  // between passed every filter — not cancelled, `running`, ours — and
  // painted a running phase straight over the failure that had just been
  // written. Polling then stopped for good, so the banner froze on a
  // phase with a moving shuttle for the rest of the session: the exact
  // lie this feature exists to prevent, reachable whenever the POST dies
  // at transport level while the server's operation carries on.
  it("F-C: a poll landing after the failure is written never repaints it as running", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    let fail: (error: Error) => void = () => {};
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
    // The first poll's fetch is held open on purpose: it answers only
    // once the failure is on screen, which is the window the defect
    // lived in.
    let answerFirstPoll: (snapshot: unknown) => void = () => {};
    apiMock.loadProgress.mockReturnValueOnce(new Promise((resolve) => { answerFirstPoll = resolve; }));
    // The snapshot the catch fetches after the POST dies: the connection
    // is gone, so this one does not answer either.
    apiMock.loadProgress.mockRejectedValue(new Error("connection closed"));
    const root = await renderApp();

    await clickOpen();

    await act(async () => {
      fail(new Error("connection closed"));
      // Let the catch run to its end — final snapshot fetch, failure
      // write, `setLoading(false)` — without letting React commit any of
      // it. The effect cleanup has not run, so `cancelled` is still
      // false, which is precisely the state the stale poll needs.
      for (let i = 0; i < 6; i += 1) await Promise.resolve();
      answerFirstPoll({
        operationId: 1, kind: "import", source: "villa.knxproj", phase: "parseTopology",
        completed: null, total: null, status: "running", error: null, clientToken: OWN_CLIENT_TOKEN,
      });
      for (let i = 0; i < 6; i += 1) await Promise.resolve();
    });

    expectFailedBanner(["Parsing the topology"]);
    expect(host!.querySelector(".load-progress")!.textContent).toContain("connection closed");
    await act(async () => root.unmount());
  });
});

// Fix round 6, F-A and F-B: two rules the banner has always followed and
// nothing ever checked. A reviewer neutralised `clearInterval` and deleted
// `runLoad`'s `setLoadSnapshot(null)`, one at a time, and the whole suite
// stayed green through both.
describe("App — a load's banner and its polling both end with the load", () => {
  // `LOAD_POLL_INTERVAL_MS` from App.tsx, which does not export it. A
  // test that advanced by less than the real interval would see no polls
  // at all and pass for the wrong reason, so this has to match.
  const POLL_INTERVAL_MS = 250;

  function runningSnapshot() {
    return {
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "parseTopology",
      completed: null, total: null, status: "running", error: null, clientToken: OWN_CLIENT_TOKEN,
    };
  }

  function failedSnapshot() {
    return {
      operationId: 1, kind: "import", source: "villa.knxproj", phase: "openContainer",
      completed: null, total: null, status: "failed", error: "invalid Zip archive",
      clientToken: OWN_CLIENT_TOKEN,
    };
  }

  async function clickOpen() {
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
  }

  async function letTheIntervalTick(times: number) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * times);
    });
  }

  // The count only means something if it was rising in the first place:
  // every test below waits for the interval to fire at least once while
  // the load is genuinely in flight before asking whether it stopped.
  async function pollsWhileRunning(): Promise<number> {
    await letTheIntervalTick(3);
    const calls = apiMock.loadProgress.mock.calls.length;
    expect(calls, "the interval must be polling while the load runs").toBeGreaterThan(1);
    return calls;
  }

  it("F-A: stops polling once the load succeeds", async () => {
    vi.useFakeTimers();
    try {
      filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
      let finish: (tree: ProjectTree) => void = () => {};
      apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((resolve) => { finish = resolve; }));
      apiMock.loadProgress.mockResolvedValue(runningSnapshot());
      const root = await renderApp();

      await clickOpen();
      await pollsWhileRunning();

      await act(async () => { finish(baseTree()); });
      const atTheEnd = apiMock.loadProgress.mock.calls.length;
      await letTheIntervalTick(10);

      expect(apiMock.loadProgress.mock.calls.length).toBe(atTheEnd);
      await act(async () => root.unmount());
    } finally {
      vi.useRealTimers();
    }
  });

  it("F-A: stops polling once the load fails", async () => {
    vi.useFakeTimers();
    try {
      filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
      let fail: (error: Error) => void = () => {};
      apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
      apiMock.loadProgress.mockResolvedValue(runningSnapshot());
      const root = await renderApp();

      await clickOpen();
      await pollsWhileRunning();

      // The catch fetches one last snapshot of its own, so the count is
      // read after the failure has fully settled rather than before.
      apiMock.loadProgress.mockResolvedValue(failedSnapshot());
      await act(async () => { fail(new Error("invalid Zip archive")); });
      const atTheEnd = apiMock.loadProgress.mock.calls.length;
      await letTheIntervalTick(10);

      expect(host!.querySelector(".load-progress")!.getAttribute("data-failed")).toBe("true");
      expect(apiMock.loadProgress.mock.calls.length).toBe(atTheEnd);
      await act(async () => root.unmount());
    } finally {
      vi.useRealTimers();
    }
  });

  it("F-A: stops polling when the app unmounts mid-load", async () => {
    vi.useFakeTimers();
    try {
      filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
      apiMock.importProject.mockReturnValue(new Promise<ProjectTree>(() => {}));
      apiMock.loadProgress.mockResolvedValue(runningSnapshot());
      const root = await renderApp();

      await clickOpen();
      await pollsWhileRunning();

      await act(async () => root.unmount());
      const atTheEnd = apiMock.loadProgress.mock.calls.length;
      await letTheIntervalTick(10);

      expect(apiMock.loadProgress.mock.calls.length).toBe(atTheEnd);
    } finally {
      vi.useRealTimers();
    }
  });

  // F-B: `runLoad` clears the snapshot before its POST. Without that one
  // line the second load opens reading "Could not load B — Failed during:
  // <A's phase>", with A's error underneath it, and keeps that text until
  // B's first poll answers — which for a B that fails early is never.
  it("F-B: a second load carries none of the first load's phase or error", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/home/knxbench/projects/villa.knxproj");
    apiMock.importProject.mockRejectedValueOnce(new Error("invalid Zip archive"));
    apiMock.loadProgress.mockResolvedValue(failedSnapshot());
    const root = await renderApp();

    await clickOpen();
    const failure = host!.querySelector(".load-progress")!;
    expect(failure.getAttribute("data-failed")).toBe("true");
    expect(failure.textContent).toContain("Opening the archive");

    // The user picks a second file. Neither its POST nor its first poll
    // ever answers, so everything the banner says about it now, it says
    // with no snapshot of its own.
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/cottage.knxproj");
    apiMock.importProject.mockReturnValue(new Promise<ProjectTree>(() => {}));
    apiMock.loadProgress.mockReturnValue(new Promise(() => {}));

    await clickOpen();

    const banner = host!.querySelector(".load-progress")!;
    expect(banner.textContent).toContain("cottage.knxproj");
    expect(banner.getAttribute("data-failed")).toBeNull();
    expect(banner.textContent, "the second load inherits no phase").not.toContain("Opening the archive");
    expect(banner.textContent, "nor an error").not.toContain("invalid Zip archive");
    expect(banner.textContent, "nor the first file's name").not.toContain("villa.knxproj");

    await act(async () => root.unmount());
  });

  // F-D: `loadSource` is written in exactly two places, and neither of
  // them is the from-scratch path — so a failed import's banner used to
  // sit under the toolbar above a brand-new, entirely unrelated project
  // for the rest of the session.
  it("F-D: starting a project from scratch takes the failed load's banner with it", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
    apiMock.importProject.mockRejectedValue(new Error("invalid Zip archive"));
    apiMock.loadProgress.mockResolvedValue(failedSnapshot());
    apiMock.newProject.mockResolvedValue(baseTree());
    const root = await renderApp();

    await clickOpen();
    expect(host!.querySelector(".load-progress")).not.toBeNull();

    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      host!.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });

    expect(host!.querySelector(".load-progress")).toBeNull();
    await act(async () => root.unmount());
  });

  // F-T26-1: `loadSource` never goes null between a failed load and the
  // next one, so without `key={loadKey}` React keeps the same
  // `LoadProgressBanner` instance alive across both — and the flavour
  // line's shuffle, which runs once in a `useState` initialiser, is
  // therefore the *first* load's shuffle, still sitting at the index the
  // first load left it on. The second load then opens on a line
  // byte-identical to the first's. Removing the `key` kept `tsc` at 0 and
  // all 589 tests green, which is the defect class this session keeps
  // finding: two behaviours, one code path, nothing telling them apart.
  //
  // `Math.random` is pinned per load rather than per call, so the
  // assertion does not depend on how many other things (a startup toast,
  // an error wrapper) happen to draw from it in between. A constant 0
  // makes Fisher-Yates a left rotation by one, opening on entry 02; a
  // constant just under 1 makes it the identity, opening on entry 01.
  it("F-T26-1: a second load reshuffles instead of inheriting the first load's flavour line", async () => {
    const dice = vi.spyOn(Math, "random");
    try {
      dice.mockReturnValue(0);
      filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/villa.knxproj");
      let fail: (error: Error) => void = () => {};
      apiMock.importProject.mockReturnValue(new Promise<ProjectTree>((_, reject) => { fail = reject; }));
      apiMock.loadProgress.mockResolvedValue(runningSnapshot());
      const root = await renderApp();

      await clickOpen();
      const first = host!.querySelector(".load-progress-flavour")!.textContent;
      expect(first, "a constant zero opens on entry 02").toBe(enMessages["loadProgress.flavour.02"]);

      // The first load dies. Its banner stays on screen — deliberately,
      // that is what tells the user what happened — so nothing unmounts.
      apiMock.loadProgress.mockResolvedValue(failedSnapshot());
      await act(async () => { fail(new Error("invalid Zip archive")); });
      expect(host!.querySelector(".load-progress")!.getAttribute("data-failed")).toBe("true");
      expect(host!.querySelector(".load-progress-flavour"), "and stops joking").toBeNull();

      // A second file, and different dice. Neither its POST nor its poll
      // ever answers, so the only thing that can have changed the flavour
      // line is the remount.
      dice.mockReturnValue(0.9999999);
      filePickerMock.pickOpenPath.mockResolvedValue("/home/knxbench/projects/cottage.knxproj");
      apiMock.importProject.mockReturnValue(new Promise<ProjectTree>(() => {}));
      apiMock.loadProgress.mockReturnValue(new Promise(() => {}));

      await clickOpen();

      const second = host!.querySelector(".load-progress-flavour")!.textContent;
      expect(second, "the second load draws its own order").toBe(enMessages["loadProgress.flavour.01"]);
      expect(second, "two loads in a row do not open on the same line").not.toBe(first);

      await act(async () => root.unmount());
    } finally {
      dice.mockRestore();
    }
  });
});

// T23 / ADR-0024. Help is the one feature whose whole point is being
// findable by someone who does not know the application, so all three
// ways in are pinned: the key, the toolbar button, and the palette row.
describe("App — in-application help (T23)", () => {
  const helpPanel = () => host!.querySelector(".help-panel");

  async function pressKey(init: KeyboardEventInit) {
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ...init }));
    });
  }

  it("opens the help panel on F1 with no project open", async () => {
    const root = await renderApp();
    expect(helpPanel()).toBeNull();

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelector("#help-panel-title")?.textContent).toBe(enMessages["help.title"]);
    expect(host!.querySelector("#help-panel-topic-title")?.textContent).toBe(enMessages["help.topic.workbench.title"]);

    await act(async () => root.unmount());
  });

  it("routes F1 from a focused flag tip to communication-object flags", async () => {
    const root = await renderApp();
    const tip = document.createElement("button");
    tip.dataset.helpTopic = "comObjectFlags";
    host!.appendChild(tip);
    tip.focus();
    await pressKey({ key: "F1" });
    expect(host!.querySelector("#help-panel-topic-title")?.textContent).toBe(enMessages["help.topic.comObjectFlags.title"]);
    await act(async () => root.unmount());
  });

  it("opens the group-range topic requested by a range tip", async () => {
    const root = await renderApp();
    await act(async () => window.dispatchEvent(new CustomEvent("knxbench:open-help", { detail: { topicId: "groupRanges" } })));
    expect(host!.querySelector("#help-panel-topic-title")?.textContent).toBe(enMessages["help.topic.groupRanges.title"]);
    await act(async () => root.unmount());
  });

  // The other half of `opensHelp`: a modified F1 belongs to the browser
  // and to the desktop, and taking it would be this application helping
  // itself to a key it was never given.
  it.each(["ctrlKey", "metaKey", "altKey", "shiftKey"] as const)(
    "leaves F1 held with %s alone",
    async (modifier) => {
      const root = await renderApp();

      await pressKey({ key: "F1", [modifier]: true });

      expect(helpPanel()).toBeNull();

      await act(async () => root.unmount());
    },
  );

  it("opens the help panel from the toolbar button", async () => {
    const root = await renderApp();
    const button = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.getAttribute("aria-label") === enMessages["toolbar.help"],
    );
    expect(button).toBeDefined();

    await act(async () => {
      button!.click();
    });

    expect(helpPanel()).not.toBeNull();

    await act(async () => root.unmount());
  });

  // The palette row exists for people who never learn F1. Without this the
  // registry entry could be wired to nothing and every other gate would
  // still be green.
  it("opens the help panel from the command palette", async () => {
    const root = await renderApp();

    await pressKey({ key: "P", ctrlKey: true, shiftKey: true });
    const row = Array.from(host!.querySelectorAll<HTMLElement>('li[role="option"]')).find(
      (li) => li.querySelector("span")?.textContent === enMessages["toolbar.help"],
    );
    expect(row).toBeDefined();

    await act(async () => {
      row!.click();
    });

    expect(helpPanel()).not.toBeNull();

    await act(async () => root.unmount());
  });

  it("closes the command palette when F1 opens help over it", async () => {
    const root = await renderApp();

    await pressKey({ key: "P", ctrlKey: true, shiftKey: true });
    expect(host!.querySelector(".command-palette-panel, .search-panel")).not.toBeNull();

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await act(async () => root.unmount());
  });

  // The invariant above, for every overlay this component owns rather than
  // for the one that happened to be tested first. Two `aria-modal` dialogs
  // at once is undefined for assistive technology, and since every
  // `.search-overlay` is `position: fixed; z-index: 10`, the loser is not
  // merely stacked — it is buried under an opaque full-viewport backdrop
  // with the keyboard focus inside it.
  it("replaces the settings dialog rather than stacking help on top of it", async () => {
    const root = await renderApp();

    const gear = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.getAttribute("aria-label") === enMessages["toolbar.settings"],
    );
    const glyph = gear?.querySelector("svg");
    expect(glyph?.getAttribute("aria-hidden")).toBe("true");
    expect(glyph?.querySelector("path")).not.toBeNull();
    expect(glyph?.querySelectorAll("line")).toHaveLength(0); // not the old sun's rays
    await act(async () => {
      gear!.click();
    });
    expect(host!.querySelector(".settings-panel")).not.toBeNull();

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelector(".settings-panel")).toBeNull();
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await act(async () => root.unmount());
  });

  // Pins the other setter in the same branch: the search overlay is the
  // one F1 was originally written to close, and deleting that line alone
  // left the whole suite green.
  it("replaces the search overlay as well", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(baseTree());
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});

    await pressKey({ key: "k", ctrlKey: true });
    expect(host!.querySelector(".search-panel")).not.toBeNull();
    expect(helpPanel()).toBeNull();

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await act(async () => root.unmount());
  });

  it("replaces the new-project dialog too", async () => {
    const root = await renderApp();

    await act(async () => {
      findButton("New project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await act(async () => root.unmount());
  });

  // F1 belongs to the browser as well: Firefox and Chrome open their own
  // help on it. Without `preventDefault` the user gets two help systems,
  // one of which is about the wrong product.
  it("takes F1 away from the browser", async () => {
    const root = await renderApp();

    const event = new KeyboardEvent("keydown", { key: "F1", bubbles: true, cancelable: true });
    await act(async () => {
      window.dispatchEvent(event);
    });

    expect(event.defaultPrevented).toBe(true);
    expect(helpPanel()).not.toBeNull();

    await act(async () => root.unmount());
  });

  // A modified F1 is not ours, so it must reach the browser untouched.
  it("leaves a modified F1 to whoever wants it", async () => {
    const root = await renderApp();

    const event = new KeyboardEvent("keydown", {
      key: "F1",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    });
    await act(async () => {
      window.dispatchEvent(event);
    });

    expect(event.defaultPrevented).toBe(false);

    await act(async () => root.unmount());
  });

  // The third dialog the branch closes. Unlike Settings and New project
  // this one is a real fetcher, which is why the api mock above grew two
  // catalogue entries.
  it("replaces the catalog browser too", async () => {
    const root = await renderApp();

    await act(async () => {
      findButton(enMessages["workbench.catalog"]).click();
    });
    await act(async () => {});
    expect(host!.querySelector(".catalog-install")).not.toBeNull();

    await pressKey({ key: "F1" });

    expect(helpPanel()).not.toBeNull();
    expect(host!.querySelector(".catalog-install")).toBeNull();
    expect(host!.querySelectorAll('[role="dialog"]')).toHaveLength(1);

    await act(async () => root.unmount());
  });

  // The one overlay `App` must *not* replace, because it is the one it
  // cannot close: `filePicker.ts` mounts `FsPicker` on a React root of
  // its own, so clearing App's state would leave a live modal buried
  // under help with no way back. The guard reads the document, not React
  // state, so the contract can be stated the same way — a `.fs-picker`
  // node exists, therefore F1 does nothing.
  //
  // This suite mocks `./filePicker` wholesale (see the top of the file),
  // so the real picker never mounts here and the node has to be put up
  // by hand. That makes this half of the pin blind to a rename of the
  // class, which is what the source assertion below is for.
  it("keeps out of the way while the file picker is up", async () => {
    const root = await renderApp();
    const picker = document.createElement("div");
    picker.className = "fs-picker";
    document.body.appendChild(picker);

    try {
      await pressKey({ key: "F1" });

      expect(helpPanel()).toBeNull();
    } finally {
      picker.remove();
    }

    // …and once it is gone, F1 works again — otherwise a guard that
    // simply always returned would pass the assertion above.
    await pressKey({ key: "F1" });
    expect(helpPanel()).not.toBeNull();

    await act(async () => root.unmount());
  });

  // Returns everything from `start` up to the `>` that ends the JSX
  // opening tag, ignoring any `>` inside a `{...}` prop expression.
  function openingTag(source: string, start: string): string {
    const from = source.indexOf(start);
    expect(from, `${start} not found`).toBeGreaterThanOrEqual(0);
    let depth = 0;
    for (let i = from; i < source.length; i += 1) {
      const c = source[i];
      if (c === "{") depth += 1;
      else if (c === "}") depth -= 1;
      else if (c === ">" && depth === 0) return source.slice(from, i);
    }
    throw new Error(`unterminated ${start}`);
  }

  // The other half: the selector in `App.tsx` and the class in
  // `FsPicker.tsx` are one contract written in two files, and nothing in
  // a mocked suite connects them. Renaming the class would leave the DOM
  // test above green and the guard dead.
  it("still names the class the file picker actually renders", () => {
    const source = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "FsPicker.tsx"),
      "utf8",
    );
    // Matched anywhere inside the opening tag rather than as two tokens
    // that must touch: `FsPicker.tsx` is the one `Overlay` call site in
    // the codebase that writes `className` before the label, so a pass
    // that makes it match its siblings would move the prop, change
    // nothing at runtime, and has no business failing this test. A
    // rename still fails it. The scan ends at the `>` that closes the
    // tag, counting the braces of the prop expressions so the arrow in
    // `onClose={() => …}` is not mistaken for it.
    const tag = openingTag(source, "<Overlay");
    expect(tag).toContain('className="fs-picker"');

    const app = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "App.tsx"),
      "utf8",
    );
    expect(app).toContain('document.querySelector(".fs-picker")');
  });
});

// T28. Five findings from a hand-run of the application, four of them in
// the workbench chrome and one in the File menu's manners. Each test here
// covers one of them; the picker's size lives in `FsPicker.test.tsx` and
// About's wording in `AboutDialog.test.tsx`, next to the code they pin.
describe("App — the File menu's manners, the stacked splitters, Quit and About (T28)", () => {
  async function openMenu() {
    const summary = host!.querySelector<HTMLElement>(".file-menu summary")!;
    await act(async () => summary.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    return host!.querySelector<HTMLDetailsElement>(".file-menu")!;
  }

  async function openProject(tree: ProjectTree = baseTree()) {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
    apiMock.importProject.mockResolvedValue(tree);
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    return root;
  }

  // F1, first half. The menu is a native `<details>`: clicking an entry
  // runs the entry and leaves the menu hanging open over the workspace.
  it("closes on an item activation, including one whose button belongs to a child component", async () => {
    // `DocumentationExportButton` owns this button; `App` never sees its
    // `onClick`, which is why the close lives on the container and not on
    // each entry. Its dialog is portalled out of the menu, so closing the
    // menu does not hide it.
    const root = await openProject();

    const menu = await openMenu();
    expect(menu.open).toBe(true);
    await act(async () => {
      findButton("Export documentation…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(menu.open).toBe(false);
    expect(document.body.querySelector(".documentation-dialog")).not.toBeNull();

    await act(async () => root.unmount());
  });

  // The dialog's own focus restoration aims at the button that opened it,
  // but that button sits in the File menu, which closed on activation, so
  // focus would drop to `<body>`. Closing it must land on the menu's
  // summary, the nearest visible control to the entry the user chose.
  it("returns focus to the File menu when the documentation dialog closes", async () => {
    const root = await openProject();

    await openMenu();
    await act(async () => {
      findButton("Export documentation…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    const close = Array.from(document.body.querySelectorAll<HTMLButtonElement>(".documentation-dialog button"))
      .find((button) => button.textContent === "Close")!;
    await act(async () => {
      close.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(document.body.querySelector(".documentation-dialog")).toBeNull();
    expect(document.activeElement).toBe(host!.querySelector(".file-menu summary"));

    await act(async () => root.unmount());
  });

  it("opens contribution analysis outside the closed File menu and restores visible focus", async () => {
    const root = await renderApp();
    const menu = await openMenu();
    const summary = menu.querySelector("summary")!;
    await act(async () => findButton("Analyze support gaps…").dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(menu.open).toBe(false);
    const panel = document.body.querySelector<HTMLElement>(".contribution-panel")!;
    expect(panel).not.toBeNull();
    expect(panel.closest(".file-menu")).toBeNull();
    expect(document.activeElement).toBe(panel.querySelector("input[type=file]"));
    const close = [...panel.querySelectorAll<HTMLButtonElement>("button")].find(b => b.textContent === "Close")!;
    await act(async () => close.click());
    expect(document.body.querySelector(".contribution-panel")).toBeNull();
    expect(document.activeElement).toBe(summary);
    await act(async () => root.unmount());
  });

  it("keeps the Debug report readable after its File menu closes and returns focus to File", async () => {
    const root = await renderApp();
    const menu = await openMenu();
    const summary = menu.querySelector("summary")!;
    await act(async () => {
      findButton("Debug report…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(menu.open).toBe(false);
    const panel = document.body.querySelector<HTMLElement>(".debug-report-panel")!;
    expect(panel).not.toBeNull();
    expect(panel.closest(".file-menu")).toBeNull();
    expect(document.activeElement).toBe(panel.querySelector("textarea"));

    const close = [...panel.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent === "Close")!;
    await act(async () => close.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(document.body.querySelector(".debug-report-panel")).toBeNull();
    expect(document.activeElement).toBe(summary);
    await act(async () => root.unmount());
  });

  // The same half, reached by keyboard. happy-dom implements no
  // activation behaviour — Enter on a focused button produces no click at
  // all — so this dispatches what a real browser dispatches on Enter: a
  // click whose `detail` is 0 because no pointer was involved. The
  // handler must not be reading coordinates or button numbers.
  it("closes on a keyboard activation, which arrives as a click with no pointer behind it", async () => {
    const root = await openProject();

    const menu = await openMenu();
    const about = findButton("About KNXBench…");
    about.focus();
    await act(async () => {
      about.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 }));
    });
    expect(menu.open).toBe(false);
    expect(host!.querySelector(".about-dialog")).not.toBeNull();

    await act(async () => root.unmount());
  });

  // F1, second half: clicking anywhere else on the page dismisses it, the
  // way every other menu on the platform behaves.
  it("closes on a pointer-down outside itself and survives one inside", async () => {
    const root = await openProject();
    const menu = await openMenu();

    await act(async () => {
      menu.querySelector("summary")!.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    });
    expect(menu.open).toBe(true);

    await act(async () => {
      document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    });
    expect(menu.open).toBe(false);

    await act(async () => root.unmount());
  });

  // The one entry that must survive its own activation: `Compare with…`
  // renders its report *inside* the menu, so closing on that click would
  // hide the answer the click asked for. `data-menu-stays-open` is the
  // opt-out, and this pins that it is honoured for both the button and
  // the report's own Close button.
  it("keeps the menu open for the entries that render their result inside it", async () => {
    apiMock.diffProject.mockResolvedValue({ infoChanges: [], installations: [] });
    const root = await openProject();
    const menu = await openMenu();

    await act(async () => {
      findButton("Compare with…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(menu.open).toBe(true);
    const panel = host!.querySelector<HTMLElement>(".project-diff-panel")!;
    expect(panel).not.toBeNull();

    await act(async () => {
      panel.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(menu.open).toBe(true);

    await act(async () => root.unmount());
  });

  // F3. Two separators between the three stacked blocks of the left
  // column, each one operable by keyboard as well as by pointer —
  // `PaneSplitter.test.tsx` drives the interaction; this pins that `App`
  // mounts them, labels them from the catalogue, and does not put them on
  // the welcome screen, where there is nothing to resize.
  it("puts a labelled separator between each pair of stacked blocks, once a project is open", async () => {
    const root = await renderApp();
    expect(host!.querySelectorAll(".pane-splitter").length).toBe(0);

    await act(async () => {
      filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxproj");
      apiMock.importProject.mockResolvedValue(baseTree());
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const splitters = [...host!.querySelectorAll<HTMLElement>(".workbench-pane-left .pane-splitter")];
    expect(splitters.length).toBe(2);
    expect(splitters.map((s) => s.getAttribute("aria-label"))).toEqual([
      enMessages["workbench.resizeNavigation"],
      enMessages["workbench.resizeDiagnostics"],
    ]);
    for (const splitter of splitters) {
      expect(splitter.getAttribute("role")).toBe("separator");
      expect(splitter.getAttribute("aria-orientation")).toBe("horizontal");
      expect(splitter.tabIndex).toBe(0);
    }

    await act(async () => root.unmount());
  });

  // F4. A browser tab cannot close itself, so the entry only exists where
  // it means something. `canQuit()` is `filePicker.ts`'s `isTauri()`,
  // which this suite already mocks.
  it("offers Quit only inside the desktop shell", async () => {
    const withoutShell = await openProject();
    await openMenu();
    expect(host!.querySelector(".file-menu-quit")).toBeNull();
    await act(async () => withoutShell.unmount());
    host!.remove();

    filePickerMock.isTauri.mockReturnValue(true);
    const withShell = await openProject();
    await openMenu();
    expect(host!.querySelector<HTMLElement>(".file-menu-quit")?.textContent)
      .toBe(enMessages["toolbar.quit"]);

    await act(async () => withShell.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("asks before quitting with unsaved work, and closes the window once it is told to", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    const root = await openProject({
      ...baseTree(),
      can_undo: false,
      is_modified: true,
    });
    await openMenu();

    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => {
      host!.querySelector(".quit-confirm-discard")!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  // §132: the window manager's close button takes the same decision as
  // File › Quit instead of ending the process unasked.
  it("keeps the window open and asks when the window manager closes a modified project", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    tauriWindowMock.destroy.mockClear();
    const root = await openProject({ ...baseTree(), is_modified: true });
    expect(tauriWindowMock.closeHandler).not.toBeNull();

    tauriWindowMock.close.mockClear();
    let mayClose = true;
    await act(async () => {
      mayClose = tauriCloseRequested();
    });
    expect(mayClose).toBe(false);
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();

    await act(async () => {
      host!.querySelector(".quit-confirm-discard")!
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);
    expect(tauriWindowMock.close).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    expect(tauriWindowMock.closeHandler).toBeNull();
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("lets the window manager close a clean project without a prompt", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    const root = await openProject({ ...baseTree(), is_modified: false });
    expect(tauriWindowMock.closeHandler).not.toBeNull();

    let mayClose = false;
    await act(async () => {
      mayClose = tauriCloseRequested();
    });
    expect(mayClose).toBe(true);
    expect(host!.querySelector(".quit-confirm")).toBeNull();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("does not listen for window close outside the desktop shell", async () => {
    tauriWindowMock.onCloseRequested.mockClear();
    const root = await openProject({ ...baseTree(), is_modified: true });
    expect(tauriWindowMock.onCloseRequested).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });

  it("quits straight away when there is nothing to lose", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    const root = await openProject({
      ...baseTree(),
      can_undo: true,
      is_modified: false,
    });
    await openMenu();

    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).toBeNull();
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("saves and quits from the quit prompt only once the save left the project clean (ISSUE-04)", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1);
    apiMock.openProject.mockResolvedValue(dirty);
    apiMock.currentProject.mockResolvedValue({ ...dirty, is_modified: false, has_store_path: true });
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => {
      findButton(enMessages["quit.save"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.saveProject).toHaveBeenCalledTimes(1);
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("keeps the quit prompt and the project open when Save and quit fails or leaves edits behind", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1);
    apiMock.openProject.mockResolvedValue(dirty);
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    const clickSave = async () => {
      await act(async () => {
        findButton(enMessages["quit.save"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
    };

    // 1. The server refuses the save.
    apiMock.saveProject.mockRejectedValueOnce(new Error("disk full"));
    await clickSave();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();

    // 2. The save worked, but a newer edit means the project is still dirty.
    apiMock.currentProject.mockResolvedValueOnce(treeAt({ ...dirty, is_modified: true }, 2));
    await clickSave();
    expect(apiMock.saveProject).toHaveBeenCalledTimes(2);
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();

    expect(tauriWindowMock.close).not.toHaveBeenCalled();
    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("does not quit when the user cancels while Save and quit is still saving", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1);
    apiMock.openProject.mockResolvedValue(dirty);
    apiMock.currentProject.mockResolvedValue({ ...dirty, is_modified: false, has_store_path: true });
    let finishSave!: () => void;
    apiMock.saveProject.mockImplementationOnce(
      () => new Promise<void>((resolve) => { finishSave = resolve; }),
    );
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(enMessages["quit.save"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    // The user changes their mind while the save is still on the wire.
    await act(async () => {
      findButton(enMessages["quit.cancel"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).toBeNull();

    await act(async () => {
      finishSave();
    });
    // The save itself still lands; the withdrawn quit does not.
    expect(apiMock.saveProject).toHaveBeenCalledTimes(1);
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("keeps the quit prompt open when the Save-As picker behind Save and quit is cancelled", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    // An ETS import has no .knxdb location yet, so Save means Save As.
    const root = await openProject({ ...baseTree(), can_undo: true, is_modified: true });
    filePickerMock.pickSavePath.mockResolvedValue(null);

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(enMessages["quit.save"]).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(filePickerMock.pickSavePath).toHaveBeenCalledTimes(1);
    expect(apiMock.saveProjectAs).not.toHaveBeenCalled();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("uses the authoritative clean tree after Save before deciding whether Quit needs a prompt", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1);
    apiMock.openProject.mockResolvedValue(dirty);
    apiMock.currentProject.mockResolvedValue({
      ...dirty,
      is_modified: false,
      has_store_path: true,
    });
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.saveProject).toHaveBeenCalledTimes(1);
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).toBeNull();
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("uses the authoritative clean tree after Save As before deciding whether Quit needs a prompt", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickSavePath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = { ...baseTree(), can_undo: true, is_modified: true };
    apiMock.currentProject.mockResolvedValue({
      ...dirty,
      is_modified: false,
      has_store_path: true,
    });
    const root = await openProject(dirty);

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.saveAs"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.saveProjectAs).toHaveBeenCalledWith("/tmp/project.knxdb");
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).toBeNull();
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it.each([
    { action: "Save", nativeOpen: true },
    { action: enMessages["toolbar.saveAs"], nativeOpen: false },
  ])("does not let a delayed clean refresh after $action erase a newer accepted edit", async ({ action, nativeOpen }) => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue(nativeOpen ? "/tmp/project.knxdb" : "/tmp/project.knxproj");
    filePickerMock.pickSavePath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = { ...baseTree(), can_undo: true, is_modified: true };
    if (nativeOpen) apiMock.openProject.mockResolvedValue(dirty);
    else apiMock.importProject.mockResolvedValue(dirty);
    let finishRefresh!: (tree: ProjectTree & { has_store_path: boolean }) => void;
    apiMock.currentProject.mockReturnValue(new Promise((resolve) => { finishRefresh = resolve; }));
    apiMock.undo.mockResolvedValue(treeAt({ ...dirty, can_undo: false, can_redo: true }, 3));
    const root = await renderApp();

    await act(async () => {
      findButton(nativeOpen ? "Open (.knxdb)…" : "Open project…")
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(action).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);

    await act(async () => {
      findButton(enMessages["toolbar.undo"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.undo).toHaveBeenCalledTimes(1);

    await act(async () => {
      finishRefresh({ ...dirty, snapshot_revision: 2, can_undo: true, is_modified: false, has_store_path: true });
    });
    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("does not let a delayed older edit response overwrite a newer clean Save snapshot", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1);
    apiMock.openProject.mockResolvedValue(dirty);
    let finishEdit!: (tree: ProjectTree) => void;
    apiMock.undo.mockReturnValue(new Promise((resolve) => { finishEdit = resolve; }));
    apiMock.currentProject.mockResolvedValue({
      ...treeAt(baseTree(), 3),
      has_store_path: true,
    });
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    act(() => {
      findButton(enMessages["toolbar.undo"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      finishEdit(treeAt({ ...dirty, can_undo: false, can_redo: true }, 2));
    });

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).toBeNull();
    expect(tauriWindowMock.destroy).toHaveBeenCalledTimes(1);

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("does not let a delayed older load response erase a newer accepted edit", async () => {
    filePickerMock.pickOpenPath
      .mockResolvedValueOnce("/tmp/old.knxproj")
      .mockResolvedValueOnce("/tmp/replacement.knxproj");
    apiMock.importProject.mockResolvedValueOnce(
      treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1),
    );
    let finishLoad!: (tree: ProjectTree) => void;
    apiMock.importProject.mockReturnValueOnce(new Promise((resolve) => { finishLoad = resolve; }));
    apiMock.undo.mockResolvedValue(
      treeAt({ ...treeWithDevice(), can_redo: true, is_modified: true }, 3),
    );
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    act(() => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(enMessages["toolbar.undo"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.textContent).toContain("Device D");
    apiMock.deviceDetail.mockResolvedValue(deviceDetailFixture());
    await act(async () => {
      treeLabel("Device D").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".tree-label.selected")?.textContent)
      .toContain("Device D");

    await act(async () => {
      finishLoad(treeAt(baseTree(), 2));
    });
    expect(host!.textContent).toContain("Device D");
    expect(host!.querySelector(".tree-label.selected")?.textContent)
      .toContain("Device D");

    await act(async () => root.unmount());
  });

  it("does not let a delayed Save As refresh replace a subsequently loaded project or its save-path state", async () => {
    // Queue only this test's loads: a leftover one-shot reply from an earlier
    // test would hand back a clean tree and skip F1's dialog.
    apiMock.importProject.mockReset();
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/old.knxproj");
    filePickerMock.pickSavePath
      .mockResolvedValueOnce("/tmp/old.knxdb")
      .mockResolvedValueOnce(null);
    apiMock.importProject.mockResolvedValueOnce(
      treeAt({ ...baseTree(), can_undo: true, is_modified: true }, 1),
    );
    let finishRefresh!: (tree: ProjectTree & { has_store_path: boolean }) => void;
    apiMock.currentProject.mockReturnValue(new Promise((resolve) => { finishRefresh = resolve; }));
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(enMessages["toolbar.saveAs"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);

    apiMock.importProject.mockResolvedValueOnce(treeAt(treeWithDevice(), 3));
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    // The first project is still modified, so F1's dialog asks first.
    await confirmReplaceDiscard();
    expect(host!.textContent).toContain("Device D");

    await act(async () => {
      finishRefresh({ ...treeAt(baseTree(), 2), has_store_path: true });
    });
    expect(host!.textContent).toContain("Device D");

    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(filePickerMock.pickSavePath).toHaveBeenCalledTimes(2);
    expect(apiMock.saveProject).not.toHaveBeenCalled();

    await act(async () => root.unmount());
  });

  it("accepts revision one from a restarted server and rejects a delayed reply from its retired process", async () => {
    // Queue only this test's loads: a leftover one-shot reply from an earlier
    // test would hand back a clean tree and skip F1's dialog.
    apiMock.importProject.mockReset();
    window.localStorage.clear();
    filePickerMock.pickOpenPath
      .mockResolvedValueOnce("/tmp/old.knxproj")
      .mockResolvedValueOnce("/tmp/replacement.knxproj");
    filePickerMock.pickSavePath
      .mockResolvedValueOnce("/tmp/old.knxdb")
      .mockResolvedValueOnce(null);
    apiMock.importProject.mockResolvedValueOnce(
      treeFromProcess({ ...baseTree(), can_undo: true, is_modified: true }, "process-a", 100),
    );
    let finishRetiredRefresh!: (tree: ProjectTree & { has_store_path: boolean }) => void;
    apiMock.currentProject.mockReturnValue(
      new Promise((resolve) => { finishRetiredRefresh = resolve; }),
    );
    const root = await renderApp();

    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton(enMessages["toolbar.saveAs"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    apiMock.importProject.mockResolvedValueOnce(
      treeFromProcess(treeWithDevice(), "process-b", 1),
    );
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    // The first project is still modified, so F1's dialog asks first.
    await confirmReplaceDiscard();
    expect(host!.textContent).toContain("Device D");
    apiMock.deviceDetail.mockResolvedValue(deviceDetailFixture());
    await act(async () => {
      treeLabel("Device D").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    await act(async () => {
      finishRetiredRefresh({
        ...treeFromProcess(baseTree(), "process-a", 101),
        has_store_path: true,
      });
    });
    expect(host!.textContent).toContain("Device D");
    expect(host!.querySelector(".tree-label.selected")?.textContent).toContain("Device D");

    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(filePickerMock.pickSavePath).toHaveBeenCalledTimes(2);
    expect(apiMock.saveProject).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    window.localStorage.clear();
  });

  it("keeps the dirty tree and reports an authoritative refresh failure after Save", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = { ...baseTree(), can_undo: true, is_modified: true };
    apiMock.openProject.mockResolvedValue(dirty);
    apiMock.currentProject.mockRejectedValue(new Error("current project unavailable"));
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector('[role="alert"]')?.textContent)
      .toContain("current project unavailable");

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });

  it("does not refresh or clear dirty state when Save itself fails", async () => {
    filePickerMock.isTauri.mockReturnValue(true);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/project.knxdb");
    const dirty = { ...baseTree(), can_undo: true, is_modified: true };
    apiMock.openProject.mockResolvedValue(dirty);
    apiMock.saveProject.mockRejectedValueOnce(new Error("disk full"));
    const root = await renderApp();

    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      findButton("Save").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.currentProject).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("disk full");

    await openMenu();
    await act(async () => {
      findButton(enMessages["toolbar.quit"])
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.querySelector(".quit-confirm")).not.toBeNull();
    expect(tauriWindowMock.destroy).not.toHaveBeenCalled();

    await act(async () => root.unmount());
    filePickerMock.isTauri.mockReturnValue(false);
  });
});

// T01b / ADR-0026. The logout control is the only thing `App` learns from
// the auth gate, and the only interesting thing about it is when it is
// absent: a server started without a password, and the desktop shell,
// both report `required: false` and must show no way to end a session that
// does not exist.
describe("App — the logout control (T01b)", () => {
  async function openMenu() {
    const summary = host!.querySelector<HTMLElement>(".file-menu summary")!;
    await act(async () => summary.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  }

  it("offers no logout when the gate reports no session at all", async () => {
    const root = await renderApp(undefined, { required: false, logout: vi.fn() });
    await openMenu();

    expect(host!.querySelector(".file-menu-logout")).toBeNull();
    expect(host!.textContent).not.toContain(enMessages["toolbar.logout"]);

    await act(async () => root.unmount());
  });

  it("offers no logout when nothing told it about a session", async () => {
    // How the desktop shell and every existing test render `App`: no
    // `session` prop at all.
    const root = await renderApp();
    await openMenu();

    expect(host!.querySelector(".file-menu-logout")).toBeNull();

    await act(async () => root.unmount());
  });

  it("offers logout, and calls it, when a session exists", async () => {
    const logout = vi.fn();
    const root = await renderApp(undefined, { required: true, logout });
    await openMenu();

    const entry = host!.querySelector<HTMLButtonElement>(".file-menu-logout")!;
    expect(entry.textContent).toBe(enMessages["toolbar.logout"]);

    await act(async () => entry.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(logout).toHaveBeenCalledTimes(1);
    // F1's menu manners apply to this entry like any other: it closes the
    // menu behind it.
    expect(host!.querySelector<HTMLDetailsElement>(".file-menu")!.open).toBe(false);

    await act(async () => root.unmount());
  });
});

// AR08: a protected ETS project asks for its password instead of failing,
// retries the same import with it, and never keeps the password anywhere.
describe("App — project password", () => {
  const passwordRefusal = (kind: string) =>
    Object.assign(new Error("project password needed"), { status: 422, body: { error: "project password needed", kind } });

  async function openProtected() {
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/villa.knxproj");
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
    return root;
  }

  async function enter(password: string) {
    const input = document.querySelector<HTMLInputElement>('input[type="password"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, password);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      Array.from(document.querySelectorAll("button")).find((b) => b.textContent === "Import")!.click();
    });
    await act(async () => {});
  }

  it("asks, retries with the password, and loads the project without an error", async () => {
    apiMock.importProject.mockReset();
    apiMock.importProject
      .mockRejectedValueOnce(passwordRefusal("projectPasswordRequired"))
      .mockResolvedValueOnce(baseTree());
    const root = await openProtected();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain("villa.knxproj");
    // A password refusal is a question, not an error: no toast, no failure banner.
    expect(document.body.textContent).not.toContain("project password needed");
    expect(document.body.textContent).not.toContain("Could not load");
    await enter("s3cret");
    expect(apiMock.importProject).toHaveBeenCalledTimes(2);
    expect(apiMock.importProject.mock.calls[0]).toHaveLength(2);
    expect(apiMock.importProject.mock.calls[1][0]).toBe("/tmp/villa.knxproj");
    expect(apiMock.importProject.mock.calls[1][2]).toBe("s3cret");
    expect(document.querySelector('input[type="password"]')).toBeNull();
    expect(host!.querySelector(".project-explorer")).not.toBeNull();
    expect(JSON.stringify(window.localStorage)).not.toContain("s3cret");
    root.unmount();
  });

  it("never asks for a password when opening a native project", async () => {
    apiMock.openProject.mockReset();
    apiMock.openProject.mockRejectedValueOnce(passwordRefusal("projectPasswordRequired"));
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/villa.knxdb");
    const root = await renderApp();
    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
    expect(document.querySelector('input[type="password"]')).toBeNull();
    expect(apiMock.openProject).toHaveBeenCalledTimes(1);
    expect(apiMock.importProject).not.toHaveBeenCalled();
    root.unmount();
  });

  it("asks again after a wrong password and stops quietly on cancel", async () => {
    apiMock.importProject.mockReset();
    apiMock.importProject
      .mockRejectedValueOnce(passwordRefusal("projectPasswordRequired"))
      .mockRejectedValueOnce(passwordRefusal("projectPasswordWrong"));
    const root = await openProtected();
    await enter("guess");
    expect(document.body.textContent).toContain("was not accepted");
    await act(async () => {
      Array.from(document.querySelectorAll("button")).find((b) => b.textContent === "Cancel")!.click();
    });
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.body.textContent).not.toContain("project password needed");
    expect(document.body.textContent).not.toContain("Could not load");
    expect(apiMock.importProject).toHaveBeenCalledTimes(2);
    expect(host!.querySelector(".project-explorer")).toBeNull();
    root.unmount();
  });
});

// AR18 independent review F1: File › Open / Import used to replace a project
// with unsaved edits without a word. They now ask first, like Quit does.
describe("App — unsaved edits before opening another project", () => {
  const unsavedRefusal = () =>
    Object.assign(new Error("the open project has unsaved changes"), {
      status: 409,
      body: { error: "the open project has unsaved changes", kind: "projectUnsavedChanges" },
    });

  async function openDirtyProject() {
    apiMock.openProject.mockReset();
    apiMock.importProject.mockReset();
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/current.knxdb");
    apiMock.openProject.mockResolvedValueOnce(treeAt({ ...baseTree(), is_modified: true }, 1));
    const root = await renderApp();
    await act(async () => {
      findButton("Open (.knxdb)…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.openProject).toHaveBeenCalledTimes(1);
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/other.knxproj");
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
    return root;
  }

  const dialog = () => document.querySelector<HTMLElement>(".replace-confirm");
  const click = async (text: string) => {
    await act(async () => {
      Array.from(dialog()!.querySelectorAll("button")).find((b) => b.textContent === text)!.click();
    });
    await act(async () => {});
  };

  it("asks before importing over unsaved edits and sends nothing on cancel", async () => {
    const root = await openDirtyProject();
    expect(dialog()?.textContent).toContain(enMessages["replace.message"]);
    expect(dialog()?.textContent).toContain("other.knxproj");
    expect(apiMock.importProject).not.toHaveBeenCalled();
    await click(enMessages["replace.cancel"]);
    expect(dialog()).toBeNull();
    expect(apiMock.importProject).not.toHaveBeenCalled();
    root.unmount();
  });

  it("discards only on the explicit button, and says so on the wire", async () => {
    const root = await openDirtyProject();
    apiMock.importProject.mockResolvedValueOnce(baseTree());
    await click(enMessages["replace.discard"]);
    expect(dialog()).toBeNull();
    expect(apiMock.importProject).toHaveBeenCalledTimes(1);
    const [path, , password, discard] = apiMock.importProject.mock.calls[0];
    expect([path, password, discard]).toEqual(["/tmp/other.knxproj", undefined, true]);
    root.unmount();
  });

  it("saves first and then imports without discarding anything", async () => {
    const root = await openDirtyProject();
    apiMock.saveProject.mockResolvedValueOnce(undefined);
    apiMock.currentProject.mockResolvedValueOnce(treeAt({ ...baseTree(), is_modified: false }, 2));
    apiMock.importProject.mockResolvedValueOnce(baseTree());
    await click(enMessages["replace.save"]);
    expect(apiMock.saveProject).toHaveBeenCalled();
    expect(apiMock.importProject).toHaveBeenCalledTimes(1);
    expect(apiMock.importProject.mock.calls[0]).toHaveLength(2);
    root.unmount();
  });

  it("turns the server's late refusal into the same question, not an error", async () => {
    apiMock.importProject.mockReset();
    apiMock.importProject.mockRejectedValueOnce(unsavedRefusal());
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/other.knxproj");
    const root = await renderApp();
    await act(async () => {
      findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {});
    expect(dialog()?.textContent).toContain("other.knxproj");
    expect(document.body.textContent).not.toContain("Could not load");
    root.unmount();
  });

  it("keeps the discard decision for the password retry of the same import", async () => {
    const root = await openDirtyProject();
    apiMock.importProject
      .mockRejectedValueOnce(
        Object.assign(new Error("project password needed"), {
          status: 422,
          body: { error: "project password needed", kind: "projectPasswordRequired" },
        }),
      )
      .mockResolvedValueOnce(baseTree());
    await click(enMessages["replace.discard"]);
    const input = document.querySelector<HTMLInputElement>('input[type="password"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "s3cret");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      Array.from(document.querySelectorAll("button")).find((b) => b.textContent === "Import")!.click();
    });
    await act(async () => {});
    expect(apiMock.importProject).toHaveBeenCalledTimes(2);
    expect(apiMock.importProject.mock.calls[1].slice(2)).toEqual(["s3cret", true]);
    root.unmount();
  });
});

describe("App — the first-run guide on request (ADR-0084)", () => {
  // The start-up decision needs an acknowledged settings record, which this
  // file never serves, so the guide only appears when asked for; its timing
  // rules are `useOnboardingGuide.test.tsx`'s. What is pinned here is the
  // wiring: the File menu opens it, and a task button reaches the real
  // workspace through the palette's command context.
  it("opens from the File menu and its product-data task lands in the catalog workspace", async () => {
    const root = await renderApp();
    expect(host!.querySelector(".onboarding-guide")).toBeNull();
    await act(async () => findButton(enMessages["command.showIntroduction"]).click());
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const guide = document.querySelector<HTMLElement>(".onboarding-guide");
    expect(guide).not.toBeNull();
    // `serverVersion` answers "0.0.0-test" in this file: a pre-release whose
    // label names no stage — which is also why it never opens by itself.
    expect(guide!.querySelector(".onboarding-stage")?.textContent).toBe(enMessages["programmingConsent.stage.preRelease"]);

    const next = () => Array.from(guide!.querySelectorAll("button")).find((b) => b.textContent === enMessages["onboarding.next"])!;
    await act(async () => next().click());
    await act(async () => next().click());
    const task = Array.from(guide!.querySelectorAll<HTMLButtonElement>(".onboarding-task"))
      .find((b) => b.textContent?.includes(enMessages["onboarding.task.productData.title"]))!;
    await act(async () => task.click());
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 50)); });

    expect(document.querySelector(".onboarding-guide")).toBeNull();
    expect(host!.querySelector(".workbench-center .catalog-workspace")).not.toBeNull();
    await act(async () => root.unmount());
  });
});

// ADR-0089: an ETS import reports the import report's own counts.
describe("App — achievement events from loading a project", () => {
  async function load(path: string, button = "Open project…"): Promise<{ seen: AchievementEvent[]; root: { unmount(): void } }> {
    filePickerMock.pickOpenPath.mockResolvedValue(path);
    const seen: AchievementEvent[] = [];
    const stop = subscribeAchievementEvents((event) => seen.push(event));
    const root = await renderApp();
    try {
      await act(async () => {
        findButton(button).dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
      await act(async () => {});
    } finally {
      stop();
    }
    return { seen, root };
  }

  it("reports an ETS import with its losses, notices and devices", async () => {
    apiMock.importProject.mockReset().mockResolvedValue({ ...treeWithDevice(), errors: 2, warnings: 5 });
    const { seen, root } = await load("/tmp/villa.knxproj");
    expect(seen.filter((event) => event.type === "etsImported")).toEqual([
      { type: "etsImported", lostItems: 2, notices: 5, deviceCount: 1, passwordProtected: false },
    ]);
    // The opened project is measured too (counting itself: achievementObservation.test.ts).
    expect(seen.filter((event) => event.type === "projectObserved").at(-1)).toMatchObject({ groupAddressCount: 0 });
    await act(async () => root.unmount());
  });

  it("says when the imported project needed its password", async () => {
    const refusal = Object.assign(new Error("project password needed"), {
      status: 422,
      body: { error: "project password needed", kind: "projectPasswordRequired" },
    });
    apiMock.importProject.mockReset().mockRejectedValueOnce(refusal).mockResolvedValueOnce(baseTree());
    const seen: AchievementEvent[] = [];
    const stop = subscribeAchievementEvents((event) => seen.push(event));
    filePickerMock.pickOpenPath.mockResolvedValue("/tmp/villa.knxproj");
    const root = await renderApp();
    try {
      await act(async () => {
        findButton("Open project…").dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
      await act(async () => {});
      const input = document.querySelector<HTMLInputElement>('input[type="password"]')!;
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "s3cret");
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await act(async () => {
        Array.from(document.querySelectorAll("button")).find((b) => b.textContent === "Import")!.click();
      });
      await act(async () => {});
    } finally {
      stop();
    }
    expect(seen.filter((event) => event.type === "etsImported")).toEqual([
      { type: "etsImported", lostItems: 0, notices: 0, deviceCount: 0, passwordProtected: true },
    ]);
    // The password itself never travels on the achievement channel.
    expect(JSON.stringify(seen)).not.toContain("s3cret");
    await act(async () => root.unmount());
  });

  it("does not call opening a KNXBench project an ETS import", async () => {
    apiMock.openProject.mockReset().mockResolvedValue(baseTree());
    const { seen, root } = await load("/tmp/house.knxdb", "Open (.knxdb)…");
    expect(seen.some((event) => event.type === "projectOpened")).toBe(true);
    expect(seen.some((event) => event.type === "etsImported")).toBe(false);
    await act(async () => root.unmount());
  });
});


describe("App — Devices navigation", () => {
  it("removes stale editable details when a snapshot refresh cannot find the selected device", async () => {
    apiMock.deviceDetail.mockResolvedValue({ ...deviceDetailFixture(), id: 1, name: "Device A" });
    const original = treeWithDragTargets(); original.can_undo = true;
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxdb"); apiMock.openProject.mockResolvedValue(original);
    const root = await renderApp();
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-labelledby="welcome-native-title"]')!.click());
    await act(async () => findButton("Topology").click());
    await act(async () => host!.querySelector<HTMLButtonElement>(".diagram-device")!.click());
    const removed = treeWithDragTargets(); removed.installations[0].topology[0].lines[0].devices = [];
    apiMock.undo.mockResolvedValueOnce(removed); apiMock.deviceDetail.mockRejectedValueOnce(new Error("device not found"));
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Undo"]')!.click());
    expect(host!.querySelector(".device-workspace"), "a missing selected device cannot retain editable fields").toBeNull();
    await act(async () => root.unmount());
  });

  it("does not offer a device list with no project open", async () => {
    const root = await renderApp();
    expect(findButton("Devices").disabled).toBe(true);
    await act(async () => root.unmount());
  });

  it("drops deleted ids from a preserved device multi-selection on a new snapshot", async () => {
    const original = treeWithDragTargets(); original.can_undo = true;
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxdb");
    apiMock.openProject.mockResolvedValue(original);
    const root = await renderApp();
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-labelledby="welcome-native-title"]')!.click());
    await act(async () => findButton("Devices").click());
    await act(async () => host!.querySelector<HTMLInputElement>(".devices-table tbody input")!.click());
    expect(host!.querySelector(".bulk-action-toolbar")).not.toBeNull();
    const removed = treeWithDragTargets(); removed.installations[0].topology[0].lines[0].devices = [];
    apiMock.undo.mockResolvedValueOnce(removed);
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Undo"]')!.click());
    expect(host!.querySelector(".bulk-action-toolbar"), "deleted ids cannot remain bulk-action targets").toBeNull();
    await act(async () => root.unmount());
  });

  it("opens a device from a group-address link and returns to that address", async () => {
    const project = treeWithDragTargets();
    project.installations[0].group_addresses = [{ id: 500, name: "Kitchen light", address: "1/1/1",
      range: null, dpts: [], links: [{ device_id: 1, device_name: "Device A", device_address: null,
        com_object_id: 20, com_object_number: 1, com_object_name: "Switch", direction: "Receive" }] }];
    filePickerMock.pickOpenPath.mockResolvedValue("/project.knxdb");
    apiMock.openProject.mockResolvedValue(project);
    apiMock.deviceDetail.mockResolvedValue({ ...deviceDetailFixture(), id: 1, name: "Device A" });
    const root = await renderApp();
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-labelledby="welcome-native-title"]')!.click());
    await act(async () => findButton("Group addresses").click());
    await act(async () => host!.querySelector<HTMLButtonElement>(".address-table .table-select")!.click());
    const link = host!.querySelector<HTMLButtonElement>(".address-links-panel .device-link");
    expect(link, "the linked project device must be a navigation button").not.toBeNull();
    await act(async () => link!.click());
    expect(host!.querySelector(".device-workspace h2")?.textContent).toBe("Device A");
    await act(async () => findButton("← Back").click());
    expect(host!.querySelector(".address-links-panel")?.textContent).toContain("1/1/1");
    await act(async () => root.unmount());
  });

  it("opens the device list from the dashboard device count", async () => {
    const root = await openDragProject();
    const count = host!.querySelector<HTMLButtonElement>(".dashboard .devices-count-link");
    expect(count, "dashboard count must link to all project devices").not.toBeNull();
    await act(async () => count!.click());
    expect(host!.querySelector(".devices-workspace")?.closest("[hidden]")).toBeNull();
    await act(async () => root.unmount());
  });

  it("opens the project-wide device list from the left navigation", async () => {
    const root = await openDragProject();
    await act(async () => findButton("Devices").click());
    expect(host!.querySelector(".devices-workspace h1")?.textContent).toBe("Devices");
    expect(host!.querySelectorAll(".devices-table tbody tr")).toHaveLength(1);
    expect(host!.querySelector(".devices-table")?.textContent).toContain("Device A");
    expect(host!.querySelector('.workbench-navigation button[aria-current="page"]')?.textContent).toBe("Devices");
    await act(async () => root.unmount());
  });

  it("replaces topology with the device editor and returns to the original view", async () => {
    apiMock.deviceDetail.mockResolvedValue({ ...deviceDetailFixture(), id: 1, name: "Device A" });
    const root = await openDragProject();
    await act(async () => findButton("Topology").click());
    await act(async () => host!.querySelector<HTMLButtonElement>(".diagram-device")!.click());
    expect(host!.querySelector(".workbench-center .device-workspace h2")?.textContent).toBe("Device A");
    expect(host!.querySelector(".workbench-center .structure-workspace")).toBeNull();
    expect(host!.querySelector('.workbench-navigation button[aria-current="page"]')?.textContent).toBe("Devices");
    await act(async () => findButton("← Back").click());
    expect(host!.querySelector(".workbench-center .structure-workspace h1")?.textContent).toBe("Topology");
    expect(host!.querySelector(".workbench-center .device-workspace")).toBeNull();
    await act(async () => root.unmount());
  });
});
