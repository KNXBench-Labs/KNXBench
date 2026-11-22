// @vitest-environment happy-dom
import { describe, expect, it, vi, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import DeviceConfigPanel from "./DeviceConfigPanel";
import type { ProjectTree } from "./bindings/ProjectTree";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
vi.mock("./i18n", () => ({ useTranslate: () => (key: string, params?: Record<string, string>) => {
  if (!params) return key;
  return Object.entries(params).reduce((acc, [k, v]) => acc.replaceAll(`{${k}}`, v), key);
} }));

let container: HTMLDivElement | null = null;
let root: Root | null = null;
afterEach(() => { act(() => root?.unmount()); container?.remove(); root = null; container = null; vi.unstubAllGlobals(); });

function treeOf(project: number, revision: number, server = "srv"): ProjectTree {
  return { server_incarnation: server, project_incarnation: project, snapshot_revision: revision } as unknown as ProjectTree;
}
async function show(tree: ProjectTree, deviceId = 3) {
  if (!container) {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
  }
  await act(async () => { root!.render(<DeviceConfigPanel deviceId={deviceId} tree={tree} onApplied={() => undefined} />); });
}
async function flush() { for (let i = 0; i < 4; i++) await act(async () => { await new Promise((r) => setTimeout(r, 0)); }); }
const body = () => document.body.textContent ?? "";
function button(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll("button")].find((b) => (b.textContent ?? "").includes(name));
  if (!found) throw new Error(`button ${name} not found`);
  return found as HTMLButtonElement;
}
const textarea = () => document.querySelector("textarea") as HTMLTextAreaElement;
async function type(value: string, field: HTMLTextAreaElement | HTMLInputElement = textarea()) {
  await act(async () => {
    const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

const sha = "a".repeat(64);
const b64 = (text: string) => btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
function profileOf(document: string, digest: string, device = 3) {
  return { device, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: sha, documentBytes: document.length, documentSha256: digest };
}
function deviceView(project: number, revision: number, document: string | null, digest = "d".repeat(64), device = 3) {
  return {
    device, serverIncarnation: "srv", projectIncarnation: project, snapshotRevision: revision, persistence: "native-working-history",
    profile: document === null ? null : { ...profileOf(document, digest, device), documentB64: b64(document), documentError: null as string | null },
  };
}
function json(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
}
function deferred() {
  let resolve!: (r: Response) => void;
  const promise = new Promise<Response>((r) => { resolve = r; });
  return { promise, resolve };
}

const ORIGINAL = '{ "known" : 1,\n  "unknown_member": [true, null] }';

describe("DeviceConfigPanel", () => {
  it("shows the stored document verbatim and previews it back as the exact stored bytes", async () => {
    const calls: string[] = [];
    const transport = vi.fn(async (url: string, init?: { body?: string }) => {
      calls.push(url);
      if (url.endsWith("/devices/3")) return json(deviceView(1, 7, ORIGINAL));
      const sent = JSON.parse(init!.body!);
      return json({ token: "tok", action: sent.action, previous: profileOf(ORIGINAL, "d".repeat(64)), noOp: true, expiresInSeconds: 300 });
    });
    vi.stubGlobal("fetch", transport);
    await show(treeOf(1, 7));
    await flush();
    expect(textarea().value).toBe(ORIGINAL);
    expect(body()).toContain("deviceConfig.current");
    expect(calls).toEqual(["/api/project/device-config-profiles/devices/3"]); // one device, no list
    await act(async () => { button("deviceConfig.preview").click(); });
    await flush();
    const previewCall = transport.mock.calls[1][1] as { body: string };
    expect(JSON.parse(previewCall.body).action.draft.documentB64).toBe(b64(ORIGINAL));
    expect(body()).toContain("deviceConfig.noChange");
  });

  it("reloads a clean editor on revision change, also for an equal-length different document", async () => {
    const other = ORIGINAL.replace("known", "knowm");
    expect(other.length).toBe(ORIGINAL.length);
    const current = { view: deviceView(1, 7, ORIGINAL, "1".repeat(64)) };
    vi.stubGlobal("fetch", vi.fn(async () => json(current.view)));
    await show(treeOf(1, 7));
    await flush();
    expect(textarea().value).toBe(ORIGINAL);
    current.view = deviceView(1, 8, other, "2".repeat(64));
    await show(treeOf(1, 8));
    await flush();
    expect(textarea().value).toBe(other);
    expect(body()).not.toContain("deviceConfig.staleDraft");
  });

  it("keeps a dirty draft, flags it stale, blocks preview and lets the user choose", async () => {
    const current = { view: deviceView(1, 7, ORIGINAL, "1".repeat(64)) };
    vi.stubGlobal("fetch", vi.fn(async () => json(current.view)));
    await show(treeOf(1, 7));
    await flush();
    await type('{"mine":true}');
    expect(body()).toContain("deviceConfig.unsaved");
    current.view = deviceView(1, 8, '{"undone":1}', "2".repeat(64));
    await show(treeOf(1, 8));
    await flush();
    expect(textarea().value).toBe('{"mine":true}');
    expect(body()).toContain("deviceConfig.staleDraft");
    expect(button("deviceConfig.preview").disabled).toBe(true);
    await act(async () => { button("deviceConfig.keepDraft").click(); });
    expect(textarea().value).toBe('{"mine":true}');
    expect(body()).not.toContain("deviceConfig.staleDraft");
    expect(button("deviceConfig.preview").disabled).toBe(false);

    current.view = deviceView(1, 9, '{"third":3}', "3".repeat(64));
    await show(treeOf(1, 9));
    await flush();
    expect(body()).toContain("deviceConfig.staleDraft");
    await act(async () => { button("deviceConfig.loadStored").click(); });
    expect(textarea().value).toBe('{"third":3}');
    expect(body()).not.toContain("deviceConfig.unsaved");
  });

  it("does not flag a dirty draft when the stored content did not change", async () => {
    const current = { view: deviceView(1, 7, ORIGINAL, "1".repeat(64)) };
    vi.stubGlobal("fetch", vi.fn(async () => json(current.view)));
    await show(treeOf(1, 7));
    await flush();
    await type('{"mine":true}');
    current.view = deviceView(1, 8, ORIGINAL, "1".repeat(64));
    await show(treeOf(1, 8));
    await flush();
    expect(textarea().value).toBe('{"mine":true}');
    expect(body()).not.toContain("deviceConfig.staleDraft");
  });

  it("discards a delayed read of an older revision and a read for a different context", async () => {
    const late = deferred();
    const fetchMock = vi.fn()
      .mockImplementationOnce(() => late.promise)
      .mockImplementationOnce(async () => json(deviceView(1, 8, '{"new":1}', "2".repeat(64))));
    vi.stubGlobal("fetch", fetchMock);
    await show(treeOf(1, 7));
    await show(treeOf(1, 8));
    await flush();
    expect(textarea().value).toBe('{"new":1}');
    await act(async () => { late.resolve(json(deviceView(1, 7, '{"old":1}', "1".repeat(64)))); });
    await flush();
    expect(textarea().value).toBe('{"new":1}');

    // A response that names another revision than the tree is not adopted, it is reported.
    fetchMock.mockImplementationOnce(async () => json(deviceView(1, 99, '{"x":0}')));
    await show(treeOf(1, 9));
    await flush();
    expect(textarea().value).toBe('{"new":1}');
    expect(body()).toContain("deviceConfig.outdated");
  });

  it("never shares a draft or a late response between projects with the same device id", async () => {
    const fetchMock = vi.fn()
      .mockImplementationOnce(async () => json(deviceView(1, 7, '{"project":"one"}', "1".repeat(64))))
      .mockImplementationOnce(async () => json(deviceView(2, 7, '{"project":"two"}', "2".repeat(64))));
    vi.stubGlobal("fetch", fetchMock);
    await show(treeOf(1, 7));
    await flush();
    await type('{"draft":"of project one"}');
    // Same device id and revision number, other project incarnation, same mounted parent slot.
    await show(treeOf(2, 7));
    await flush();
    expect(textarea().value).toBe('{"project":"two"}');
    expect(body()).not.toContain("deviceConfig.unsaved");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("drops a delayed read of the previous project after a project switch", async () => {
    const late = deferred();
    const fetchMock = vi.fn()
      .mockImplementationOnce(() => late.promise)
      .mockImplementationOnce(async () => json(deviceView(2, 7, '{"project":"two"}', "2".repeat(64))));
    vi.stubGlobal("fetch", fetchMock);
    await show(treeOf(1, 7));
    await show(treeOf(2, 7));
    await flush();
    await act(async () => { late.resolve(json(deviceView(1, 7, '{"project":"one-late"}'))); });
    await flush();
    expect(textarea().value).toBe('{"project":"two"}');
  });

  it("drops a preview that resolves after the revision moved on and cancels its token", async () => {
    const slowPreview = deferred();
    const calls: string[] = [];
    vi.stubGlobal("fetch", vi.fn(async (url: string) => {
      calls.push(url);
      if (url.endsWith("/devices/3")) return json(deviceView(1, calls.filter((c) => c.endsWith("/devices/3")).length === 1 ? 7 : 8, ORIGINAL));
      if (url.endsWith("/preview")) return slowPreview.promise;
      return json({ cancelled: true });
    }));
    await show(treeOf(1, 7));
    await flush();
    await act(async () => { button("deviceConfig.preview").click(); });
    await show(treeOf(1, 8));
    await flush();
    const action = { kind: "set", draft: { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: sha, documentB64: b64(ORIGINAL) } };
    await act(async () => { slowPreview.resolve(json({ token: "late", action, previous: null, noOp: false, expiresInSeconds: 300 })); });
    await flush();
    expect(body()).not.toContain("deviceConfig.confirmText");
    expect(calls).toContain("/api/project/device-config-profiles/cancel");
  });

  it("explains a non-UTF-8 stored document without discarding or rewriting it", async () => {
    const view = deviceView(1, 7, "x");
    view.profile!.documentB64 = "_w"; // single 0xFF byte
    view.profile!.documentBytes = 1;
    vi.stubGlobal("fetch", vi.fn(async () => json(view)));
    await show(treeOf(1, 7));
    await flush();
    expect(body()).toContain("deviceConfig.notUtf8");
    expect(textarea().value).toBe("");
  });

  it("reports a stored document that is not strict JSON", async () => {
    const view = deviceView(1, 7, '{"a":1,"a":2}');
    view.profile!.documentError = "duplicate member";
    vi.stubGlobal("fetch", vi.fn(async () => json(view)));
    await show(treeOf(1, 7));
    await flush();
    expect(body()).toContain("deviceConfig.storedNotStrict");
    expect(textarea().value).toBe('{"a":1,"a":2}');
  });

  it("refuses invalid JSON before any preview request", async () => {
    const transport = vi.fn(async () => json(deviceView(1, 7, null)));
    vi.stubGlobal("fetch", transport);
    await show(treeOf(1, 7));
    await flush();
    expect(body()).toContain("deviceConfig.none");
    const inputs = [...document.querySelectorAll("input")] as HTMLInputElement[];
    for (const [i, value] of ["smartsensor", "1.0.0", sha].entries()) await type(value, inputs[i]);
    await type("not json");
    await act(async () => { button("deviceConfig.preview").click(); });
    await flush();
    expect(body()).toContain("deviceConfig.invalidJson");
    expect(transport).toHaveBeenCalledTimes(1);
  });
});
