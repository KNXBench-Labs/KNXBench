/** Notes workspace host: real identity, target catalogue and refresh after Apply. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NotesClient, NotesView } from "./features/notes/client";
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

let roots: Root[] = [];
beforeEach(() => { apiMock.currentProject.mockReset(); setUiLanguage("en"); });
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren();
  resetSettingsForTests(); resetUiLanguageForTests();
});

async function mount(props: Partial<React.ComponentProps<typeof NotesWorkspace>> & { client: NotesClient }) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onTreeUpdate = vi.fn(); const onDirtyChange = vi.fn();
  await act(async () => root.render(<NotesWorkspace tree={tree()} active onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} {...props} />));
  return { host, root, onTreeUpdate, onDirtyChange };
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
  it("hands the editor the tree's real identity and its stable-ID target catalogue", async () => {
    const client = fakeClient();
    const { host } = await mount({ client });
    expect(client.read).toHaveBeenCalledOnce();
    const options = Array.from(host.querySelectorAll("select option")).map((option) => [option.getAttribute("value"), option.textContent]);
    expect(options).toEqual([["project", "Project"], ["installation:0", "Installation: Home"], ["device:41", "Device: 1.1.4 Actuator"]]);
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
    const { host, root, onTreeUpdate, onDirtyChange } = await mount({ client });
    await editTitle(host, "Draft in progress");
    await act(async () => root.render(<NotesWorkspace tree={tree()} active={false} onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} client={client} />));
    expect(host.querySelector<HTMLElement>(".notes-workspace")?.hidden).toBe(true);
    await act(async () => root.render(<NotesWorkspace tree={tree()} active onTreeUpdate={onTreeUpdate} onDirtyChange={onDirtyChange} client={client} />));
    const values = Array.from(host.querySelectorAll("textarea")).map((field) => field.value);
    expect(values).toContain("Draft in progress");
    expect(client.read).toHaveBeenCalledOnce();
  });

  it("uses the German editor copy in the German UI", async () => {
    setUiLanguage("de");
    const { host } = await mount({ client: fakeClient() });
    expect(host.textContent).toContain("Projektnotizen");
    expect(Array.from(host.querySelectorAll("select option")).map((option) => option.textContent)).toContain("Gerät: 1.1.4 Actuator");
  });
});
