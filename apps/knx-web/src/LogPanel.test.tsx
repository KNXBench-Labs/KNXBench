// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { LogEntry } from "./api";
import { messages as enMessages } from "./messages/en";

const apiMock = vi.hoisted(() => ({
  getSessionLog: vi.fn().mockResolvedValue([]),
}));
const exportMock = vi.hoisted(() => ({ saveSessionLog: vi.fn().mockResolvedValue(true) }));

vi.mock("./sessionLogExport", async (importOriginal) => ({
  ...await importOriginal<typeof import("./sessionLogExport")>(),
  ...exportMock,
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
  exportMock.saveSessionLog.mockResolvedValue(true);
});

function baseTree(): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };
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

function setSearch(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

async function renderPanel(tree: ProjectTree, refreshKey = 0) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<LogPanel tree={tree} refreshKey={refreshKey} />);
  });
  return root;
}

describe("LogPanel", () => {
  it("renders known settings diagnostics from the catalogue instead of fallback prose", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({
        source: "settings",
        message: "UNRELATED SERVER FALLBACK",
        diagnostic: { kind: "migrated", fromVersion: 0, toVersion: 1 },
      }),
    ]);
    const root = await renderPanel(baseTree());
    expect(host!.textContent).toContain("Settings migrated from schema 0 to 1.");
    expect(host!.textContent).not.toContain("UNRELATED SERVER FALLBACK");
    root.unmount();
  });

  it("shows the empty state when there are no entries", async () => {
    const root = await renderPanel(baseTree());
    expect(host!.textContent).toContain("No log entries yet.");
    root.unmount();
  });

  it("shows the fix-round-2 (B4) English-text disclosure once at least one entry exists, and not before", async () => {
    apiMock.getSessionLog.mockResolvedValue([]);
    const root = await renderPanel(baseTree());
    expect(host!.textContent).not.toContain(enMessages["logPanel.entryTextIsEnglish"]);

    apiMock.getSessionLog.mockResolvedValue([entry({ message: "opened project" })]);
    const newTree = baseTree();
    await act(async () => {
      root.render(<LogPanel tree={newTree} refreshKey={0} />);
    });
    expect(host!.textContent).toContain(enMessages["logPanel.entryTextIsEnglish"]);

    root.unmount();
  });

  it("searches source, summary and detail case-insensitively, composes with severity, and clears without refetch", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ source: "IMPORT", severity: "warning", message: "missing product" }),
      entry({ source: "edit", severity: "error", message: "Bad ADDRESS" }),
      entry({ source: "save", severity: "info", detail: "address checked" }),
    ]);
    const root = await renderPanel(baseTree());
    const search = host!.querySelector<HTMLInputElement>('input[type="search"]')!;
    expect(search?.getAttribute("aria-label")).toBe("Search session log");
    await act(async () => { setSearch(search, "aDdReSs"); });
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(2);
    expect(host!.textContent).toContain("2 of 3 entries");
    const error = [...host!.querySelectorAll<HTMLInputElement>(".log-panel-filter input")].find((input) => input.closest("label")!.textContent === "Error")!;
    await act(async () => error.click());
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(1);
    expect(host!.textContent).toContain("1 of 3 entries");
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-clear")!.click());
    expect(search.value).toBe("");
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(2);
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("offers explicit all/filtered JSON export and retains a dropped marker outside the filtered view", async () => {
    const marker = entry({ severity: "warning", source: "log", message: "2 log entries dropped after exceeding the 1000-entry session log cap" });
    const matching = entry({ severity: "error", source: "import", message: "Bad address" });
    apiMock.getSessionLog.mockResolvedValue([marker, matching]);
    const root = await renderPanel(baseTree());
    const search = host!.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => { setSearch(search, "address"); });
    expect(host!.textContent).toContain("2 entries dropped");
    const scope = host!.querySelector<HTMLSelectElement>(".log-panel-export-scope")!;
    expect(scope?.value).toBe("filtered");
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenLastCalledWith([marker, matching], [matching], "filtered");
    await act(async () => { scope.value = "all"; scope.dispatchEvent(new Event("change", { bubbles: true })); });
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenLastCalledWith([marker, matching], [marker, matching], "all");
    root.unmount();
  });

  it("can export an empty log without suggesting it is a lifetime audit", async () => {
    const root = await renderPanel(baseTree());
    expect(host!.textContent).toContain("not a lifetime audit");
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenCalledWith([], [], "filtered");
    root.unmount();
  });

  it("shows a file-save failure without losing the log and clears it on retry", async () => {
    const item = entry({ message: "keep me" });
    apiMock.getSessionLog.mockResolvedValue([item]);
    exportMock.saveSessionLog.mockRejectedValueOnce(new Error("disk full"));
    const root = await renderPanel(baseTree());
    const button = host!.querySelector<HTMLButtonElement>(".log-panel-export")!;
    await act(async () => button.click());
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe("disk full");
    expect(host!.textContent).toContain("keep me");
    await act(async () => button.click());
    expect(host!.querySelector('[role="alert"]')).toBeNull();
    root.unmount();
  });

  it("never exports the previous project's entries during a refetch or after a failed refetch", async () => {
    const oldEntry = entry({ message: "previous project" });
    apiMock.getSessionLog.mockResolvedValueOnce([oldEntry]);
    let rejectRefetch!: (error: Error) => void;
    apiMock.getSessionLog.mockImplementationOnce(() => new Promise((_, reject) => { rejectRefetch = reject; }));
    const root = await renderPanel(baseTree());
    const button = host!.querySelector<HTMLButtonElement>(".log-panel-export")!;
    expect(button.disabled).toBe(false);
    await act(async () => root.render(<LogPanel tree={baseTree()} refreshKey={0} />));
    expect(button.disabled).toBe(true);
    expect(host!.textContent).not.toContain("previous project");
    await act(async () => button.click());
    expect(exportMock.saveSessionLog).not.toHaveBeenCalled();
    await act(async () => rejectRefetch(new Error("failed to refetch")));
    expect(button.disabled).toBe(true);
    expect(host!.textContent).not.toContain("previous project");
    expect(host!.querySelector(".field-error")!.textContent).toBe("failed to refetch");
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

  it("shows a distinct message when filters hide every entry (vs. a truly empty log)", async () => {
    apiMock.getSessionLog.mockResolvedValue([entry({ severity: "info", message: "ok" })]);
    const root = await renderPanel(baseTree());

    const checkboxes = Array.from(host!.querySelectorAll<HTMLInputElement>(".log-panel-filter input"));
    for (const cb of checkboxes) {
      await act(async () => {
        cb.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
    }

    expect(host!.textContent).not.toContain("No log entries yet.");
    expect(host!.textContent).toContain("No log entries match the current filters.");
    root.unmount();
  });

  it("renders a fetch error and clears it on a subsequent successful fetch", async () => {
    apiMock.getSessionLog.mockRejectedValueOnce(new Error("server unreachable"));
    const root = await renderPanel(baseTree());

    expect(host!.querySelector(".field-error")!.textContent).toBe("server unreachable");

    apiMock.getSessionLog.mockResolvedValue([]);
    const newTree = baseTree();
    await act(async () => {
      root.render(<LogPanel tree={newTree} refreshKey={0} />);
    });
    expect(host!.querySelector(".field-error")).toBeNull();

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
      root.render(<LogPanel tree={newTree} refreshKey={0} />);
    });
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(2);
    root.unmount();
  });

  it("refetches when refreshKey changes, even with the same tree (a failed operation never changes tree)", async () => {
    const tree = baseTree();
    const root = await renderPanel(tree, 0);
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.render(<LogPanel tree={tree} refreshKey={1} />);
    });
    expect(apiMock.getSessionLog).toHaveBeenCalledTimes(2);
    root.unmount();
  });
});
