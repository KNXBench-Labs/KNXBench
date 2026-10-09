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
