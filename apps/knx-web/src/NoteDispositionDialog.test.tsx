/** Note decisions before a delete: defaults, move targets, bound apply, refusals. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
beforeEach(() => Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }));
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NoteTargetsView } from "./features/notes/client";
const api = vi.hoisted(() => ({
  deleteWithNotes: vi.fn(),
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  noteRefusal: vi.fn(),
}));
vi.mock("./api", () => api);
import NoteDispositionDialog, { useNoteAwareDelete } from "./NoteDispositionDialog";
import type { Deletion, DispositionRequired } from "./noteDisposition";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

const context = { serverIncarnation: "srv", projectIncarnation: 2, snapshotRevision: 9 };
const required: DispositionRequired = {
  message: "this change would leave 2 project notes without their target",
  notes: [
    { id: 1, title: "Wiring check", target: { kind: "device", id: 4 } },
    { id: 3, title: "Object hint", target: { kind: "comObject", id: 8 } },
  ],
  removedTargets: [{ kind: "device", id: 4 }, { kind: "comObject", id: 8 }],
  context,
};
const deletion: Deletion = { kind: "devices", ids: [4] };
const catalogue: NoteTargetsView = {
  ...context,
  targets: [
    { target: { kind: "line", id: 2 }, owner: [], parts: ["1.1", "Main line"] },
    { target: { kind: "device", id: 4 }, owner: [], parts: ["1.1.4", "Dimmer"] },
    { target: { kind: "comObject", id: 8 }, owner: ["1.1.4", "Dimmer"], parts: ["0", "Switch"] },
    { target: { kind: "device", id: 5 }, owner: [], parts: ["1.1.5", "Sensor"] },
  ],
};
const tree = { schema_version: 16, snapshot_revision: 10 } as unknown as ProjectTree;
const http = (status: number, body: Record<string, unknown>) => Object.assign(new Error(String(body.error)), { status, body });

let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLDivElement | undefined;
const deleted = vi.fn();
const closed = vi.fn();
afterEach(async () => {
  if (root) await act(() => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
  vi.clearAllMocks(); resetSettingsForTests(); resetUiLanguageForTests();
});
const text = () => document.body.textContent ?? "";
function button(label: string) { return [...document.querySelectorAll("button")].find((b) => b.textContent === label); }
async function render(targets = vi.fn().mockResolvedValue(catalogue)) {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root!.render(<NoteDispositionDialog deletion={deletion} required={required} targets={targets}
    onDeleted={deleted} onClose={closed} />));
  return targets;
}
function fieldset(note: number) { return document.querySelector(`[data-note-disposition-note="${note}"]`)!; }
async function pick(note: number, value: "project" | "move" | "delete") {
  const radio = fieldset(note).querySelector<HTMLInputElement>(`input[value="${value}"]`)!;
  await act(async () => radio.click());
}
async function moveTo(note: number, key: string) {
  const select = fieldset(note).querySelector("select")!;
  const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!;
  await act(async () => { setter.call(select, key); select.dispatchEvent(new Event("change", { bubbles: true })); });
}

it("lists every stranded note with its target and starts on keeping it", async () => {
  await render();
  expect(text()).toContain("Notes need a decision");
  expect(text()).toContain("Deleting this removes the targets of 2 project notes");
  expect(fieldset(1).textContent).toContain("Wiring check");
  expect(fieldset(1).textContent).toContain("on Device: 1.1.4 Dimmer");
  expect(fieldset(3).textContent).toContain("on Communication object: 0 Switch · 1.1.4 Dimmer");
  for (const note of [1, 3]) {
    expect(fieldset(note).querySelector<HTMLInputElement>('input[value="project"]')!.checked).toBe(true);
  }
  expect(button("Delete and apply choices")!.disabled).toBe(false);
  expect(api.deleteWithNotes).not.toHaveBeenCalled();
});

it("offers only surviving elements as move targets and waits for one", async () => {
  await render();
  const options = [...fieldset(1).querySelectorAll("option")].map((option) => option.textContent);
  expect(options).toEqual(["Choose an element…", "Line: 1.1 Main line", "Device: 1.1.5 Sensor"]);
  await pick(1, "move");
  expect(button("Delete and apply choices")!.disabled).toBe(true);
  expect(text()).toContain("Choose an element for every note you move.");
  await moveTo(1, "line:2");
  expect(button("Delete and apply choices")!.disabled).toBe(false);
});

it("sends exactly the reviewed choices bound to the refusal's snapshot", async () => {
  api.deleteWithNotes.mockResolvedValue(tree);
  await render();
  await pick(1, "move");
  await moveTo(1, "device:5");
  await pick(3, "delete");
  await act(async () => button("Delete and apply choices")!.click());
  expect(api.deleteWithNotes).toHaveBeenCalledWith(context, deletion, [
    { note: 1, action: { kind: "move", target: { kind: "device", id: 5 } } },
    { note: 3, action: { kind: "delete" } },
  ]);
  expect(deleted).toHaveBeenCalledWith(tree);
});

it("a stale project blocks a retry and says nothing was deleted", async () => {
  api.deleteWithNotes.mockRejectedValue(http(409, { kind: "noteDispositionStale", error: "changed" }));
  await render();
  await act(async () => button("Delete and apply choices")!.click());
  const alert = document.querySelector('[role="alert"]')!;
  expect(alert.textContent).toContain("nothing was deleted");
  expect(document.activeElement).toBe(alert);
  expect(button("Delete and apply choices")!.disabled).toBe(true);
  expect(deleted).not.toHaveBeenCalled();
});

it("a refused choice keeps the dialog open with the server's reason", async () => {
  api.deleteWithNotes.mockRejectedValue(http(422, { kind: "noteDispositionRefused", error: "notes without a choice: 3" }));
  await render();
  await act(async () => button("Delete and apply choices")!.click());
  expect(document.querySelector('[role="alert"]')!.textContent).toBe("Nothing was deleted: notes without a choice: 3");
  expect(button("Delete and apply choices")!.disabled).toBe(false);
});

it("cancel changes nothing", async () => {
  await render();
  await act(async () => button("Cancel")!.click());
  expect(closed).toHaveBeenCalledOnce();
  expect(api.deleteWithNotes).not.toHaveBeenCalled();
});

it("keeping and deleting still work when move targets cannot be read", async () => {
  await render(vi.fn().mockRejectedValue(new Error("offline")));
  expect(text()).toContain("Move targets could not be loaded");
  expect(fieldset(1).textContent).toContain("on Device #4");
  expect(fieldset(1).querySelector("select")!.disabled).toBe(true);
  expect(button("Delete and apply choices")!.disabled).toBe(false);
});

it("speaks German with du", async () => {
  settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
  await render();
  expect(text()).toContain("Notizen brauchen eine Entscheidung");
  expect(text()).toContain("gelöscht wird erst, wenn du bestätigst");
  expect(fieldset(1).textContent).toContain("Am Projekt behalten");
  expect(button("Löschen und Auswahl anwenden")).toBeDefined();
});

function Harness(props: { action: () => Promise<ProjectTree>; onError: (message: string) => void }) {
  const noteAware = useNoteAwareDelete(deleted, vi.fn().mockResolvedValue(catalogue));
  return <><button type="button" onClick={() => void noteAware.run(deletion, props.action, props.onError)}>Delete</button>{noteAware.dialog}</>;
}
async function renderHarness(action: () => Promise<ProjectTree>, onError = vi.fn()) {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root!.render(<Harness action={action} onError={onError} />));
  await act(async () => button("Delete")!.click());
  return onError;
}

it("the hook opens the dialog only for the structured refusal", async () => {
  const onError = await renderHarness(() => Promise.reject(http(409, { kind: "noteDispositionRequired", error: required.message,
    notes: required.notes, removedTargets: required.removedTargets, ...context })));
  expect(document.querySelector("[data-note-disposition]")).not.toBeNull();
  expect(onError).not.toHaveBeenCalled();
  expect(deleted).not.toHaveBeenCalled();
});

it("the hook passes other refusals and successes through unchanged", async () => {
  const onError = await renderHarness(() => Promise.reject(http(400, { error: "device 4 still has group links" })));
  expect(onError).toHaveBeenCalledWith("device 4 still has group links");
  expect(document.querySelector("[data-note-disposition]")).toBeNull();
  await act(() => root!.unmount()); host?.remove(); root = undefined;
  await renderHarness(() => Promise.resolve(tree));
  expect(deleted).toHaveBeenCalledWith(tree);
  expect(document.querySelector("[data-note-disposition]")).toBeNull();
});
