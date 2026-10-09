/** Name drafts survive refusals without leaking across asynchronous identity changes. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import { renameTree } from "./rename.fixture";
import RenameNameField from "./RenameNameField";
const apiMock = vi.hoisted(() => ({ renameEntity: vi.fn(), currentProject: vi.fn(), deviceCatalog: vi.fn(), errorMessage: (e: unknown) => e instanceof Error ? e.message : String(e) }));
vi.mock("./api", () => apiMock);
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root; let host: HTMLDivElement;
afterEach(async () => { if (root) await act(async () => root.unmount()); host?.remove(); vi.resetAllMocks(); });
async function setup() {
  const applied = vi.fn(); const tree = renameTree();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  async function render(next = tree, id = 2) {
    await act(async () => root.render(<RenameNameField tree={next} target={{ kind: "group_address", id,
      name: next.installations[id - 1].group_addresses[0].name }} onApplied={applied} />));
  }
  await render(); return { applied, tree, render };
}
async function type(value: string) {
  const input = host.querySelector<HTMLInputElement>('input[data-rename-name]')!;
  await act(async () => {
    input.focus(); Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }); return input;
}
async function key(input: HTMLInputElement, key: string) {
  await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })));
}
it("Enter commits exact text once, and subsequent blur cannot duplicate it", async () => {
  const { applied, tree } = await setup(); const updated = { ...tree, snapshot_revision: 2 };
  apiMock.renameEntity.mockResolvedValue(updated);
  const input = await type("  Küche 🛠  "); await key(input, "Enter");
  await act(async () => input.blur());
  expect(apiMock.renameEntity).toHaveBeenCalledExactlyOnceWith("group_address", 2, "  Küche 🛠  ", tree, "Original");
  expect(applied).toHaveBeenCalledExactlyOnceWith(updated);
});
it("rejects multiline paste visibly rather than letting a text input silently strip it", async () => {
  await setup(); const input = await type("Existing draft");
  const paste = new Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(paste, "clipboardData", { value: { getData: () => "New\nname" } });
  await act(async () => input.dispatchEvent(paste));
  expect(paste.defaultPrevented).toBe(true); expect(input.value).toBe("Existing draft");
  expect(host.querySelector('[role="alert"]')?.textContent).toContain("Control characters");
  expect(apiMock.renameEntity).not.toHaveBeenCalled();
});
it("Escape followed by blur never commits a cancelled draft", async () => {
  await setup(); const input = await type("Cancel me"); await key(input, "Escape");
  await act(async () => input.blur()); expect(apiMock.renameEntity).not.toHaveBeenCalled();
  expect(input.value).toBe("Original");
});
it("refusal retains the draft and refresh is read-only before explicit reapplication", async () => {
  const { applied, tree } = await setup(); apiMock.renameEntity.mockRejectedValue(new Error("Conflict"));
  const input = await type("Keep draft"); await key(input, "Enter");
  expect(input.value).toBe("Keep draft"); expect(host.querySelector('[role="alert"]')?.textContent).toBe("Conflict");
  expect(applied).not.toHaveBeenCalled();
  apiMock.currentProject.mockResolvedValue({ ...tree, snapshot_revision: 2 });
  const refresh = [...host.querySelectorAll("button")].find((b) => b.textContent === "Refresh state")!;
  await act(async () => refresh.click());
  expect(apiMock.renameEntity).toHaveBeenCalledTimes(1); expect(input.value).toBe("Keep draft");
  apiMock.renameEntity.mockResolvedValue({ ...tree, snapshot_revision: 3 }); await key(input, "Enter");
  expect(apiMock.renameEntity).toHaveBeenLastCalledWith("group_address", 2, "Keep draft", expect.objectContaining({ snapshot_revision: 2 }), "Original");
});
it("refresh refuses a different project incarnation even when entity IDs and names match", async () => {
  const { applied, tree } = await setup(); apiMock.renameEntity.mockRejectedValue(new Error("Lost response"));
  const input = await type("Keep draft"); await key(input, "Enter");
  apiMock.currentProject.mockResolvedValue({ ...tree, snapshot_revision: 2, project_incarnation: 1 });
  await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "Refresh state")!.click());
  expect(applied).not.toHaveBeenCalled(); expect(input.value).toBe("Keep draft");
  expect(host.querySelector('[role="alert"]')?.textContent).toContain("Project context changed");
});
it("a new target is not left busy by an old pending request and its late result is ignored", async () => {
  const { applied, tree, render } = await setup(); let resolve!: (tree: ProjectTree) => void;
  apiMock.renameEntity.mockImplementation(() => new Promise<ProjectTree>((done) => { resolve = done; }));
  const input = await type("Pending"); await key(input, "Enter"); expect(input.disabled).toBe(true);
  await render(tree, 1);
  const next = host.querySelector<HTMLInputElement>('input[data-rename-name]')!;
  expect(next.disabled).toBe(false); expect(next.value).toBe("Original");
  await act(async () => resolve({ ...tree, snapshot_revision: 2 })); expect(applied).not.toHaveBeenCalled();
});
it("returning to the same target does not accept the earlier editor generation's reply", async () => {
  const { applied, tree, render } = await setup(); let resolve!: (tree: ProjectTree) => void;
  apiMock.renameEntity.mockImplementation(() => new Promise<ProjectTree>((done) => { resolve = done; }));
  const input = await type("Old generation"); await key(input, "Enter");
  await render(tree, 1); await render(tree, 2);
  await act(async () => resolve({ ...tree, snapshot_revision: 2 }));
  expect(applied).not.toHaveBeenCalled();
});
it("a lost response for an unplaced canonical device can be reconciled without a duplicate write", async () => {
  const { tree, applied } = await setup();
  await act(async () => root.render(<RenameNameField tree={tree} target={{ kind: "device", id: 99, name: "Original" }} onApplied={applied} />));
  apiMock.renameEntity.mockRejectedValue(new Error("Lost response"));
  const input = await type("Already committed"); await key(input, "Enter");
  apiMock.currentProject.mockResolvedValue({ ...tree, snapshot_revision: 2 });
  apiMock.deviceCatalog.mockResolvedValue({ serverIncarnation: "rename", snapshotRevision: 2,
    devices: [{ id: 99, device: { id: 99, name: "Already committed" } }] });
  await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "Refresh state")!.click());
  expect(applied).toHaveBeenCalledTimes(1); expect(input.value).toBe("Already committed");
  await key(input, "Enter"); expect(apiMock.renameEntity).toHaveBeenCalledTimes(1);
  expect(host.querySelector('[role="alert"]')).toBeNull();
});
it("an unrelated refreshed snapshot does not erase a typed draft or rebase it automatically", async () => {
  const { tree, render } = await setup(); await type("My draft");
  await render({ ...tree, snapshot_revision: 7 });
  apiMock.renameEntity.mockRejectedValue(new Error("Stale"));
  await key(host.querySelector('input')!, "Enter");
  expect(apiMock.renameEntity).toHaveBeenCalledWith("group_address", 2, "My draft", tree, "Original");
});
it.each(["", "   ", "x\u0001name", "🛠".repeat(1025)])("invalid drafts are retained and never sent (%s)", async (draft) => {
  await setup(); const input = await type(draft); await key(input, "Enter");
  expect(apiMock.renameEntity).not.toHaveBeenCalled(); expect(input.value).toBe(draft);
  expect(host.querySelector('[role="alert"]')).not.toBeNull();
});
