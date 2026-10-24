/** Device communication table interaction and editor-lifetime contracts. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
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
  const editor = v.host.querySelector<HTMLSelectElement>('.dpt-field select')!;
  await v.set(editor, "DPST-9-2");
  await v.set(v.input("Search communication objects"), "Object 2");
  expect(v.host.textContent).toContain("Editing — outside filter");
  expect(v.host.querySelector(".com-table-count")?.textContent).toBe("1 / 2 objects");
  await v.click(v.button("By channel"));
  const c = { key: "opaque", kind: "Channel" as const, text: "Moved", name: null, number: null, order: 0 };
  await v.rerender([object(1, { dpt: "DPST-1-1", activation: "Active", channel: c }), object(2)]);
  expect(v.host.querySelector('.dpt-field select')).toBe(editor);
  expect(editor.value).toBe("DPST-9-2");
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it("keeps a dirty DPT draft when a refreshed committed DPT changes", async () => {
  const v = await render([object(1, { dpt: "DPST-1-1" })]);
  await v.click(v.button("All objects")); await v.click(v.host.querySelector<HTMLElement>(".com-object-detail summary")!);
  const input = v.host.querySelector<HTMLSelectElement>('.dpt-field select')!;
  await v.set(input, "DPST-9-2"); await v.rerender([object(1, { dpt: "DPST-5-1" })]);
  expect(input.value).toBe("DPST-9-2"); await v.cleanup();
});
it("invalidates a removed object instead of publishing its delayed result", async () => {
  let resolve!: (t: ProjectTree) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
  const v = await render([object(1)]); await v.click(v.button("All objects"));
  await v.click(v.host.querySelector<HTMLElement>(".com-object-detail summary")!);
  const input = v.host.querySelector<HTMLSelectElement>('.dpt-field select')!;
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
  const editor = v.host.querySelector<HTMLSelectElement>('.dpt-field select')!;
  await v.set(editor, "DPST-9-2");
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
  const editor = v.host.querySelector<HTMLSelectElement>('.dpt-field select')!;
  await v.set(editor, "DPST-1-1"); await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.rerender([object(1)], 10); await act(async () => resolve({ ...tree, is_modified: true }));
  expect(v.applied).not.toHaveBeenCalled(); expect(v.input("Search communication objects").value).toBe("");
  await v.cleanup();
});

// Issue #16: an explicit channel collapse hides every child row, open editors
// included; the editors stay mounted, so drafts and requests survive reopening.
const channelButton = (host: HTMLElement) => host.querySelector<HTMLButtonElement>(".com-object-channel-summary")!;
const summaryRow = (host: HTMLElement, id: number) => host.querySelector<HTMLTableRowElement>(`tr[data-object-id="${id}"]`)!;
const editorRow = (host: HTMLElement, id: number) => host.querySelector<HTMLTableRowElement>(`tr[data-editor-id="${id}"]`)!;
async function openGrouped(v: Awaited<ReturnType<typeof render>>, ids: number[]) {
  await v.click(channelButton(v.host));
  for (const id of ids) await v.click(summaryRow(v.host, id).querySelector<HTMLElement>("summary")!);
}
it("issue #16: collapsing a channel hides every child and its open editors, and reopening restores them", async () => {
  const v = await render([object(1, { dpt: "DPST-1-1" }), object(2), object(3)]);
  await openGrouped(v, [1, 2]);
  const draft = editorRow(v.host, 1).querySelector<HTMLSelectElement>(".dpt-field select")!;
  await v.set(draft, "DPST-9-2");
  await v.click(channelButton(v.host));
  expect(channelButton(v.host).getAttribute("aria-expanded")).toBe("false");
  for (const id of [1, 2, 3]) expect(summaryRow(v.host, id).hidden, `summary ${id}`).toBe(true);
  for (const id of [1, 2]) expect(editorRow(v.host, id).hidden, `editor ${id}`).toBe(true);
  // Hidden, not unmounted: the very same control still holds the draft.
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  await v.click(channelButton(v.host));
  for (const id of [1, 2, 3]) expect(summaryRow(v.host, id).hidden, `summary ${id}`).toBe(false);
  for (const id of [1, 2]) expect(editorRow(v.host, id).hidden, `editor ${id}`).toBe(false);
  expect(editorRow(v.host, 3).hidden).toBe(true);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  expect(draft.value).toBe("DPST-9-2");
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it("issue #16: an expanded channel still shows an open editor outside the filter; explicit collapse hides it", async () => {
  const v = await render([object(1), object(2)]);
  await openGrouped(v, [1]);
  await v.set(v.input("Search communication objects"), "Object 2");
  expect(channelButton(v.host).getAttribute("aria-expanded")).toBe("true");
  expect(summaryRow(v.host, 1).hidden).toBe(false);
  expect(editorRow(v.host, 1).hidden).toBe(false);
  expect(v.host.textContent).toContain("Editing — outside filter");
  await v.click(channelButton(v.host));
  expect(summaryRow(v.host, 1).hidden).toBe(true);
  expect(editorRow(v.host, 1).hidden).toBe(true);
  expect(summaryRow(v.host, 2).hidden).toBe(true);
  await v.click(channelButton(v.host));
  expect(summaryRow(v.host, 1).hidden).toBe(false);
  expect(editorRow(v.host, 1).hidden).toBe(false);
  await v.cleanup();
});
it.each(["succeeds", "is refused"])("issue #16: a request that %s while its channel is collapsed runs once and shows its outcome on reopening", async (outcome) => {
  let settle!: (value: unknown) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise((resolve, reject) => { settle = outcome === "succeeds" ? resolve : () => reject(new Error("refused by server")); }));
  const v = await render([object(1)]);
  await openGrouped(v, [1]);
  const editor = editorRow(v.host, 1).querySelector<HTMLSelectElement>(".dpt-field select")!;
  await v.set(editor, "DPST-1-1");
  await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.click(channelButton(v.host));
  expect(editorRow(v.host, 1).hidden).toBe(true);
  await act(async () => settle({ ...tree, is_modified: true }));
  await v.click(channelButton(v.host));
  expect(editorRow(v.host, 1).hidden).toBe(false);
  expect(api.setComObjectDpt).toHaveBeenCalledTimes(1);
  if (outcome === "succeeds") expect(v.applied).toHaveBeenCalledTimes(1);
  else { expect(editorRow(v.host, 1).textContent).toContain("refused by server"); expect(v.applied).not.toHaveBeenCalled(); }
  await v.cleanup();
});
it("#17: a filled product name and channel heading say where the text came from; others say nothing", async () => {
  const channel = { key: "/1", kind: "Channel", text: "Kitchen", name: "A", number: "1", order: 0,
    text_fill: { template: "{{0:Page 1}}", source: "StoredValue" as const } };
  const v = await render([
    object(1, { name: "Page 1 - Lamp", activation: "Active", channel,
      name_fill: { template: "Page 1 - {{0:Icon 1}}", source: "ProductDefault" } }),
    object(2, { name: "Dim {{0}}", activation: "Active", channel, name_fill: { template: "Dim {{0}}", source: "NoReference" } }),
    object(3, { name: "My {{0}} name", activation: "Active", channel }),
  ]);
  const name = (id: number) => v.host.querySelector<HTMLElement>(`tr[data-object-id="${id}"] .com-object-name`)!;
  expect(name(1).textContent).toBe("Page 1 - Lamp");
  expect(name(1).getAttribute("data-text-fill")).toBe("ProductDefault");
  expect(name(1).title).toBe("Filled in from the text parameter's default in the product.\nProduct text: Page 1 - {{0:Icon 1}}");
  // Not filled, visibly: the braces stay, and the tooltip says why.
  expect(name(2).textContent).toBe("Dim {{0}}");
  expect(name(2).title).toBe("Not filled in: the product names no text parameter for this text.\nProduct text: Dim {{0}}");
  // A user's name has no fill record and no tooltip.
  expect(name(3).hasAttribute("data-text-fill")).toBe(false);
  expect(name(3).title).toBe("");
  const heading = v.host.querySelector<HTMLElement>(".com-object-channel-summary strong")!;
  expect(heading.textContent).toBe("Kitchen");
  expect(heading.getAttribute("data-text-fill")).toBe("StoredValue");
  expect(heading.title).toBe("Filled in from this device's value of the text parameter.\nProduct text: {{0:Page 1}}");
  await v.cleanup();
});
it("names the object size the product data resolves, and nothing when it is unknown (census O3)", async () => {
  const v = await render([object(1, { size: "2 Bytes" }), object(2)]);
  const row = (id: number) => v.host.querySelector(`tr[data-object-id="${id}"]`)!;
  expect(row(1).querySelector(".com-object-size")?.textContent).toBe("Size: 2 Bytes");
  expect(row(2).querySelector(".com-object-size")).toBeNull();
  await v.cleanup();
});

it("WP-06a: shows a saved project folder by its declared ids and says why another is withheld", async () => {
  const v = await render([
    object(1, { project_folder: { status: "Assigned", key: "Channel:4:CH-1/Folder:4:PB-3",
      path: [{ kind: "Channel", ref_id: "CH-1" }, { kind: "Folder", ref_id: "PB-3" }] } }),
    object(2, { project_folder: { status: "ModuleObject", key: null, path: [] } }),
    object(3, { project_folder: { status: "NoFolder", key: null, path: [] } }),
    object(4)]);
  const note = (id: number) => v.host.querySelector<HTMLElement>(`tr[data-object-id="${id}"] .com-object-project-folder`);
  expect(note(1)?.textContent).toBe("Project folder: PB-3");
  expect(note(1)?.dataset.folderStatus).toBe("Assigned");
  expect(note(1)?.title).toBe("Saved project tree: CH-1 › PB-3. Ids as stored in the project; the product's folder names are not looked up.");
  expect(note(2)?.textContent).toBe("No project folder: folders of module objects are not supported yet");
  expect(note(3)).toBeNull();
  expect(note(4)).toBeNull();
  await v.set(v.input("Search communication objects"), "PB-3");
  const visible = [...v.host.querySelectorAll<HTMLElement>("tr.com-object-summary")].filter(r => !r.hidden).map(r => r.dataset.objectId);
  expect(visible).toEqual(["1"]);
  await v.cleanup();
});

tableColumnSmoke([["communication-objects", 6]], async () => { const v = await render([object(1)]); return v; });

tableColumnSmoke([["communication-objects", 7]], async () => {
  const v = await render([object(1)]); await v.click(v.button("All objects")); return v;
});

it.each(["By channel", "All objects"])("%s measures its DPT fit floor while preserving the stored minimum and rendered delta", async view => {
  const actEnvironment = globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean };
  const previousEnvironment = actEnvironment.IS_REACT_ACT_ENVIRONMENT;
  actEnvironment.IS_REACT_ACT_ENVIRONMENT = true;
  const pane = vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(300);
  window.localStorage.clear();
  const measurement = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.hasAttribute("data-table-word-measure") ? this.textContent!.length * 10 : 0 } as DOMRect;
  });
  const v = await render([object(1, { program_dpt: "DPST-9-1", dpt_text: "Windgeschwindigkeit" })]);
  try {
    await v.click(v.button(view));
    const dptColumn = () => v.host.querySelector<HTMLTableColElement>("colgroup col:nth-child(4)")!;
    const dptHandle = v.host.querySelector<HTMLElement>('th[data-column="dpt"] [role="separator"]')!;
    expect(parseFloat(dptColumn().style.width)).toBe(190);
    expect(parseFloat(v.host.querySelector<HTMLTableElement>(".com-table")!.style.width)).toBeGreaterThan(300);
    await act(async () => dptHandle.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })));
    expect(JSON.parse(window.localStorage.getItem("knxbench:table-column-widths:v1:communication-objects")!).dpt).toBe(100);
    expect(parseFloat(dptColumn().style.width)).toBe(190 + 100 - (view === "All objects" ? 114 : 136));
    await act(async () => dptHandle.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(parseFloat(dptColumn().style.width)).toBe(190);
  } finally {
    await v.cleanup();
    pane.mockRestore(); measurement.mockRestore();
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = previousEnvironment;
    window.localStorage.clear();
  }
});
