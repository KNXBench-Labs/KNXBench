/** Object-local function choices retain the shared editor's write and lifetime safeguards. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { ParameterField, ParameterPanel } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
const api = vi.hoisted(() => ({ setParameterValue: vi.fn(), currentProject: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { ComObjectFunctionControls } from "./ParameterPanel";
import ComMutationBoundary from "./ComMutationBoundary";

const tree: ProjectTree = { schema_version: 16, errors: 0, warnings: 0, can_undo: true, can_redo: false,
  is_modified: true, group_address_style: "ThreeLevel", installations: [] };
const snapshot = { serverIncarnation: "function-test", projectIncarnation: 1, snapshotRevision: 2 };
const field: ParameterField = { etsId: "Function", name: "Function", text: "Button function", nameLanguage: null,
  textLanguage: null, kind: "Restriction", value: "0", valueSource: "Stored", editable: true,
  writeEtsId: "Function", access: null, min: null, max: null, displayOrder: null, controlledComObjectIds: [7],
  enumOptions: [{ value: "0", text: "Switch", language: null }, { value: "1", text: "Dimming", language: null }] };
const panel: ParameterPanel = { programId: "Fictional", sourceLanguage: null, tree: null, stale: [], diagnostics: [],
  sections: [{ scope: null, fields: [field] }] };
async function mount(shown: ParameterPanel | null = panel) {
  api.setParameterValue.mockReset();
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host), applied = vi.fn(), acceptWrite = vi.fn(), busy = vi.fn();
  await act(async () => root.render(<ComMutationBoundary onBusy={busy}>
    <input aria-label="Sibling DPT" />
    <ComObjectFunctionControls deviceId={3} objectId={7} onValueApplied={applied}
      state={{ panel: shown, snapshot, language: null, loadError: null, acceptWrite }} />
  </ComMutationBoundary>));
  return { host, applied, acceptWrite, busy, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}
async function choose(host: HTMLElement) {
  await act(async () => {
    const select = host.querySelector<HTMLSelectElement>("select")!;
    Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value")!.set!.call(select, "1");
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
}
it("disables sibling controls while pending and never publishes after unmount", async () => {
  const view = await mount();
  let resolve!: (value: ParameterPanel) => void;
  api.setParameterValue.mockImplementation(() => new Promise<ParameterPanel>(r => { resolve = r; }));
  await choose(view.host);
  expect(api.setParameterValue).toHaveBeenCalledExactlyOnceWith(3, "Function", "1", null, { ...snapshot, expectedValue: "0" });
  expect(view.host.querySelector("fieldset")!.disabled).toBe(true);
  expect(view.busy).toHaveBeenCalledWith(true);
  await view.cleanup();
  await act(async () => resolve({ ...panel, tree }));
  expect(view.applied).not.toHaveBeenCalled();
});
it("applies immediately and forwards the authoritative tree exactly once", async () => {
  const view = await mount();
  api.setParameterValue.mockResolvedValue({ ...panel, tree });
  await choose(view.host);
  expect(view.applied).toHaveBeenCalledExactlyOnceWith(tree);
  expect(view.acceptWrite).toHaveBeenCalledExactlyOnceWith(3, null, { ...panel, tree });
  expect(view.host.querySelector("fieldset")!.disabled).toBe(false);
  await view.cleanup();
});
it("keeps an ordinary refusal inline and restores the committed choice", async () => {
  const view = await mount();
  api.setParameterValue.mockRejectedValue(new Error("Unsupported object mapping; nothing changed"));
  await choose(view.host);
  expect(view.host.querySelector("select")!.value).toBe("0");
  expect(view.host.textContent).toContain("Unsupported object mapping; nothing changed");
  expect(view.applied).not.toHaveBeenCalled();
  await view.cleanup();
});
it("does not guess choices from another object, restricted field, module or older server", async () => {
  const view = await mount({ ...panel, sections: [
    { scope: null, fields: [{ ...field, controlledComObjectIds: [8] }, { ...field, controlledComObjectIds: undefined },
      { ...field, access: "Read" }] },
    { scope: { moduleNode: 1, moduleId: "Module", moduleDefId: "Definition" }, fields: [field] },
  ] });
  expect(view.host.querySelector("select")).toBeNull();
  expect(view.host.textContent).toContain("Other settings are under Parameters");
  expect(api.setParameterValue).not.toHaveBeenCalled();
  await view.cleanup();
});
it("shows loading rather than an unsupported claim before the panel arrives", async () => {
  const view = await mount(null);
  expect(view.host.textContent).not.toContain("No supported function selection");
  expect(view.host.querySelector(".task-status-text")).not.toBeNull();
  await view.cleanup();
});
