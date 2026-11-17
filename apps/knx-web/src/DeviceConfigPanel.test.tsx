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
async function renderPanel(props: { deviceId: number; onApplied?: (t: ProjectTree) => void }) {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root!.render(<DeviceConfigPanel deviceId={props.deviceId} tree={tree} onApplied={props.onApplied ?? (() => undefined)} />); });
  return container;
}
async function flush() { await act(async () => { await new Promise((r) => setTimeout(r, 0)); }); }
function textOf(el: Element | null): string { return el?.textContent ?? ""; }
function button(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll("button")].find((b) => textOf(b).includes(name));
  if (!found) throw new Error(`button ${name} not found`);
  return found as HTMLButtonElement;
}
function textarea(): HTMLTextAreaElement {
  const found = document.querySelector("textarea");
  if (!found) throw new Error("textarea not found");
  return found;
}
async function type(value: string) {
  const field = textarea();
  await act(async () => {
    const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
    setter.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => { await new Promise((r) => setTimeout(r, 0)); });
}

const tree = {
  server_incarnation: "srv",
  project_incarnation: 1,
  snapshot_revision: 7,
} as unknown as ProjectTree;

function jsonResponse(value: unknown, ok = true, status = ok ? 200 : 400): Response {
  return new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
}

const sha = "a".repeat(64);
const view = {
  serverIncarnation: "srv", projectIncarnation: 1, snapshotRevision: 7,
  profiles: [{ device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: sha, documentBytes: 14 }],
  persistence: "native-working-history",
};

describe("DeviceConfigPanel", () => {
  it("loads the stored profile and previews a change", async () => {
    const transport = vi.fn()
      .mockResolvedValueOnce(jsonResponse(view))
      .mockResolvedValueOnce(jsonResponse({ token: "tok", action: { kind: "set", draft: { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: sha, documentB64: "eyJ4IjoxfQ" } }, previous: view.profiles[0], noOp: false, expiresInSeconds: 300 }));
    vi.stubGlobal("fetch", transport);
    await renderPanel({ deviceId: 3 });
    await flush();
    expect(textOf(document.body)).toContain("deviceConfig.current");
    await type('{"x":1}');
    await act(async () => { button("deviceConfig.preview").click(); });
    await flush();
    expect(textOf(document.body)).toContain("deviceConfig.confirmText");
  });

  it("refuses invalid JSON before any request", async () => {
    const emptyView = { ...view, profiles: [] };
    const transport = vi.fn().mockResolvedValue(jsonResponse(emptyView));
    vi.stubGlobal("fetch", transport);
    await renderPanel({ deviceId: 3 });
    for (let i = 0; i < 5; i++) await flush();
    expect(textOf(document.body)).toContain("deviceConfig.none");
    await type("not json");
    const btn = button("deviceConfig.preview");
    expect((btn as HTMLButtonElement).disabled).toBe(true); // module fields empty
    // Fill module identity so the button enables, then invalid JSON must refuse.
    const inputs = [...document.querySelectorAll("input")] as HTMLInputElement[];
    for (const [i, value] of ["smartsensor", "1.0.0", sha].entries()) {
      await act(async () => {
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
        setter.call(inputs[i], value);
        inputs[i].dispatchEvent(new Event("input", { bubbles: true }));
      });
    }
    await act(async () => { button("deviceConfig.preview").click(); });
    for (let i = 0; i < 10; i++) await flush();
    expect(textOf(document.body)).toContain("deviceConfig.invalidJson");
    expect(transport).toHaveBeenCalledTimes(1); // only the initial read
  });
});
