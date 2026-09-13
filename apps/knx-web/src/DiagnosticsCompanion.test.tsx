/** Tests that the companion window edits nothing, undoes nothing and starts no second session. */
// @vitest-environment happy-dom
//
// The four safety properties of the companion are the reason this file
// exists. Three of them are absences, and an absence is exactly the kind
// of property that rots quietly: nothing fails when a future edit adds an
// import, a button or a start call, it just stops being true. So two of
// the three are asserted against the module's own source text, and the
// rest against its behaviour at runtime.
import { readFileSync } from "node:fs";
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

import DiagnosticsCompanion from "./DiagnosticsCompanion";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// Via `fileURLToPath`, not `new URL(..., import.meta.url)`: happy-dom
// replaces the global `URL`, and `readFileSync` rejects the result of that
// constructor with "The URL must be of scheme file". Same idiom as
// `diagnosticShell.test.ts`, for the same reason.
const SOURCE = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "DiagnosticsCompanion.tsx"), "utf8");

let host: HTMLDivElement | undefined;

function notFoundError(): Error & { status: number } {
  const error = new Error("no bus session") as Error & { status: number };
  error.status = 404;
  return error;
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
  apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
  apiMock.getSessionLog.mockResolvedValue([]);
});

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

describe("one editing workspace", () => {
  // The import list is the guarantee. Anything reachable from this module
  // is reachable from the companion window, so the check is on what the
  // module is allowed to import, not on what today's render happens to
  // show.
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
    const logTab = Array.from(host!.querySelectorAll(".companion-tabs button")).find(
      (b) => b.textContent === "Log",
    )!;
    await act(async () => {
      logTab.dispatchEvent(new MouseEvent("click", { bubbles: true }));
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

  it("calls no mutating endpoint while it lives, only the two read paths", async () => {
    const root = await renderCompanion();
    await act(async () => {
      root.unmount();
    });

    expect(apiMock.undo).not.toHaveBeenCalled();
    expect(apiMock.redo).not.toHaveBeenCalled();
    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
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
