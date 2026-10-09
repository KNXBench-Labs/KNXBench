/** Device communication table interaction and editor-lifetime contracts. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi, beforeEach } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ProjectTree } from "./bindings/ProjectTree";
function object(id: number, overrides: Partial<ComObjectNode> = {}): ComObjectNode {
  return { id, number: id, name: `Object ${id}`, function_text: null, description: null,
    description_layer: null, dpt: null, dpt_layer: null, program_dpt: null, dpt_text: null,
    is_active: true, activation: "NotEvaluated", channel: null, read: false, write: false,
    transmit: false, update: false, communication: true, read_on_init: false, links: [], ...overrides };
}
beforeEach(() => vi.clearAllMocks());
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }),
  setComObjectDpt: vi.fn(), setComObjectDescription: vi.fn(), setComObjectFlag: vi.fn(), unlinkComObject: vi.fn(), linkComObject: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./Inspector";
const tree: ProjectTree = { schema_version: 11, installations: [], group_address_style: "ThreeLevel", errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false };
function detail(objects: ComObjectNode[], id = 9): DeviceDetail { return { id, name: "Example", address: null, description: null, com_objects: objects,
  product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } }; }
async function render(objects: ComObjectNode[]) {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host); const applied = vi.fn();
  const rerender = async (objects: ComObjectNode[], id = 9) => { await act(async () => root.render(<DeviceWorkspace detail={detail(objects, id)} tree={tree} onApplied={applied} />)); };
  await rerender(objects);
  const button = (text: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find(b => b.textContent === text)!;
  const input = (label: string) => host.querySelector<HTMLInputElement>(`[aria-label="${label}"]`)!;
  const set = async (el: HTMLInputElement | HTMLSelectElement, value: string) => { await act(async () => {
    Object.getOwnPropertyDescriptor(el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype, "value")!.set!.call(el, value);
    el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
  }); };
  return { host, button, input, set, applied, rerender,
    click: async (el: HTMLElement) => { await act(async () => el.click()); },
    cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}
it("renders explicit headings and switches to a sortable flat table without writes", async () => {
  const v = await render([object(10), object(2)]);
  expect([...v.host.querySelectorAll("thead th")].map(n => n.textContent)).toEqual(["No.", "Name", "Function", "DPT", "Group addresses", "Status"]);
  await v.click(v.button("All objects"));
  expect(v.host.querySelector("thead")?.textContent).toContain("Channel");
  await v.click(v.host.querySelector<HTMLButtonElement>('th[data-column="number"] button')!);
  expect([...v.host.querySelectorAll("tr[data-object-id]")].map(n => n.getAttribute("data-object-id"))).toEqual(["2", "10"]);
  expect(v.host.querySelector('th[data-column="number"]')?.getAttribute("aria-sort")).toBe("ascending");
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it("reveals filtered channels and restores the unfiltered collapse state", async () => {
  const v = await render([object(1), object(2)]);
  const channel = () => v.host.querySelector<HTMLButtonElement>(".com-object-channel-summary")!;
  expect(channel().getAttribute("aria-expanded")).toBe("false");
  await v.set(v.input("Search communication objects"), "Object 1");
  expect(channel().getAttribute("aria-expanded")).toBe("true");
  expect(v.host.querySelector(".com-table-count")?.textContent).toBe("1 / 2 objects");
  await v.click(channel()); expect(channel().getAttribute("aria-expanded")).toBe("false");
  await v.click(v.button("Reset filters")); expect(channel().getAttribute("aria-expanded")).toBe("false");
  expect(v.host.querySelector(".com-table-count")?.textContent).toBe("2 / 2 objects");
  await v.set(v.input("Search communication objects"), "Object 1");
  expect(channel().getAttribute("aria-expanded")).toBe("true");
  await v.cleanup();
});
it("preserves the same open editor DOM across filtering, views, sorting and channel changes", async () => {
  const v = await render([object(1, { dpt: "DPST-1-1" }), object(2)]);
  await v.click(v.button("All objects"));
  await v.click(v.host.querySelector<HTMLDetailsElement>('[data-object-id="1"] summary')!);
  const editor = v.host.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')!;
  await v.set(editor, "DPST-9-2");
  await v.set(v.input("Search communication objects"), "Object 2");
  expect(v.host.textContent).toContain("Editing — outside filter");
  expect(v.host.querySelector(".com-table-count")?.textContent).toBe("1 / 2 objects");
  await v.click(v.button("By channel"));
  const c = { key: "opaque", kind: "Channel" as const, text: "Moved", name: null, number: null, order: 0 };
  await v.rerender([object(1, { dpt: "DPST-1-1", activation: "Active", channel: c }), object(2)]);
  expect(v.host.querySelector('input[placeholder="DPST-9-1"]')).toBe(editor);
  expect(editor.value).toBe("DPST-9-2");
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it("keeps a dirty DPT draft when a refreshed committed DPT changes", async () => {
  const v = await render([object(1, { dpt: "DPST-1-1" })]);
  await v.click(v.button("All objects")); await v.click(v.host.querySelector<HTMLElement>(".com-object-detail summary")!);
  const input = v.host.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')!;
  await v.set(input, "DPST-9-2"); await v.rerender([object(1, { dpt: "DPST-5-1" })]);
  expect(input.value).toBe("DPST-9-2"); await v.cleanup();
});
it("invalidates a removed object instead of publishing its delayed result", async () => {
  let resolve!: (t: ProjectTree) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
  const v = await render([object(1)]); await v.click(v.button("All objects"));
  await v.click(v.host.querySelector<HTMLElement>(".com-object-detail summary")!);
  const input = v.host.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')!;
  await v.set(input, "DPST-1-1"); await act(async () => input.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.rerender([]); await act(async () => resolve({ ...tree, is_modified: true }));
  expect(v.applied).not.toHaveBeenCalled(); expect(v.host.querySelector(".com-object-detail")).toBeNull();
  expect(v.host.querySelector(".com-object-removal")?.textContent).toContain("edited object is no longer present");
  await v.cleanup();
});
it("keeps pending/refused edits visible and blocks duplicate requests through presentation changes", async () => {
  let reject!: (e: Error) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise((_resolve, r) => { reject = r; }));
  const v = await render([object(1)]); await v.click(v.button("All objects"));
  await v.click(v.host.querySelector<HTMLElement>('[data-object-id="1"] summary')!);
  const editor = v.host.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')!;
  await v.set(editor, "bad");
  await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.set(v.input("Search communication objects"), "no match");
  await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  expect(api.setComObjectDpt).toHaveBeenCalledTimes(1);
  await act(async () => reject(new Error("refused")));
  expect(v.host.textContent).toContain("refused");
  expect(v.applied).not.toHaveBeenCalled(); await v.cleanup();
});
it("drops delayed editor results after switching device", async () => {
  let resolve!: (t: ProjectTree) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
  const v = await render([object(1)]); await v.click(v.button("All objects"));
  await v.click(v.host.querySelector<HTMLElement>('[data-object-id="1"] summary')!);
  const editor = v.host.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')!;
  await v.set(editor, "DPST-1-1"); await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.rerender([object(1)], 10); await act(async () => resolve({ ...tree, is_modified: true }));
  expect(v.applied).not.toHaveBeenCalled(); expect(v.input("Search communication objects").value).toBe("");
  await v.cleanup();
});
