/** Exercises project-history consent, native/session disclosure and stale requests. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ProjectHistoryDialog from "./ProjectHistoryDialog";
import { admitProjectHistory, type ProjectHistory } from "./projectHistory";
import { HISTORY_FIXTURE } from "./projectHistory.fixture";
import { messages as en } from "./messages/en";

const api = vi.hoisted(() => ({ projectHistory: vi.fn(), createProjectVersion: vi.fn(), restoreProjectVersion: vi.fn(), deleteProjectVersion: vi.fn(), clearProjectUndo: vi.fn() }));
vi.mock("./api", () => api);
let root: Root | undefined;
let host: HTMLDivElement | undefined;
const changed = vi.fn();
const close = vi.fn();
function view(): ProjectHistory { return admitProjectHistory(structuredClone(HISTORY_FIXTURE)); }
function next(): ProjectHistory {
  const input = structuredClone(HISTORY_FIXTURE);
  input.generation++; input.snapshotRevision++; input.project.snapshot_revision++;
  return admitProjectHistory(input);
}

beforeEach(() => {
  vi.clearAllMocks();
  api.projectHistory.mockResolvedValue(view());
  api.createProjectVersion.mockResolvedValue(next());
  api.restoreProjectVersion.mockResolvedValue(next());
  api.deleteProjectVersion.mockResolvedValue(next());
  api.clearProjectUndo.mockResolvedValue(next());
});
afterEach(() => { act(() => root?.unmount()); host?.remove(); root = undefined; host = undefined; });
async function render(tree = view().project) {
  if (!host) { host = document.createElement("div"); document.body.append(host); root = createRoot(host); }
  await act(async () => { root!.render(<ProjectHistoryDialog tree={tree} onTreeUpdate={changed} onClose={close} />); });
  return document.querySelector(".project-history-panel") as HTMLElement;
}
function button(text: string): HTMLButtonElement {
  const found = [...document.querySelectorAll(".project-history-panel button")].find((b) => b.textContent === text);
  expect(found, `button ${text}`).toBeDefined(); return found as HTMLButtonElement;
}
async function click(text: string) { await act(async () => button(text).click()); }
async function label(text: string) {
  const input = document.querySelector<HTMLInputElement>("#project-version-label")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, text); input.dispatchEvent(new Event("input", { bubbles: true })); });
}

describe("ProjectHistoryDialog", () => {
  it("shows reported source-context and decoded budgets without guessing legacy limits", async () => {
    let dialog = await render();
    expect(dialog.querySelector('[data-history-limit="context"]')).toBeNull();
    expect(dialog.querySelector('[data-history-limit="decoded"]')).toBeNull();
    api.projectHistory.mockResolvedValue({...view(), limits: {...view().limits, maxContextBytes: 268435456, maxDecodedImageBytes: 536870912, maxDecodedHistoryBytes: 536870912}});
    await click(en["projectHistory.refresh"]);
    dialog = document.querySelector(".project-history-panel") as HTMLElement;
    expect(dialog.querySelector('[data-history-limit="context"]')?.textContent).toBe("Shared source-context limit: 256 MiB stored.");
    expect(dialog.querySelector('[data-history-limit="decoded"]')?.textContent).toBe("Decoded image limit: 512 MiB per image.");
    expect(dialog.querySelector('[data-history-limit="total-decoded"]')?.textContent).toBe("Total decoded history limit: 512 MiB; shared context is counted once.");
  });
  it("shows persistent recovery separately from Save and independent-backup scope", async () => {
    const dialog = await render();
    expect(dialog.textContent).toContain(en["projectHistory.native"]);
    expect(dialog.textContent).toContain(en["projectHistory.backupCaveat"]);
    expect(dialog.textContent).toContain("Before redesign");
    expect(dialog.querySelector("[data-history-stat=undo]")?.textContent).toContain("1");
  });
  it("discloses session-only history and disables named versions before Save As", async () => {
    api.projectHistory.mockResolvedValue(admitProjectHistory({ ...structuredClone(HISTORY_FIXTURE), persistence: "session", generation: 0, versions: [], totalBytes: 0 }));
    const dialog = await render();
    expect(dialog.textContent).toContain(en["projectHistory.session"]);
    expect(button(en["projectHistory.create"]).disabled).toBe(true);
  });
  it("creates a version with the exact label and publishes the returned project revision", async () => {
    await render(); await label("  My checkpoint  "); await click(en["projectHistory.create"]);
    expect(api.createProjectVersion).toHaveBeenCalledWith(view(), "  My checkpoint  ");
    expect(changed).toHaveBeenCalledWith(next().project, false);
    expect(close).not.toHaveBeenCalled();
  });
  it("opening or cancelling restore confirmation never sends a restore", async () => {
    await render(); await click(en["projectHistory.restore"]);
    expect(document.querySelector(".project-history-panel")?.textContent).toContain("Before redesign");
    expect(api.restoreProjectVersion).not.toHaveBeenCalled();
    expect(button(en["projectHistory.cancel"]) === document.activeElement).toBe(true);
    await click(en["projectHistory.cancel"]);
    expect(api.restoreProjectVersion).not.toHaveBeenCalled();
  });
  it("confirms exactly the selected version and the view that was reviewed", async () => {
    await render(); await click(en["projectHistory.restore"]); await click(en["projectHistory.confirmRestore"]);
    expect(api.restoreProjectVersion).toHaveBeenCalledWith(view(), 2, true);
    expect(changed).toHaveBeenCalledWith(next().project, true);
  });
  it("deleting a version and clearing undo each have their own explicit confirmation", async () => {
    await render(); await click(en["projectHistory.delete"]);
    expect(api.deleteProjectVersion).not.toHaveBeenCalled();
    await click(en["projectHistory.confirmDelete"]);
    expect(api.deleteProjectVersion).toHaveBeenCalledWith(view(), 2, true);
    await click(en["projectHistory.clearUndo"]);
    expect(api.clearProjectUndo).not.toHaveBeenCalled();
    await click(en["projectHistory.confirmClear"]);
    expect(api.clearProjectUndo).toHaveBeenCalledWith(next(), true);
  });
  it("does not publish success or close after a refused restore", async () => {
    api.restoreProjectVersion.mockRejectedValue(new Error("Stale editor witness"));
    await render(); await click(en["projectHistory.restore"]); await click(en["projectHistory.confirmRestore"]);
    expect(changed).not.toHaveBeenCalled(); expect(close).not.toHaveBeenCalled();
    expect(document.querySelector("[role=alert]")?.textContent).toContain("Stale editor witness");
    expect(button(en["projectHistory.restore"]).disabled).toBe(true);
  });
  it("does not present an unavailable read as an empty history", async () => {
    api.projectHistory.mockRejectedValue(new Error("Corrupt history witness"));
    const dialog = await render();
    expect(dialog.textContent).toContain(en["projectHistory.failed"]);
    expect(dialog.textContent).not.toContain(en["projectHistory.empty"]);
    expect(api.restoreProjectVersion).not.toHaveBeenCalled();
  });
  it("does not cancel an acknowledged restore when newer metadata arrives while it is pending", async () => {
    let resolve!: (value: ProjectHistory) => void;
    api.restoreProjectVersion.mockReturnValue(new Promise<ProjectHistory>((done) => { resolve = done; }));
    await render(); await click(en["projectHistory.restore"]); await click(en["projectHistory.confirmRestore"]);
    const newer = structuredClone(next().project); newer.snapshot_revision = (newer.snapshot_revision ?? 0) + 1;
    await render(newer);
    expect(api.projectHistory).toHaveBeenCalledTimes(1);
    await act(async () => resolve(next()));
    expect(changed).toHaveBeenCalledWith(next().project, true);
  });
  it("does not publish a pending restore response from a retired server lifetime", async () => {
    let resolve!: (value: ProjectHistory) => void;
    api.restoreProjectVersion.mockReturnValue(new Promise<ProjectHistory>((done) => { resolve = done; }));
    await render(); await click(en["projectHistory.restore"]); await click(en["projectHistory.confirmRestore"]);
    const other = structuredClone(view().project); other.server_incarnation = "retired-history-control";
    await render(other);
    await act(async () => resolve(next()));
    expect(changed).not.toHaveBeenCalled();
    expect(document.querySelector("[role=alert]")?.textContent).toContain("server lifetime");
  });
  it("invalidates an unconfirmed restore when the parent project revision changes", async () => {
    await render(); await click(en["projectHistory.restore"]);
    api.projectHistory.mockResolvedValue(next());
    await render(next().project);
    expect([...document.querySelectorAll("button")].some((b) => b.textContent === en["projectHistory.confirmRestore"])).toBe(false);
    expect(api.restoreProjectVersion).not.toHaveBeenCalled();
    expect(api.projectHistory).toHaveBeenCalledTimes(2);
  });
});

// WP-22: chronological checkpoint timeline over the existing stored versions.
const stamp = "2026-10-09T08:00:00.000Z";
function manyVersions(): ProjectHistory {
  const input = structuredClone(HISTORY_FIXTURE);
  const base = HISTORY_FIXTURE.versions[0];
  input.totalBytes = 400000; input.undoSteps = 3; input.redoSteps = 2; input.project.can_redo = true;
  // Newest-first, like the server; ids 5 and 3 share one instant.
  input.versions = [
    { ...base, id: 5, createdAt: stamp, reason: "save", label: "Saved five" },
    { ...base, id: 3, createdAt: stamp, reason: "named", label: "Alpha checkpoint" },
    { ...base, id: 2, createdAt: "2026-10-09T07:00:00.000Z", reason: "replaced_workspace", label: "Recovered" },
    { ...base, id: 7, createdAt: "2026-10-09T06:00:00.000Z", reason: "pre_restore", label: "Before restore" },
  ];
  input.project.can_undo = true;
  return admitProjectHistory(input);
}
const timeline = () => document.querySelector<HTMLOListElement>("ol.project-history-timeline")!;
const checkpoints = () => [...timeline().querySelectorAll<HTMLElement>("li[data-version-id]")];
const writes = () => api.createProjectVersion.mock.calls.length + api.restoreProjectVersion.mock.calls.length
  + api.deleteProjectVersion.mock.calls.length + api.clearProjectUndo.mock.calls.length;

describe("ProjectHistoryDialog timeline", () => {
  it("lists every stored version oldest-left to newest-right as a real ordered list with the current state last", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render();
    expect(timeline().tagName).toBe("OL");
    expect(checkpoints().map((li) => li.dataset.versionId)).toEqual(["7", "2", "3", "5"]);
    const first = checkpoints()[0];
    expect(first.querySelector("button")).not.toBeNull();
    expect(first.querySelector("time")?.getAttribute("datetime")).toBe("2026-10-09T06:00:00.000Z");
    expect(first.textContent).toContain(en["projectHistory.reason.pre_restore"]);
    const items = [...timeline().children];
    const current = items[items.length - 1] as HTMLElement;
    expect(current.dataset.timelineCurrent).toBeDefined();
    expect(current.querySelector("button")).toBeNull();
    expect(current.dataset.versionId).toBeUndefined();
    expect(current.textContent).toContain(en["projectHistory.timelineCurrent"]);
    expect(current.getAttribute("aria-current")).toBe("step");
    expect(checkpoints().some((li) => li.hasAttribute("aria-current"))).toBe(false);
    // undo is 3 and redo is 2; a swap or a lost count changes this exact text.
    expect(current.querySelector("span")?.textContent).toBe(en["projectHistory.timelineCurrentDetail"].replace("{undo}", "3").replace("{redo}", "2"));
    expect(current.querySelector("span")?.textContent).not.toBe(en["projectHistory.timelineCurrentDetail"].replace("{undo}", "2").replace("{redo}", "3"));
    expect(document.querySelector("[data-history-stat=undo] dd")?.textContent).toBe("3");
    expect(document.querySelector("[data-history-stat=redo] dd")?.textContent).toBe("2");
  });
  it("states plainly that the timeline shows retained versions, not every edit", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    const dialog = await render();
    expect(en["projectHistory.timelineNote"]).toMatch(/not every edit/);
    expect(dialog.querySelector(".project-history-timeline-note")?.textContent).toBe(en["projectHistory.timelineNote"]);
    expect(dialog.textContent).not.toMatch(/undefined|\{/);
  });
  it("selecting a checkpoint only selects: no write, restore still needs its confirmation", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render();
    const target = checkpoints().find((li) => li.dataset.versionId === "2")!.querySelector("button")!;
    await act(async () => target.click());
    expect(target.getAttribute("aria-pressed")).toBe("true");
    expect(checkpoints().filter((li) => li.querySelector("button")?.getAttribute("aria-pressed") === "true")).toHaveLength(1);
    expect(writes()).toBe(0);
    expect(document.querySelector(".project-history-selection")?.textContent).toContain(en["projectHistory.reason.replaced_workspace"]);
    await click(en["projectHistory.restore"]);
    expect(writes()).toBe(0);
    await click(en["projectHistory.confirmRestore"]);
    expect(api.restoreProjectVersion).toHaveBeenCalledTimes(1);
    expect(api.restoreProjectVersion).toHaveBeenCalledWith(manyVersions(), 2, true);
  });
  it("keeps the selection by version id while filtering and tells when it is hidden", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    const dialog = await render();
    await act(async () => checkpoints().find((li) => li.dataset.versionId === "2")!.querySelector("button")!.click());
    const search = dialog.querySelector<HTMLInputElement>("input[type=search]")!;
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "Alpha"); search.dispatchEvent(new Event("input", { bubbles: true })); });
    expect(checkpoints().map((li) => li.dataset.versionId)).toEqual(["3"]);
    expect(en["projectHistory.timelineHidden"]).toContain("{name}");
    expect(dialog.querySelector("[data-timeline-hidden-selection]")?.textContent).toBe(en["projectHistory.timelineHidden"].replace("{name}", en["projectHistory.reason.replaced_workspace"]));
    expect(dialog.querySelector(".project-history-selection")?.textContent).toContain(en["projectHistory.reason.replaced_workspace"]);
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, ""); search.dispatchEvent(new Event("input", { bubbles: true })); });
    expect(dialog.querySelector("[data-timeline-hidden-selection]")).toBeNull();
    expect(checkpoints().find((li) => li.dataset.versionId === "2")!.querySelector("button")!.getAttribute("aria-pressed")).toBe("true");
  });
  it("keeps the selected id across a refresh that reorders the versions and never mutates the API array", async () => {
    const first = manyVersions();
    const pristine = structuredClone(first.versions);
    api.projectHistory.mockResolvedValue(first);
    await render();
    await act(async () => checkpoints().find((li) => li.dataset.versionId === "3")!.querySelector("button")!.click());
    expect(first.versions).toEqual(pristine);
    const reordered = manyVersions(); reordered.versions.reverse();
    api.projectHistory.mockResolvedValue(reordered);
    await click(en["projectHistory.refresh"]);
    expect(checkpoints().map((li) => li.dataset.versionId)).toEqual(["7", "2", "3", "5"]);
    expect(checkpoints().find((li) => li.dataset.versionId === "3")!.querySelector("button")!.getAttribute("aria-pressed")).toBe("true");
  });
  it("shows only the current state for session-only history, with no selectable checkpoint", async () => {
    api.projectHistory.mockResolvedValue(admitProjectHistory({ ...structuredClone(HISTORY_FIXTURE), persistence: "session", generation: 0, versions: [], totalBytes: 0 }));
    const dialog = await render();
    expect(checkpoints()).toHaveLength(0);
    expect(timeline().querySelectorAll("button")).toHaveLength(0);
    expect(timeline().querySelector("[data-timeline-current]")).not.toBeNull();
    expect(dialog.textContent).toContain(en["projectHistory.session"]);
  });
  it("renders no timeline while loading or after a failed read", async () => {
    api.projectHistory.mockRejectedValue(new Error("Corrupt history witness"));
    await render();
    expect(document.querySelector("ol.project-history-timeline")).toBeNull();
  });
});

const setSearch = async (text: string) => {
  const search = document.querySelector<HTMLInputElement>("input[type=search]")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, text); search.dispatchEvent(new Event("input", { bubbles: true })); });
};
const pick = async (id: number) => act(async () => checkpoints().find((li) => li.dataset.versionId === String(id))!.querySelector("button")!.click());
const pressed = () => checkpoints().filter((li) => li.querySelector("button")?.getAttribute("aria-pressed") === "true").map((li) => li.dataset.versionId);
const selectionLine = () => document.querySelector(".project-history-selection")?.textContent;

describe("ProjectHistoryDialog hidden selection", () => {
  it("disables Restore and Delete while the filter hides the selected version, without switching or writing", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2);
    expect(button(en["projectHistory.restore"]).disabled).toBe(false);
    await setSearch("Alpha");
    expect(checkpoints().map((li) => li.dataset.versionId)).toEqual(["3"]);
    expect(button(en["projectHistory.restore"]).disabled).toBe(true);
    expect(button(en["projectHistory.delete"]).disabled).toBe(true);
    expect(button(en["projectHistory.clearUndo"]).disabled).toBe(false);
    expect(document.querySelector("[data-timeline-hidden-selection]")).not.toBeNull();
    await click(en["projectHistory.restore"]); await click(en["projectHistory.delete"]);
    expect(document.querySelector(".project-history-confirmation")).toBeNull();
    expect(pressed()).toEqual([]);
    expect(selectionLine()).toContain(en["projectHistory.reason.replaced_workspace"]);
    expect(writes()).toBe(0);
  });
  it("re-enables the actions for the same version when the filter clears, and the real confirmation still guards the write", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2); await setSearch("Alpha"); await setSearch("");
    expect(pressed()).toEqual(["2"]);
    expect(button(en["projectHistory.restore"]).disabled).toBe(false);
    await click(en["projectHistory.restore"]);
    expect(writes()).toBe(0);
    expect(document.querySelector(".project-history-confirmation")?.textContent).toContain(en["projectHistory.reason.replaced_workspace"]);
    await click(en["projectHistory.confirmRestore"]);
    expect(api.restoreProjectVersion).toHaveBeenCalledWith(manyVersions(), 2, true);
  });
  it("a visible pick while filtered enables the actions for that version, not the hidden one", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2); await setSearch("Alpha"); await pick(3);
    expect(pressed()).toEqual(["3"]);
    await click(en["projectHistory.delete"]);
    expect(document.querySelector(".project-history-confirmation")?.textContent).toContain("Alpha checkpoint");
    await click(en["projectHistory.confirmDelete"]);
    expect(api.deleteProjectVersion).toHaveBeenCalledWith(manyVersions(), 3, true);
  });
  it("keeps the existing fallback to the newest stored version when delete or refresh removes the selection", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2);
    const without = (id: number) => { const v = manyVersions(); v.versions = v.versions.filter((x) => x.id !== id); return v; };
    api.deleteProjectVersion.mockResolvedValue(without(2));
    await click(en["projectHistory.delete"]); await click(en["projectHistory.confirmDelete"]);
    expect(pressed()).toEqual(["5"]);
    await pick(3);
    api.projectHistory.mockResolvedValue(without(3));
    await click(en["projectHistory.refresh"]);
    expect(pressed()).toEqual(["5"]);
    expect(selectionLine()).toContain(en["projectHistory.reason.save"]);
  });
});

describe("ProjectHistoryDialog timeline gating", () => {
  const timelineButtons = () => [...timeline().querySelectorAll<HTMLButtonElement>("button")];
  async function expectInert() {
    expect(timelineButtons()).toHaveLength(4);
    expect(timelineButtons().every((b) => b.disabled)).toBe(true);
    const before = pressed();
    for (const b of timelineButtons()) await act(async () => b.click());
    expect(pressed()).toEqual(before);
  }
  it("is disabled and cannot change the selection while a refresh is loading", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2);
    let resolve!: (value: ProjectHistory) => void;
    api.projectHistory.mockReturnValue(new Promise<ProjectHistory>((done) => { resolve = done; }));
    await click(en["projectHistory.refresh"]);
    await expectInert();
    expect(pressed()).toEqual(["2"]);
    await act(async () => resolve(manyVersions()));
    expect(timelineButtons().every((b) => !b.disabled)).toBe(true);
  });
  it("is disabled and cannot change the selection after a refresh error keeps the old view", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2);
    api.projectHistory.mockRejectedValue(new Error("Refresh witness"));
    await click(en["projectHistory.refresh"]);
    expect(document.querySelector("[role=alert]")?.textContent).toContain("Refresh witness");
    await expectInert();
    expect(pressed()).toEqual(["2"]);
  });
  it("is disabled and cannot change the selection during an active mutation", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render(); await pick(2);
    let resolve!: (value: ProjectHistory) => void;
    api.createProjectVersion.mockReturnValue(new Promise<ProjectHistory>((done) => { resolve = done; }));
    await label("Pending checkpoint"); await click(en["projectHistory.create"]);
    expect(document.querySelector("[data-project-history-busy]")).not.toBeNull();
    await expectInert();
    expect(pressed()).toEqual(["2"]);
    await act(async () => resolve(manyVersions()));
    expect(document.querySelector("[data-project-history-busy]")).toBeNull();
    expect(timelineButtons().every((b) => !b.disabled)).toBe(true);
  });
});

describe("ProjectHistoryDialog alternate list", () => {
  it("keeps the plain list inside a details element that is collapsed by default and titled by the existing versions string", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render();
    const details = document.querySelector<HTMLDetailsElement>("details.project-history-alternate")!;
    expect(details.open).toBe(false);
    expect(details.querySelector(":scope > summary")?.textContent).toBe(en["projectHistory.versions"]);
    expect(details.querySelectorAll("ul.project-history-versions button")).toHaveLength(4);
    expect(timeline().closest("details")).toBeNull();
  });
  it("selects through the alternate list, and the timeline follows", async () => {
    api.projectHistory.mockResolvedValue(manyVersions());
    await render();
    const details = document.querySelector<HTMLDetailsElement>("details.project-history-alternate")!;
    await act(async () => { details.open = true; });
    const row = [...details.querySelectorAll<HTMLButtonElement>("ul.project-history-versions button")].find((b) => b.textContent!.includes("#2"))!;
    await act(async () => row.click());
    expect(row.getAttribute("aria-pressed")).toBe("true");
    expect(pressed()).toEqual(["2"]);
    expect(writes()).toBe(0);
  });
});

describe("ProjectHistoryDialog remount scroll", () => {
  it("reveals the selected checkpoint last after the confirmation is cancelled, not the newest end", async () => {
    const reveal = vi.fn(function (this: HTMLElement) { const scroller = this.closest<HTMLElement>(".project-history-timeline-scroll")!; scroller.scrollLeft = 100; });
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: reveal });
    Object.defineProperty(HTMLElement.prototype, "scrollWidth", { configurable: true, get() { return 1000; } });
    try {
      api.projectHistory.mockResolvedValue(manyVersions());
      await render(); await pick(7);
      await click(en["projectHistory.restore"]); await click(en["projectHistory.cancel"]);
      const scroller = document.querySelector<HTMLElement>(".project-history-timeline-scroll")!;
      expect(scroller.scrollLeft).toBe(100);
      expect(reveal).toHaveBeenCalled();
      expect(reveal.mock.contexts.at(-1)).toBe(timeline().querySelector('[data-version-id="7"] button'));
      expect(writes()).toBe(0);
    } finally {
      delete (HTMLElement.prototype as unknown as Record<string, unknown>).scrollIntoView;
      delete (HTMLElement.prototype as unknown as Record<string, unknown>).scrollWidth;
    }
  });
});
