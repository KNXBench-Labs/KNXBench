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
  vi.unstubAllGlobals();
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
  it("renders 16000 entries a page at a time, searches all of them and keeps exports whole", async () => {
    const server = Array.from({ length: 16000 }, (_, index) => entry({ message: `server ${index}`, timestamp: new Date(Date.UTC(2026, 9, 13, 0, 0, index)).toISOString() }));
    apiMock.getSessionLog.mockResolvedValue(server);
    const root = await renderPanel(baseTree());
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(500);
    expect(host!.querySelector(".log-entry-message")!.textContent).toBe("server 15999");
    expect(host!.querySelector(".log-panel-more")!.textContent).toContain("Showing the newest 500 of 16000 matching entries.");
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-show-more")!.click());
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(1000);
    // A refetch after a project change keeps the expanded list.
    const newer = [...server, entry({ message: "server 16000", timestamp: new Date(Date.UTC(2026, 9, 13, 5, 0, 0)).toISOString() })];
    apiMock.getSessionLog.mockResolvedValue(newer);
    await act(async () => root.render(<LogPanel tree={baseTree()} refreshKey={1} />));
    expect(host!.querySelector(".log-entry-message")!.textContent).toBe("server 16000");
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(1000);
    apiMock.getSessionLog.mockResolvedValue(server);
    await act(async () => root.render(<LogPanel tree={baseTree()} refreshKey={2} />));
    // Search spans every entry, not only the rendered page.
    await act(() => setSearch(host!.querySelector<HTMLInputElement>(".log-panel-search input")!, "server 7"));
    expect(host!.querySelector(".log-panel-count")!.textContent).toBe("1111 of 16000 entries");
    expect(host!.querySelectorAll(".log-entry")).toHaveLength(500);
    await act(() => setSearch(host!.querySelector<HTMLInputElement>(".log-panel-search input")!, "server 1599"));
    expect(host!.querySelector(".log-panel-count")!.textContent).toBe("11 of 16000 entries");
    expect(host!.querySelector(".log-panel-more")).toBeNull();
    const scope = host!.querySelector<HTMLSelectElement>(".log-panel-export-scope")!;
    await act(() => { scope.value = "all"; scope.dispatchEvent(new Event("change", { bubbles: true })); });
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenLastCalledWith(server, server, "all");
    await act(() => root.unmount());
  });

  it("previews a grouped file list in the row and keeps every line for the details", async () => {
    const paths = Array.from({ length: 1400 }, (_, n) => `M-0001/Baggages/${n}.png`);
    const grouped = entry({ severity: "warning", source: "import:unsupported", message: "1400 retained files: manufacturer payload (png)", detail: paths.join("\n") });
    apiMock.getSessionLog.mockResolvedValue([grouped]);
    const root = await renderPanel(baseTree());
    const detail = host!.querySelector(".log-entry-detail")!;
    expect(detail.textContent).toBe(`${paths.slice(0, 3).join("\n")}… 1397 more lines in the details`);
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-entry-open")!.click());
    const dialogText = [...document.querySelectorAll(".log-detail-panel pre")].map(el => el.textContent).join("\n");
    expect(dialogText).toContain(paths[1399]);
    await act(() => root.unmount());
  });

  it("keeps the full backend v1 export at 1000 records despite a separate 1000 UI records", async () => {
    const server = Array.from({ length: 1000 }, (_, index) => entry({ message: `server ${index}` }));
    const notifications = Array.from({ length: 1000 }, (_, id) => ({ id, entry: entry({ source: "ui:toast:error", message: `browser ${id}` }) }));
    apiMock.getSessionLog.mockResolvedValue(server);
    const tree = baseTree(); const root = await renderPanel(tree);
    await act(() => root.render(<LogPanel tree={tree} refreshKey={0} notifications={notifications} notificationDroppedCount={7} />));
    const scope = host!.querySelector<HTMLSelectElement>(".log-panel-export-scope")!;
    await act(() => { scope.value = "all"; scope.dispatchEvent(new Event("change", { bubbles: true })); });
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenLastCalledWith(server, server, "all");
    expect(host!.textContent).toContain("JSON export contains only server entries");
    expect(host!.textContent).toContain("7 UI notifications dropped");
    await act(() => root.unmount());
  });
  it("exports no fake server rows when the search matches only a client notification", async () => {
    const raw = entry({ message: "actual server entry" });
    apiMock.getSessionLog.mockResolvedValue([raw]);
    const tree = baseTree(); const root = await renderPanel(tree);
    await act(() => root.render(<LogPanel tree={tree} refreshKey={0} notifications={[{ id: 1, entry: entry({ source: "ui:toast:error", message: "browser-only search" }) }]} />));
    await act(() => setSearch(host!.querySelector<HTMLInputElement>(".log-panel-search input")!, "browser-only search"));
    await act(async () => host!.querySelector<HTMLButtonElement>(".log-panel-export")!.click());
    expect(exportMock.saveSessionLog).toHaveBeenLastCalledWith([raw], [], "filtered");
    await act(() => root.unmount());
  });
  it("opens a backend entry and exposes exact multiline raw text for selection and copy", async () => {
    const raw = entry({ message: "raw message\nsecond line", detail: "detail\n<not HTML>", location: "1.1.1", diagnostic: { kind: "migrated", fromVersion: 0, toVersion: 1 } });
    apiMock.getSessionLog.mockResolvedValue([raw]);
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const root = await renderPanel(baseTree());
    const open = host!.querySelector<HTMLButtonElement>(".log-entry-open");
    expect(open).not.toBeNull();
    await act(() => open!.click());
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    expect(dialog).not.toBeNull();
    const text = dialog.querySelector<HTMLTextAreaElement>("textarea")!;
    expect(JSON.parse(text.value)).toEqual(raw);
    expect(text.readOnly).toBe(true);
    await act(async () => dialog.querySelector<HTMLButtonElement>(".log-detail-copy-message")!.click());
    expect(writeText).toHaveBeenLastCalledWith(raw.message);
    await act(async () => dialog.querySelector<HTMLButtonElement>(".log-detail-copy-all")!.click());
    expect(JSON.parse(writeText.mock.lastCall![0])).toEqual(raw);
    await act(() => root.unmount()); vi.unstubAllGlobals();
  });
  it("falls back to selectable exact JSON when clipboard permission is refused", async () => {
    const raw = entry({ message: "same\nraw", detail: "\tUnicode ü <xml>" });
    apiMock.getSessionLog.mockResolvedValue([raw]);
    vi.stubGlobal("navigator", { clipboard: { writeText: vi.fn().mockRejectedValue(new Error("permission denied")) } });
    const root = await renderPanel(baseTree());
    await act(() => host!.querySelector<HTMLButtonElement>(".log-entry-open")!.click());
    await act(async () => document.querySelector<HTMLButtonElement>(".log-detail-copy-all")!.click());
    const text = document.querySelector<HTMLTextAreaElement>(".log-detail-raw textarea")!;
    expect(document.activeElement).toBe(text);
    expect(text.selectionStart).toBe(0); expect(text.selectionEnd).toBe(text.value.length);
    expect(JSON.parse(text.value)).toEqual(raw);
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("copy it manually");
    await act(() => root.unmount());
  });
  it("reports an evicted target without opening a same-text replacement", async () => {
    const root = await renderPanel(baseTree());
    await act(() => root.render(<LogPanel tree={baseTree()} refreshKey={0} notificationTarget={{ id: 7 }} notifications={[{ id: 8, entry: entry({ message: "same" }) }]} />));
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(host!.textContent).toContain("No other message was selected");
    await act(() => root.unmount());
  });
  it("jumps to an exact UI notification even with duplicate text and restores filters", async () => {
    apiMock.getSessionLog.mockResolvedValue([]);
    const notifications = [4, 5].map(id => ({ id, entry: entry({ source: "ui:toast:error", severity: "error", message: "same", detail: `detail ${id}` }) }));
    const tree = baseTree();
    const root = await renderPanel(tree);
    await act(() => host!.querySelector<HTMLInputElement>('.log-panel-filter input')!.click());
    await act(() => setSearch(host!.querySelector<HTMLInputElement>('input[type="search"]')!, "hidden"));
    await act(() => root.render(<LogPanel tree={tree} refreshKey={0} notifications={notifications} notificationTarget={{ id: 4 }} notificationDroppedCount={2} />));
    expect(host!.querySelector('.log-entry--selected')?.textContent).toContain("detail 4");
    const text = document.querySelector<HTMLTextAreaElement>('[role="dialog"] textarea')!;
    expect(JSON.parse(text.value).detail).toBe("detail 4");
    expect(host!.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("");
    expect(host!.textContent).toContain("2 UI notifications dropped");
    await act(() => root.unmount());
  });
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

  it("keeps source-observation meaning and manufacturer errors visible", async () => {
    apiMock.getSessionLog.mockResolvedValue([
      entry({ source: "import:source-observations", message: "XML element observations include retained subtrees; not semantic acceptance", detail: '{"topology":{"DeviceInstance":3}}' }),
      entry({ source: "import:manufacturer", severity: "error", message: "IdConflict: hardware/synthetic (2 occurrences); original source retained", location: "M-0001/Hardware.xml" }),
    ]);
    const root = await renderPanel({ ...baseTree(), schema_version: 12 });
    expect(host!.textContent).toContain("not semantic acceptance");
    expect(host!.textContent).toContain("IdConflict: hardware/synthetic");
    expect(host!.textContent).toContain("M-0001/Hardware.xml");
    expect(host!.textContent).toContain("DeviceInstance");
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


it("labels raw RTF-like previews without interpreting control words or HTML", async () => {
  const raw = String.raw`{\rtf1\ansi <b>literal</b> \u20320?}`;
  apiMock.getSessionLog.mockResolvedValue([entry({ source: "import:opaque:raw-text", message: "RTF-like raw text candidate; not validated or interpreted; original retained", detail: raw })]);
  const root = await renderPanel(baseTree());
  expect(host?.textContent).toContain("RTF-like raw text — not interpreted");
  expect(host?.querySelector(".log-entry-raw-text")?.textContent).toBe(raw);
  expect(host?.querySelector(".log-entry-raw-text b")).toBeNull();
  expect(host?.querySelector(".log-entry-raw-text input, .log-entry-raw-text textarea")).toBeNull();
  await act(async () => root.unmount());
});

describe("LogPanel — import findings opened from the project status", () => {
  const importLog = () => [
    entry({ severity: "warning", source: "import:unknown", message: "unknown element A" }),
    entry({ severity: "warning", source: "import:conflict", message: "conflict B" }),
    entry({ severity: "error", source: "import:parse", message: "lost C" }),
    entry({ severity: "info", source: "import:opaque", message: "opaque D" }),
    entry({ severity: "warning", source: "csv-import:problem", message: "csv E" }),
    entry({ severity: "info", source: "import", message: "imported F" }),
  ];
  const listed = () => [...host!.querySelectorAll(".log-entry-message")].map((n) => n.textContent);
  const note = () => host!.querySelector<HTMLElement>(".log-panel-import-scope");

  async function focusOn(severity: "error" | "warning", expected: number, onHandled = vi.fn()) {
    apiMock.getSessionLog.mockResolvedValue(importLog());
    const tree = { ...baseTree(), [severity === "error" ? "errors" : "warnings"]: expected };
    const root = await renderPanel(tree);
    await act(async () => root.render(<LogPanel tree={tree} refreshKey={0} importFocus={{ severity }} onImportFocusHandled={onHandled} />));
    return root;
  }

  it("lists exactly the counted import warnings and says they match", async () => {
    const handled = vi.fn();
    const root = await focusOn("warning", 2, handled);
    expect(handled).toHaveBeenCalledTimes(1);
    expect(listed().sort()).toEqual(["conflict B", "unknown element A"]);
    expect(note()?.getAttribute("role")).toBe("region");
    expect(note()?.textContent).toContain(enMessages["logPanel.importScope.title"]);
    expect(note()?.querySelector('[role="status"]')?.textContent).toBe("Showing 2 import warnings, as counted in the project status.");
    expect(document.activeElement).toBe(note());
    await act(async () => root.unmount());
  });

  it("lists import errors for the error counter", async () => {
    const root = await focusOn("error", 1);
    expect(listed()).toEqual(["lost C"]);
    expect(note()?.querySelector('[role="status"]')?.textContent).toBe("Showing 1 import error, as counted in the project status.");
    await act(async () => root.unmount());
  });

  it("says so when the log holds fewer findings than counted", async () => {
    const root = await focusOn("warning", 5);
    expect(note()?.querySelector('[role="status"]')?.textContent)
      .toBe("The project status counts 5 import warnings; this session log holds 2 matching entries.");
    await act(async () => root.unmount());
  });

  it("says so when the log holds none of the counted findings", async () => {
    apiMock.getSessionLog.mockResolvedValue([entry({ source: "open", message: "opened project" })]);
    const tree = { ...baseTree(), warnings: 3 };
    const root = await renderPanel(tree);
    await act(async () => root.render(<LogPanel tree={tree} refreshKey={0} importFocus={{ severity: "warning" }} />));
    expect(note()?.querySelector('[role="status"]')?.textContent)
      .toBe("The project status counts 3 import warnings, but the session log of this server run holds none of them.");
    expect(listed()).toEqual([]);
    await act(async () => root.unmount());
  });

  it("compares against the project on screen, not the one that was clicked", async () => {
    const root = await focusOn("warning", 2);
    // Another project replaces the first while the scoped log stays open.
    apiMock.getSessionLog.mockResolvedValue([entry({ severity: "warning", source: "import:unknown", message: "new project warning" })]);
    await act(async () => root.render(<LogPanel tree={{ ...baseTree(), warnings: 1 }} refreshKey={0} />));
    expect(listed()).toEqual(["new project warning"]);
    expect(note()?.querySelector('[role="status"]')?.textContent).toBe("Showing 1 import warning, as counted in the project status.");
    await act(async () => root.unmount());
  });

  it("returns to every entry and severity on request", async () => {
    const root = await focusOn("warning", 2);
    const showAll = [...note()!.querySelectorAll("button")].find((b) => b.textContent === enMessages["logPanel.importScope.showAll"])!;
    await act(async () => showAll.click());
    expect(note()).toBeNull();
    expect(listed()).toHaveLength(6);
    expect([...host!.querySelectorAll<HTMLInputElement>(".log-panel-filter input")].map((i) => i.checked)).toEqual([true, true, true]);
    await act(async () => root.unmount());
  });
});
