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

// WP-06 folder grouping: a third, read-only view keyed by the opaque saved-folder key.
type Folder = NonNullable<ComObjectNode["project_folder"]>;
const pb3 = (ch: string): Folder => ({ status: "Assigned", key: `Channel:4:${ch}/Folder:4:PB-3`,
  path: [{ kind: "Channel", ref_id: ch }, { kind: "Folder", ref_id: "PB-3" }] });
const VIEWS = ["By channel", "By saved project folder", "All objects"];
async function show(v: Awaited<ReturnType<typeof render>>, name: string) {
  const button = v.button(name);
  expect(button, `view button ${name}`).toBeDefined();
  await v.click(button);
}
const groupHeader = (host: HTMLElement, key: string) => host.querySelector<HTMLTableRowElement>(`tr.com-object-channel[data-group-key="${key}"]`);
const groupButton = (host: HTMLElement, key: string) => groupHeader(host, key)?.querySelector<HTMLButtonElement>(".com-object-channel-summary") ?? null;
const F1 = "folder:Channel:4:CH-1/Folder:4:PB-3", F2 = "folder:Channel:4:CH-2/Folder:4:PB-3";
it("WP-06 folder view: three exclusive views, six columns, disambiguated headings and group keys", async () => {
  const c = { key: "c-1", kind: "Channel" as const, text: "Kitchen", name: null, number: null, order: 0 };
  const v = await render([object(1, { activation: "Active", channel: c, project_folder: pb3("CH-1") }),
    object(2, { project_folder: pb3("CH-2") }), object(3, { project_folder: { status: "ModuleObject", key: null, path: [] } }), object(4)]);
  // Channel view: unchanged default, its headers name their group key too.
  expect(VIEWS.map(name => v.button(name)?.getAttribute("aria-pressed"))).toEqual(["true", "false", "false"]);
  expect([...v.host.querySelectorAll<HTMLElement>("tr.com-object-channel")].map(r => [r.dataset.groupKey, r.dataset.channelKey]))
    .toEqual([["channel:c-1", "channel:c-1"], ["unassigned", "unassigned"]]);
  expect(summaryRow(v.host, 1).dataset.groupKey).toBe("channel:c-1");
  expect(v.host.querySelector(".com-folder-view-note")).toBeNull();
  await show(v, "By saved project folder");
  expect(VIEWS.map(name => v.button(name)?.getAttribute("aria-pressed"))).toEqual(["false", "true", "false"]);
  expect([...v.host.querySelectorAll("thead th")].map(n => n.textContent)).toEqual(["No.", "Name", "Function", "DPT", "Group addresses", "Status"]);
  const headers = [...v.host.querySelectorAll<HTMLElement>("tr.com-object-channel")];
  expect(headers.map(r => r.dataset.groupKey)).toEqual([F1, F2, "folder-none"]);
  // Folder headings never pretend to be evaluated channels.
  expect(headers.map(r => r.hasAttribute("data-channel-key"))).toEqual([false, false, false]);
  expect(headers.map(r => r.querySelector("strong")?.textContent)).toEqual(["PB-3", "PB-3", "Without saved project folder"]);
  // Equal Folder ids stay distinguishable by the always-visible full path.
  expect(headers.map(r => r.querySelector(".com-object-folder-path")?.textContent ?? null))
    .toEqual(["Saved path: CH-1 › PB-3", "Saved path: CH-2 › PB-3", null]);
  expect(headers.map(r => r.querySelector(".com-object-channel-count")?.textContent)).toEqual(["1 / 1 objects", "1 / 1 objects", "2 / 2 objects"]);
  expect([1, 2, 3, 4].map(id => summaryRow(v.host, id).dataset.groupKey)).toEqual([F1, F2, "folder-none", "folder-none"]);
  // The row keeps its evaluated channel as a separate fact, and its refusal note.
  expect(summaryRow(v.host, 1).dataset.channelKey).toBe("channel:c-1");
  expect(summaryRow(v.host, 3).querySelector(".com-object-project-folder")?.textContent).toBe("No project folder: folders of module objects are not supported yet");
  expect(v.host.querySelector(".com-folder-view-note")?.textContent).toBe(
    "Folders as saved in the project at import; later parameter changes are not reflected and the product's folder names are not looked up.");
  await show(v, "All objects");
  expect(VIEWS.map(name => v.button(name)?.getAttribute("aria-pressed"))).toEqual(["false", "false", "true"]);
  expect(v.host.querySelectorAll("tr.com-object-channel")).toHaveLength(0);
  expect(summaryRow(v.host, 1).hasAttribute("data-group-key")).toBe(false);
  expect(v.host.querySelector(".com-folder-view-note")).toBeNull();
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it("WP-06 folder view: channel and folder collapse states are independent and visibility follows the current view", async () => {
  // Both objects are unevaluated, so the channel view has only `unassigned`.
  const v = await render([object(1, { project_folder: pb3("CH-1") }), object(2)]);
  await v.click(channelButton(v.host));
  expect(channelButton(v.host).getAttribute("aria-expanded")).toBe("true");
  await show(v, "By saved project folder");
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("false");
  expect(groupButton(v.host, "folder-none")?.getAttribute("aria-expanded")).toBe("false");
  for (const id of [1, 2]) expect(summaryRow(v.host, id).hidden, `summary ${id}`).toBe(true);
  await v.click(groupButton(v.host, F1)!);
  expect(summaryRow(v.host, 1).hidden).toBe(false);
  expect(summaryRow(v.host, 2).hidden).toBe(true);
  await show(v, "By channel");
  expect(channelButton(v.host).getAttribute("aria-expanded")).toBe("true");
  for (const id of [1, 2]) expect(summaryRow(v.host, id).hidden, `summary ${id}`).toBe(false);
  await show(v, "By saved project folder");
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("true");
  expect(groupButton(v.host, "folder-none")?.getAttribute("aria-expanded")).toBe("false");
  await v.cleanup();
});
it("WP-06 folder view: filters reveal and hide folder groups per view and reset restores folder expansion", async () => {
  const v = await render([object(1, { project_folder: pb3("CH-1") }), object(2, { project_folder: pb3("CH-2") }), object(3)]);
  await show(v, "By saved project folder");
  await v.click(groupButton(v.host, F1)!);
  await v.set(v.input("Search communication objects"), "Object 1");
  expect(v.host.querySelector(".com-table-count")?.textContent).toBe("1 / 3 objects");
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("true");
  expect(groupHeader(v.host, F2)?.hidden).toBe(true);
  expect(groupHeader(v.host, "folder-none")?.hidden).toBe(true);
  await v.click(groupButton(v.host, F1)!);
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("false");
  expect(summaryRow(v.host, 1).hidden).toBe(true);
  // The same object's channel group is not collapsed by the folder collapse.
  await show(v, "By channel");
  expect(channelButton(v.host).getAttribute("aria-expanded")).toBe("true");
  expect(summaryRow(v.host, 1).hidden).toBe(false);
  await show(v, "By saved project folder");
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("false");
  await v.click(v.button("Reset filters"));
  expect(groupButton(v.host, F1)?.getAttribute("aria-expanded")).toBe("true");
  expect(groupButton(v.host, F2)?.getAttribute("aria-expanded")).toBe("false");
  expect(groupHeader(v.host, F2)?.hidden).toBe(false);
  await v.cleanup();
});
it("WP-06 folder view: the same editor DOM and draft survive all three views and a folder collapse", async () => {
  const v = await render([object(1, { dpt: "DPST-1-1", project_folder: pb3("CH-1") }), object(2)]);
  await openGrouped(v, [1]);
  const draft = editorRow(v.host, 1).querySelector<HTMLSelectElement>(".dpt-field select")!;
  await v.set(draft, "DPST-9-2");
  await show(v, "By saved project folder");
  expect(editorRow(v.host, 1).hidden).toBe(true);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  await v.click(groupButton(v.host, F1)!);
  expect(editorRow(v.host, 1).hidden).toBe(false);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  await show(v, "All objects");
  expect(editorRow(v.host, 1).hidden).toBe(false);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  await show(v, "By saved project folder");
  await v.click(groupButton(v.host, F1)!);
  expect(editorRow(v.host, 1).hidden).toBe(true);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(draft);
  expect(draft.value).toBe("DPST-9-2");
  expect(api.setComObjectDpt).not.toHaveBeenCalled();
  await v.cleanup();
});
it.each(["succeeds", "is refused"])("WP-06 folder view: a request that %s while its folder is collapsed runs once and shows its outcome", async (outcome) => {
  let settle!: (value: unknown) => void;
  api.setComObjectDpt.mockImplementationOnce(() => new Promise((resolve, reject) => { settle = outcome === "succeeds" ? resolve : () => reject(new Error("refused by server")); }));
  const v = await render([object(1, { project_folder: pb3("CH-1") })]);
  await show(v, "By saved project folder");
  await v.click(groupButton(v.host, F1)!);
  await v.click(summaryRow(v.host, 1).querySelector<HTMLElement>("summary")!);
  const editor = editorRow(v.host, 1).querySelector<HTMLSelectElement>(".dpt-field select")!;
  await v.set(editor, "DPST-1-1");
  await act(async () => editor.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  await v.click(groupButton(v.host, F1)!);
  expect(editorRow(v.host, 1).hidden).toBe(true);
  await act(async () => settle({ ...tree, is_modified: true }));
  await show(v, "By channel"); await show(v, "By saved project folder");
  await v.click(groupButton(v.host, F1)!);
  expect(editorRow(v.host, 1).hidden).toBe(false);
  expect(editorRow(v.host, 1).querySelector(".dpt-field select")).toBe(editor);
  expect(api.setComObjectDpt).toHaveBeenCalledTimes(1);
  if (outcome === "succeeds") expect(v.applied).toHaveBeenCalledTimes(1);
  else { expect(editorRow(v.host, 1).textContent).toContain("refused by server"); expect(v.applied).not.toHaveBeenCalled(); }
  await v.cleanup();
});

tableColumnSmoke([["communication-objects", 6]], async () => { const v = await render([object(1)]); return v; });

tableColumnSmoke([["communication-objects", 6]], async () => {
  const v = await render([object(1, { project_folder: pb3("CH-1") })]); await show(v, "By saved project folder"); return v;
});

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
