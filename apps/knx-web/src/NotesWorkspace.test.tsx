/** Notes workspace host: real identity, target catalogue and refresh after Apply. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NoteTargetsReader, NoteTargetsView, NotesClient, NotesView } from "./features/notes/client";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function setUiLanguage(language: "en" | "de") {
  saveUiLanguage(settingsStorage, language);
  resetUiLanguageForTests();
}

const apiMock = vi.hoisted(() => ({ currentProject: vi.fn() }));
vi.mock("./api", () => apiMock);

import NotesWorkspace from "./NotesWorkspace";

function tree(revision = 12): ProjectTree {
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    server_incarnation: "srv-7", project_incarnation: 3, snapshot_revision: revision,
    group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Home", topology: [], buildings: [], unassigned: [
      { id: 41, name: "Actuator", address: "1.1.4", description: null, com_object_count: 2 },
    ], group_ranges: [], group_addresses: [] }],
  };
}

function view(revision = 12): NotesView {
  return { serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: revision, persistence: "session-only",
    notes: [{ id: 1, position: 0, target: { kind: "device", id: 41 }, title: "Check wiring", body: "Line 1", includeInDocumentation: false }] };
}

function fakeClient(overrides: Partial<NotesClient> = {}): NotesClient {
  return {
    read: vi.fn().mockResolvedValue(view()),
    preview: vi.fn(async (_context, action) => ({ token: "plan-1", action, previous: view().notes[0], createdId: null, noOp: false, expiresInSeconds: 300 })),
    apply: vi.fn().mockResolvedValue({ changed: true, nativeGeneration: null, snapshotRevision: 13, persistence: "session-only" }),
    cancel: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

function catalogue(revision = 12, overrides: Partial<NoteTargetsView> = {}): NoteTargetsView {
  return { serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: revision, targets: [
    { target: { kind: "installation", id: 0 }, owner: [], parts: ["Home"] },
    { target: { kind: "device", id: 41 }, owner: [], parts: ["1.1.4", "Actuator"] },
    { target: { kind: "comObject", id: 81 }, owner: ["1.1.4", "Actuator"], parts: ["0", "Switch"] },
  ], ...overrides };
}

/** Answers in order; the last answer repeats. */
function fakeTargets(...answers: (NoteTargetsView | Error)[]) {
  const answer = (entry: NoteTargetsView | Error) => entry instanceof Error ? Promise.reject(entry) : Promise.resolve(entry);
  const reader = vi.fn<NoteTargetsReader>();
  for (const entry of answers.slice(0, -1)) reader.mockImplementationOnce(() => answer(entry));
  const last = answers.at(-1) ?? catalogue();
  return reader.mockImplementation(() => answer(last));
}

function optionsOf(host: Element) {
  return Array.from(host.querySelectorAll("select option")).map((option) => [option.getAttribute("value"), option.textContent]);
}

let roots: Root[] = [];
beforeEach(() => { apiMock.currentProject.mockReset(); setUiLanguage("en"); });
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren();
  resetSettingsForTests(); resetUiLanguageForTests();
});

async function mount(props: Partial<React.ComponentProps<typeof NotesWorkspace>> & { client: NotesClient }) {
  const targets = props.targets ?? fakeTargets();
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onTreeUpdate = vi.fn(); const onDirtyChange = vi.fn();
  await act(async () => root.render(<NotesWorkspace tree={tree()} active onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} {...props} targets={targets} />));
  return { host, root, onTreeUpdate, onDirtyChange, targets };
}

function button(host: Element, label: string): HTMLButtonElement {
  const found = Array.from(host.querySelectorAll("button")).find((entry) => entry.textContent === label);
  expect(found, `missing button ${label}`).toBeDefined();
  return found!;
}

