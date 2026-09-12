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
import type { LogEntry } from "./api";

const apiMock = vi.hoisted(() => ({
  importProject: vi.fn(),
  getSessionLog: vi.fn().mockResolvedValue([]),
  productLanguages: vi.fn().mockResolvedValue([]),
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
