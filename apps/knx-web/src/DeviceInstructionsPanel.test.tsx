/** Instructions tab host: identity, read-only fallback and refresh after Apply. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstructionClient, InstructionView } from "./features/rtf/client";
import { literal } from "./features/rtf/document";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const apiMock = vi.hoisted(() => ({ currentProject: vi.fn() }));
vi.mock("./api", () => apiMock);

import DeviceInstructionsPanel from "./DeviceInstructionsPanel";

function tree(revision = 9): ProjectTree {
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    server_incarnation: "srv-i", project_incarnation: 2, snapshot_revision: revision,
    group_address_style: "ThreeLevel", installations: [],
  };
}
function view(revision = 9): InstructionView {
  return { serverIncarnation: "srv-i", projectIncarnation: 2, snapshotRevision: revision, device: 41,
    originalXmlSha256: "a".repeat(64), originalPresent: true, originalBytes: 12, originalFormat: "plain",
    unsupported: [], previewError: null, effectiveDocument: literal("Mount at 1.5 m"), overrideDocument: null,
    persistence: "session-only" };
}
function fakeClient(overrides: Partial<InstructionClient> = {}): InstructionClient {
  return {
    read: vi.fn().mockResolvedValue(view()),
    preview: vi.fn().mockResolvedValue({ token: "plan-i", device: 41, replacement: literal("Mount at 1.6 m"), conversionRequired: false, noOp: false, expiresInSeconds: 300 }),
    apply: vi.fn().mockResolvedValue({ changed: true, nativeGeneration: null, snapshotRevision: 10, persistence: "session-only" }),
    cancel: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

let roots: Root[] = [];
beforeEach(() => { apiMock.currentProject.mockReset(); });
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren(); resetSettingsForTests(); resetUiLanguageForTests();
});

async function mount(props: Partial<React.ComponentProps<typeof DeviceInstructionsPanel>> & { client: InstructionClient }) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onApplied = vi.fn(); const onDirtyChange = vi.fn();
  await act(async () => root.render(<DeviceInstructionsPanel deviceId={41} tree={tree()} onApplied={onApplied} onDirtyChange={onDirtyChange} {...props} />));
  return { host, root, onApplied, onDirtyChange };
}
function button(host: Element, label: string) {
  const found = Array.from(host.querySelectorAll("button")).find((entry) => entry.textContent === label);
  expect(found, `missing button ${label}`).toBeDefined();
  return found!;
}
async function type(host: Element, value: string) {
  const field = host.querySelector("textarea")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("DeviceInstructionsPanel", () => {
  it("previews against the tree's real identity for the selected device", async () => {
    const client = fakeClient();
    const { host, onDirtyChange } = await mount({ client });
    expect(client.read).toHaveBeenCalledWith(41, expect.anything());
    await type(host, "Mount at 1.6 m");
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    await act(async () => button(host, "Preview change").click());
    expect(client.preview).toHaveBeenCalledWith(41, { serverIncarnation: "srv-i", projectIncarnation: 2, snapshotRevision: 9 }, literal("Mount at 1.6 m"), expect.anything());
  });

  it("refreshes the authoritative project through the host after Apply", async () => {
    const client = fakeClient();
    const refreshed = tree(10);
    apiMock.currentProject.mockResolvedValue(refreshed);
    const { host, onApplied, onDirtyChange } = await mount({ client });
    await type(host, "Mount at 1.6 m");
    await act(async () => button(host, "Preview change").click());
    await act(async () => button(host, "Apply change").click());
    expect(client.apply).toHaveBeenCalledWith(41, "plan-i", false, expect.anything());
    expect(apiMock.currentProject).toHaveBeenCalledOnce();
    expect(onApplied).toHaveBeenCalledWith(refreshed);
    expect(onDirtyChange).toHaveBeenLastCalledWith(false);
  });

  it("refreshes even after a no-op Apply", async () => {
    const client = fakeClient({ apply: vi.fn().mockResolvedValue({ changed: false, nativeGeneration: null, snapshotRevision: 9, persistence: "session-only" }) });
    apiMock.currentProject.mockResolvedValue(tree());
    const { host } = await mount({ client });
    await type(host, "Mount at 1.6 m");
    await act(async () => button(host, "Preview change").click());
    await act(async () => button(host, "Apply change").click());
    expect(apiMock.currentProject).toHaveBeenCalledOnce();
  });

  it("shows a refresh failure instead of swallowing it", async () => {
    const client = fakeClient();
    apiMock.currentProject.mockRejectedValue(new Error("server went away"));
    const { host, onApplied } = await mount({ client });
    await type(host, "Mount at 1.6 m");
    await act(async () => button(host, "Preview change").click());
    await act(async () => button(host, "Apply change").click());
    expect(client.apply).toHaveBeenCalledOnce();
    expect(onApplied).not.toHaveBeenCalled();
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("server went away");
  });

  it("mounts read-only, with no write controls, when the tree lacks a complete identity", async () => {
    const client = fakeClient();
    const { host } = await mount({ client, tree: { ...tree(), server_incarnation: undefined } });
    expect(client.read).toHaveBeenCalledOnce();
    expect(host.querySelector("textarea")?.readOnly).toBe(true);
    expect(Array.from(host.querySelectorAll("button")).map((entry) => entry.textContent)).not.toContain("Preview change");
  });

  it("reports a clean state when it unmounts", async () => {
    const client = fakeClient();
    const { host, root, onDirtyChange } = await mount({ client });
    await type(host, "Mount at 1.6 m");
    await act(async () => root.unmount());
    roots = roots.filter((entry) => entry !== root);
    expect(onDirtyChange).toHaveBeenLastCalledWith(false);
  });

  it("uses the German editor copy in the German UI", async () => {
    saveUiLanguage(settingsStorage, "de"); resetUiLanguageForTests();
    const { host } = await mount({ client: fakeClient() });
    expect(host.textContent).toContain("Installationshinweise");
  });
});
