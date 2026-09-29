/** Tests that the companion window edits nothing, undoes nothing and starts no second session. */
// @vitest-environment happy-dom
//
// The four safety properties of the companion are the reason this file
// exists. Three of them are absences, and an absence is exactly the kind
// of property that rots quietly: nothing fails when a future edit adds an
// import, a button or a start call, it just stops being true. So two of
// the three are asserted against the module's own source text, and the
// rest against its behaviour at runtime.
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  startBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  pollBusTelegrams: vi.fn(),
  writeBusValue: vi.fn(),
  getSessionLog: vi.fn(),
  discoverBusInterfaces: vi.fn().mockResolvedValue({ interfaces: [] }),
  undo: vi.fn(),
  redo: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error && typeof (error as { status: unknown }).status === "number"
      ? (error as { status: number }).status
      : undefined,
}));

import { resetBusDiscoveryForTests } from "./busDiscovery";
import DiagnosticsCompanion from "./DiagnosticsCompanion";
import { resetSettingsForTests } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// Via `fileURLToPath`, not `new URL(..., import.meta.url)`: happy-dom
// replaces the global `URL`, and `readFileSync` rejects the result of that
// constructor with "The URL must be of scheme file". Same idiom as
// `diagnosticShell.test.ts`, for the same reason.
const SRC_DIR = dirname(fileURLToPath(import.meta.url));
const SOURCE = readFileSync(join(SRC_DIR, "DiagnosticsCompanion.tsx"), "utf8");

/// Resolves a relative import specifier the way Vite does for this source
/// tree — extensionless, `.ts`/`.tsx`, or a directory index. Returns a
/// path relative to `src/`, so the pinned list above reads as module names
/// rather than as this machine's directory layout. Linux-first, like the
/// rest of the product: the separator in the pinned names is `/`.
function resolveModule(fromRelative: string, specifier: string): string | null {
  const base = join(dirname(fromRelative), specifier);
  for (const extension of [".ts", ".tsx", "/index.ts", "/index.tsx"]) {
    if (existsSync(join(SRC_DIR, base + extension))) return base + extension;
  }
  return null; // A package import, or a `.json`/asset — neither carries our code.
}

/// Breadth-first over relative imports, skipping `import type` because a
/// type import is erased before anything can run and so cannot reach an
/// endpoint. Returns module name → source text.
function valueImportGraph(entry: string): Map<string, string> {
  const graph = new Map<string, string>();
  const queue = [entry];
  while (queue.length > 0) {
    const current = queue.shift()!;
    if (graph.has(current)) continue;
    const source = readFileSync(join(SRC_DIR, current), "utf8");
    graph.set(current, source);
    for (const match of source.matchAll(/^import\s+(type\s+)?[^;]*?from\s+"(\.[^"]+)";$/gm)) {
      if (match[1] !== undefined) continue;
      const resolved = resolveModule(current, match[2]);
      if (resolved !== null && !graph.has(resolved)) queue.push(resolved);
    }
  }
  return graph;
}

/// Every `api.x(` call site in the graph. `api.ts` itself is excluded —
/// inside it, `api.` is prose in comments, not a call into the module.
/// Matched across a line break too: `LogPanel` writes `api` and
/// `.getSessionLog()` on separate lines, and a single-line regex silently
/// misses it.
function apiCallsIn(graph: Map<string, string>): string[] {
  const calls = new Set<string>();
  for (const [module, source] of graph) {
    if (module === "api.ts") continue;
    for (const match of source.matchAll(/\bapi\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)) {
      calls.add(match[1]);
    }
  }
  return [...calls];
}

