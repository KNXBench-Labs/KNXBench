// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { LogEntry } from "./api";

const apiMock = vi.hoisted(() => ({
  getSessionLog: vi.fn().mockResolvedValue([]),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import LogPanel from "./LogPanel";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  apiMock.getSessionLog.mockResolvedValue([]);
});

function baseTree(): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, installations: [] };
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

async function renderPanel(tree: ProjectTree) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<LogPanel tree={tree} />);
  });
  return root;
}

describe("LogPanel", () => {
  it("shows the empty state when there are no entries", async () => {
    const root = await renderPanel(baseTree());
    expect(host!.textContent).toContain("No log entries yet.");
    root.unmount();
  });

  it("renders entries newest-first", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ timestamp: "2026-09-10T12:00:00Z", message: "first" }),
      entry({ timestamp: "2026-09-10T12:05:00Z", message: "second" }),
    ]);
    const root = await renderPanel(baseTree());

    const messages = Array.from(host!.querySelectorAll(".log-entry-message")).map((el) => el.textContent);
    expect(messages).toEqual(["second", "first"]);
    root.unmount();
  });

  it("hides entries via severity filters without refetching", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ timestamp: "2026-09-10T12:00:00Z", severity: "error", message: "boom" }),
      entry({ timestamp: "2026-09-10T12:01:00Z", severity: "warning", message: "hmm" }),
      entry({ timestamp: "2026-09-10T12:02:00Z", severity: "info", message: "ok" }),
    ]);
    const root = await renderPanel(baseTree());

    expect(host!.textContent).toContain("boom");
    expect(host!.textContent).toContain("hmm");
    expect(host!.textContent).toContain("ok");
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    const checkboxes = Array.from(host!.querySelectorAll<HTMLInputElement>(".log-panel-filter input"));
    const errorCheckbox = checkboxes.find((cb) => cb.closest(".log-panel-filter")!.textContent === "Error")!;
    await act(async () => {
      errorCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(host!.textContent).not.toContain("boom");
    expect(host!.textContent).toContain("hmm");
    expect(host!.textContent).toContain("ok");
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    const warningCheckbox = checkboxes.find((cb) => cb.closest(".log-panel-filter")!.textContent === "Warning")!;
    await act(async () => {
      warningCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.textContent).not.toContain("hmm");
    expect(host!.textContent).toContain("ok");
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    const infoCheckbox = checkboxes.find((cb) => cb.closest(".log-panel-filter")!.textContent === "Info")!;
    await act(async () => {
      infoCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.textContent).not.toContain("ok");
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("shows location and detail when present", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ location: "/KNX/Project/Foo", detail: "extra info" }),
    ]);
    const root = await renderPanel(baseTree());
    expect(host!.textContent).toContain("/KNX/Project/Foo");
    expect(host!.textContent).toContain("extra info");
    root.unmount();
  });

  it("refetches when the tree prop changes", async () => {
    const root = await renderPanel(baseTree());
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    const newTree = baseTree();
    await act(async () => {
      root.render(<LogPanel tree={newTree} />);
    });
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(2);
    root.unmount();
  });
});