async function editTitle(host: Element, value: string) {
  const caption = Array.from(host.querySelectorAll("label")).find((entry) => entry.textContent === "Title")!;
  const field = document.getElementById(caption.htmlFor) as HTMLTextAreaElement;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("NotesWorkspace", () => {
  it("hands the editor the tree's real identity and the service's target catalogue, objects included", async () => {
    const client = fakeClient();
    const { host, targets } = await mount({ client });
    expect(client.read).toHaveBeenCalledOnce();
    expect(targets).toHaveBeenCalledOnce();
    expect(optionsOf(host)).toEqual([["project", "Project"], ["installation:0", "Installation: Home"], ["device:41", "Device: 1.1.4 Actuator"],
      ["comObject:81", "Communication object: 0 Switch · 1.1.4 Actuator"]]);
    await editTitle(host, "Check wiring twice");
    await act(async () => button(host, "Preview change").click());
    expect(client.preview).toHaveBeenCalledWith({ serverIncarnation: "srv-7", projectIncarnation: 3, snapshotRevision: 12 },
      expect.objectContaining({ kind: "replace", id: 1 }), expect.anything());
  });

  it("refreshes the authoritative project through the host after a confirmed Apply", async () => {
    const client = fakeClient();
    const refreshed = tree(13);
    apiMock.currentProject.mockResolvedValue(refreshed);
    const { host, onTreeUpdate, onDirtyChange } = await mount({ client });
    await editTitle(host, "Check wiring twice");
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    await act(async () => button(host, "Preview change").click());
    expect(apiMock.currentProject).not.toHaveBeenCalled();
    await act(async () => button(host, "Confirm change").click());
    expect(client.apply).toHaveBeenCalledWith("plan-1", expect.anything());
    expect(apiMock.currentProject).toHaveBeenCalledOnce();
    expect(onTreeUpdate).toHaveBeenCalledWith(refreshed);
    expect(onDirtyChange).toHaveBeenLastCalledWith(false);
  });

  it("reports a failed refresh as such, not as a failed change", async () => {
    const client = fakeClient();
    apiMock.currentProject.mockRejectedValue(new Error("offline"));
    const { host, onTreeUpdate } = await mount({ client });
    await editTitle(host, "Check wiring twice");
    await act(async () => button(host, "Preview change").click());
    await act(async () => button(host, "Confirm change").click());
    expect(client.apply).toHaveBeenCalledOnce();
    expect(onTreeUpdate).not.toHaveBeenCalled();
    expect(host.querySelector('[role="alert"]')?.textContent).toMatch(/committed|refresh/i);
  });

  it("mounts no editor and reads nothing without a complete project identity", async () => {
    const client = fakeClient();
    const { host } = await mount({ client, tree: { ...tree(), snapshot_revision: undefined } });
    expect(client.read).not.toHaveBeenCalled();
    expect(host.querySelector("textarea")).toBeNull();
    expect(host.querySelector('[role="status"]')?.textContent).toContain("complete project identity");
  });

  it("stays mounted while hidden so a draft survives workbench navigation", async () => {
    const client = fakeClient();
    const { host, root, onTreeUpdate, onDirtyChange, targets } = await mount({ client });
    await editTitle(host, "Draft in progress");
    await act(async () => root.render(<NotesWorkspace tree={tree()} active={false} onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} client={client} targets={targets} />));
    expect(host.querySelector<HTMLElement>(".notes-workspace")?.hidden).toBe(true);
    await act(async () => root.render(<NotesWorkspace tree={tree()} active onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} client={client} targets={targets} />));
    const values = Array.from(host.querySelectorAll("textarea")).map((field) => field.value);
    expect(values).toContain("Draft in progress");
    expect(client.read).toHaveBeenCalledOnce();
  });

  it("uses the German editor copy in the German UI", async () => {
    setUiLanguage("de");
    const { host } = await mount({ client: fakeClient() });
    expect(host.textContent).toContain("Projektnotizen");
    expect(Array.from(host.querySelectorAll("select option")).map((option) => option.textContent)).toContain("Gerät: 1.1.4 Actuator");
    expect(Array.from(host.querySelectorAll("select option")).map((option) => option.textContent))
      .toContain("Kommunikationsobjekt: 0 Switch · 1.1.4 Actuator");
  });

  it("reads no targets without a complete project identity", async () => {
    const targets = fakeTargets();
    await mount({ client: fakeClient(), targets, tree: { ...tree(), snapshot_revision: undefined } });
    expect(targets).not.toHaveBeenCalled();
  });

  it("shows a stored target as unresolved only after the catalogue has answered", async () => {
    let answer: (view: NoteTargetsView) => void = () => undefined;
    const targets = vi.fn<NoteTargetsReader>(() => new Promise((resolve) => { answer = resolve; }));
    const client = fakeClient();
    const { host } = await mount({ client, targets });
    expect(host.querySelector("textarea")).toBeNull();
    expect(host.querySelector('[role="status"]')?.textContent).toBe("Loading note targets…");
    await act(async () => answer(catalogue()));
    expect(host.querySelector("select")?.value).toBe("device:41");
    expect(host.textContent).not.toContain("Stored target not in host catalogue");
  });

  it("keeps the editor usable when the catalogue fails, says so, and retries on request", async () => {
    const targets = fakeTargets(new Error("offline"), catalogue());
    const client = fakeClient();
    const { host } = await mount({ client, targets });
    const alert = host.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain("Note targets could not be loaded");
    expect(optionsOf(host).map(([value]) => value)).toEqual(["device:41", "project"]);
    await act(async () => button(host, "Load targets again").click());
    expect(targets).toHaveBeenCalledTimes(2);
    expect(host.querySelector('[role="alert"]')).toBeNull();
    expect(optionsOf(host).map(([value]) => value)).toEqual(["project", "installation:0", "device:41", "comObject:81"]);
  });

  it("ignores a catalogue that belongs to another server or project", async () => {
    const targets = fakeTargets(catalogue(12, { projectIncarnation: 99 }));
    const { host } = await mount({ client: fakeClient(), targets });
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("Note targets could not be loaded");
    expect(optionsOf(host).map(([value]) => value)).not.toContain("comObject:81");
  });

  it("reloads the catalogue for a new revision and keeps the old choices meanwhile", async () => {
    const renamed = catalogue(13, { targets: [{ target: { kind: "device", id: 41 }, owner: [], parts: ["1.1.4", "Renamed"] }] });
    let answer: (view: NoteTargetsView) => void = () => undefined;
    const targets = vi.fn<NoteTargetsReader>()
      .mockResolvedValueOnce(catalogue())
      .mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
    const client = fakeClient();
    const { host, root, onTreeUpdate, onDirtyChange } = await mount({ client, targets });
    await act(async () => root.render(<NotesWorkspace tree={tree(13)} active onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} client={client} targets={targets} />));
    expect(targets).toHaveBeenCalledTimes(2);
    expect(optionsOf(host)).toContainEqual(["device:41", "Device: 1.1.4 Actuator"]);
    await act(async () => answer(renamed));
    expect(optionsOf(host)).toContainEqual(["device:41", "Device: 1.1.4 Renamed"]);
  });
});