/// Every literal `fetch` target in the graph given a method other than the
/// default `GET`. Deliberately not restricted to `/api/` — a mutation
/// posted anywhere should show up here.
function mutatingFetchesIn(graph: Map<string, string>): string[] {
  const targets = new Set<string>();
  for (const source of graph.values()) {
    for (const match of source.matchAll(/fetch\(\s*["`]([^"`]+)["`]\s*,\s*\{([^}]*)\}/g)) {
      if (/method\s*:\s*["'](?!GET)/.test(match[2])) targets.add(match[1]);
    }
  }
  return [...targets];
}

let host: HTMLDivElement | undefined;

function notFoundError(): Error & { status: number } {
  const error = new Error("no bus session") as Error & { status: number };
  error.status = 404;
  return error;
}

/// The `api` exports the mock recorded at least one call to. Reading the
/// mock rather than naming suspects is the point: the question is "what
/// did it touch", not "did it touch the things we thought of".
function calledApiExports(): string[] {
  return Object.entries(apiMock)
    .filter(([, fn]) => fn.mock.calls.length > 0)
    .map(([name]) => name)
    .sort();
}

function logTab(): Element {
  return Array.from(host!.querySelectorAll(".companion-tabs button")).find(
    (b) => b.textContent === "Log",
  )!;
}

async function renderCompanion() {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<DiagnosticsCompanion />);
  });
  return root;
}

beforeEach(() => {
  window.localStorage.clear();
  resetSettingsForTests();
  apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
  apiMock.getSessionLog.mockResolvedValue([]);
});

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

describe("one editing workspace", () => {
  // This file's import list is a tripwire, not a guarantee — the
  // distinction the previous version of this comment got wrong. A depth-1
  // list says what *this* module names; it says nothing about what those
  // modules in turn reach, and in fact `./diagnosticsWindow` reaches
  // `./filePicker` reaches `./FsPicker`, which `POST`s to
  // `/api/fs/upload`. What the tripwire is good for is catching an editing
  // import added *here*, which is the likely future edit.
  // "the companion reaches no project mutation" is asserted separately,
  // over the whole graph, further down.
  it("imports only the monitor, the log and their own support modules", () => {
    const imports = Array.from(SOURCE.matchAll(/^import[^;]*?from "([^"]+)";$/gm)).map((m) => m[1]);
    expect(imports.sort()).toEqual([
      "./BusMonitorPanel",
      "./LogPanel",
      "./busContext",
      "./diagnosticsWindow",
      "./i18n",
      "react",
    ]);
  });

  it("names no editing surface at all", () => {
    for (const forbidden of [
      "./App",
      "./Inspector",
      "./ProjectExplorer",
      "./CommandPalette",
      "./CatalogBrowser",
      "./StructureWorkspace",
      "./DeviceWorkspace",
      "./FsPicker",
    ]) {
      expect(SOURCE).not.toContain(`from "${forbidden}"`);
    }
  });

  // The honest version of the property the depth-1 list only gestured at.
  // Walks the transitive *value* import graph (`import type` carries no
  // runtime code, so it is skipped) and pins three facts about it: which
  // modules are in it, which `api` exports it calls, and which raw
  // `fetch`es in it are not reads. Pinned rather than pattern-matched,
  // because every entry below is a decision someone made, and a new one
  // should fail here and be argued about rather than slip in under a
  // regex that happened to allow it.
  it("reaches no project mutation anywhere in its transitive import graph", () => {
    const graph = valueImportGraph("DiagnosticsCompanion.tsx");

    // Fifteen modules, not the thirty an earlier review counted: the
    // difference is exactly the fifteen `bindings/*.ts` files, which are
    // reached only by `import type` and are erased before anything runs.
    // `filePicker` and `FsPicker` are the uncomfortable
    // two: they are here because `diagnosticsWindow` imports `isTauri`,
    // which is `typeof window !== "undefined" && "__TAURI__" in window`
    // and nothing else. The picker entry points that reach `FsPicker`'s
    // upload are `pickOpenPath`/`pickSavePath`, which this window never
    // calls.
    //
    // T28 adds `settingsStore.ts`, reached through `uiLanguage.ts` and
    // `languagePack.ts`: the companion renders translated strings, so it
    // reads the same settings document the editor does. It writes there
    // too — `PUT /api/settings` — and that write is deliberate and not a
    // project mutation: it carries preferences, one key at a time, into a
    // file that holds nothing but preferences. It does not appear in
    // `mutatingFetchesIn` below because that regex only sees a string
    // literal as `fetch`'s first argument, and this one passes a variable.
    //
    // T01b adds `session.ts`: `api.ts` publishes "the server answered 401"
    // from inside its shared `request()` helper, and that notifier lives
    // there. It holds a `Set` of callbacks and nothing else — no fetch, no
    // project state, nothing this window could mutate with — so it is the
    // cheapest possible way for the companion to end up at the same login
    // screen as the editor instead of silently failing to poll.
    //
    // T26 adds `gaNotation.ts`, reached from both bus components: the
    // monitor renders a destination group address and the compose form
    // reads one back, and both go through the display-notation preference
    // to do it. It reads and writes the same settings document
    // `settingsStore.ts` already brought into this graph, and touches no
    // project state at all.
    //
    // T25 adds `busDiscovery.ts`: the module-level result of the
    // KNXnet/IP interface search, which `BusMonitorPanel` reads. It holds
    // one value and a `Set` of subscribers, and the one request it makes
    // is the search itself — a multicast question and a read of the
    // answers, which changes nothing on any device and nothing in any
    // project.
    //
    // T10 adds `gatewayPreference.ts`: both diagnostics workflows read one
    // passive string through the already-allowed settings store. It imports
    // no API and cannot discover, connect, scan, or mutate a project; the
    // exact API-call assertion below keeps that boundary explicit.
    expect([...graph.keys()].sort()).toEqual([
      "BusComposeForm.tsx",
      "BusMonitorPanel.tsx",
      "DiagnosticsCompanion.tsx",
      "FsPicker.tsx",
      "HelpTip.tsx",
      "LogPanel.tsx",
      "Overlay.tsx",
      "api.ts",
      "busContext.ts",
      "busDiscovery.ts",
      "busMonitorCapture.ts",
      "busMonitorStatistics.ts",
      "diagnosticsWindow.ts",
      "filePicker.ts",
      "gaNotation.ts",
      "gatewayPreference.ts",
      "help.ts",
      "i18n.ts",
      "languagePack.ts",
      "localJsonDownload.ts",
      "messages/de.ts",
      "messages/en.ts",
      "session.ts",
      "sessionLogExport.ts",
      "settingsDiagnostic.ts",
      "settingsStore.ts",
      "uiLanguage.ts",
    ]);

    // Every `api.x(...)` the graph performs. Four of them are `POST`s.
    // Three write to the *bus*, never to the project, and each needs a
    // deliberate click; the fourth, `discoverBusInterfaces`, is a `POST`
    // only so that no cache or prefetch can fire it, and it writes
    // nothing anywhere — it asks by multicast who is listening and reads
    // the replies. No undo, no redo, no save, no import, no rename, no
    // parameter or flag write appears — out of 61 exported functions, 47
    // of which issue a mutating request:
    //   grep -cE '^export (async )?function ' api.ts
    //   grep -cE 'method: "(POST|PUT|DELETE|PATCH)"' api.ts
    expect(apiCallsIn(graph).sort()).toEqual([
      "discoverBusInterfaces",
      "errorMessage",
      "errorStatus",
      "getSessionLog",
      "pollBusTelegrams",
      "startBusMonitor",
      "stopBusMonitor",
      "writeBusValue",
    ]);

    // `api.ts`'s own uploader and `FsPicker`'s. Both are project-mutating
    // and both are unreachable from this window: the first is the body of
    // `installCatalog`, absent from the list above; the second is reached
    // only through the picker paths named earlier.
    expect(mutatingFetchesIn(graph).sort()).toEqual([
      "/api/catalog/install",
      "/api/fs/upload",
    ]);
  });

  it("renders the monitor, the log tab and the read-only notice, and no editor chrome", async () => {
    await renderCompanion();

    expect(host!.querySelector(".companion-read-only")!.getAttribute("role")).toBe("note");
    expect(host!.querySelector(".bus-monitor-panel")).not.toBeNull();
    expect(host!.querySelector(".project-explorer")).toBeNull();
    expect(host!.querySelector(".inspector")).toBeNull();
    expect(host!.querySelector(".toolbar")).toBeNull();
  });

  it("switches to the session log without ever offering a save or an import", async () => {
    await renderCompanion();
    await act(async () => {
      logTab().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(host!.querySelector(".log-panel")).not.toBeNull();
    const labels = Array.from(host!.querySelectorAll("button")).map((b) => b.textContent);
    expect(labels).not.toContain("Save");
    expect(labels).not.toContain("Undo");
    expect(labels).not.toContain("Redo");
  });
});

describe("no project mutation and no project undo", () => {
  // `App.tsx` binds Ctrl+Z and Ctrl+Shift+Z on `window`. That listener is
  // installed by `App`, which this window never mounts — so the keystroke
  // that would rewrite the project in the editing window does nothing
  // here. Pressed against the live companion rather than argued about.
  it("ignores Ctrl+Z and Ctrl+Shift+Z", async () => {
    await renderCompanion();
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "z", ctrlKey: true, shiftKey: true, bubbles: true }),
      );
    });

    expect(apiMock.undo).not.toHaveBeenCalled();
    expect(apiMock.redo).not.toHaveBeenCalled();
  });

  // Stated as a complement, not as named absences: the assertion is on the
  // set of `api` exports actually called, so a fourth mutator added to the
  // module and to the mock fails here instead of slipping past a list of
  // three `not.toHaveBeenCalled()`s that nobody extended.
  //
  // Left alone, the set is one entry. `LogPanel` — and so the log read —
  // mounts only once the user picks the Log tab, which the second case
  // below does; the monitor is the tab the window opens on.
  it("calls only the telegram read over an untouched lifecycle", async () => {
    const root = await renderCompanion();
    await act(async () => {
      root.unmount();
    });

    expect(calledApiExports()).toEqual(["pollBusTelegrams"]);
  });

  it("adds the log read and still nothing else once the log tab is opened", async () => {
    const root = await renderCompanion();
    await act(async () => {
      logTab().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      root.unmount();
    });

    expect(calledApiExports()).toEqual(["getSessionLog", "pollBusTelegrams"]);
  });
});

describe("one shared bus session", () => {
  it("asks whether a session exists instead of starting one", async () => {
    await renderCompanion();

    expect(apiMock.pollBusTelegrams).toHaveBeenCalledWith(0);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
  });

  // Closing the companion must not end the session the other window is
  // watching. This is the difference between a second screen and a
  // saboteur.
  it("does not stop the session when the window closes", async () => {
    const root = await renderCompanion();
    await act(async () => {
      root.unmount();
    });

    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });
});

describe("the companion's own interface search", () => {
  // `ensureBusDiscovery`'s own doc comment names this exact case: "a
  // component that needs the result mounts before [the startup search]
  // ever ran (a second window, a test rendering the panel alone)". The
  // companion is the one window that mounts `BusMonitorPanel` without
  // `App` above it, so this is the only place that path is actually
  // exercised — and until now nothing here asserted it ran at all.
  // Reset first: an earlier test in this file may already have driven
  // the module-level store past `"idle"`, which would make
  // `ensureBusDiscovery` a silent no-op and this assertion vacuous.
  it("runs the interface search on mount and does not start a session", async () => {
    resetBusDiscoveryForTests();
    await renderCompanion();

    expect(apiMock.discoverBusInterfaces).toHaveBeenCalled();
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
  });
});
