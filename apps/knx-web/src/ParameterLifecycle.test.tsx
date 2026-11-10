/** Parameter save races settle per field and publish only for the current editor snapshot. */
// @vitest-environment happy-dom
import { act, StrictMode, Suspense, startTransition, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterPanel as Panel } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetSettingsForTests } from "./settingsStore";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ deviceParameters: vi.fn(), setParameterValue: vi.fn(), currentProject: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (e: unknown) => e instanceof Error ? e.message : String(e) }));
import ParameterPanel, { ParameterPanelContent, useDeviceParameters } from "./ParameterPanel";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function tree(revision: number): ProjectTree {
  return { schema_version: 13, errors: 0, warnings: 0, can_undo: revision > 1, can_redo: false,
    is_modified: revision > 1, group_address_style: "ThreeLevel", installations: [],
    server_incarnation: "parameter-races", project_incarnation: 1, snapshot_revision: revision };
}
function panel(revision: number, a = "5", b = "10"): Panel {
  return { programId: "PROG", sourceLanguage: null, tree: tree(revision), stale: [], diagnostics: [],
    sections: [{ scope: null, fields: [["A", a], ["B", b]].map(([etsId, value]) => ({
      etsId, value, name: etsId, text: etsId, textLanguage: null, nameLanguage: null, kind: "Number",
      valueSource: "Stored", editable: true, min: "0", max: "100", enumOptions: [], displayOrder: null,
      access: "ReadWrite", writeEtsId: etsId,
    })) }] };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
let host: HTMLDivElement;
let root: Root | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined; host?.remove(); vi.resetAllMocks();
  resetSettingsForTests(); resetProductLanguageForTests(); resetUiLanguageForTests();
});
async function mount(onApplied = vi.fn(), strict = false) {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  function Editor() {
    const [snapshot, setSnapshot] = useState(tree(1));
    return <ParameterPanel deviceId={1} refreshKey={snapshot} onValueApplied={next => {
      onApplied(next);
      // App.publishTree rejects an older response instead of refetching on it.
      setSnapshot(previous => (next.snapshot_revision ?? 0) >= (previous.snapshot_revision ?? 0) ? next : previous);
    }} />;
  }
  await act(async () => root!.render(strict ? <StrictMode><Editor /></StrictMode> : <Editor />));
  return onApplied;
}
function input(id: string) { return host.querySelector<HTMLInputElement>(`[data-ets-id="${id}"] input`)!; }
async function edit(id: string, value: string) {
  await act(async () => {
    const element = input(id); element.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => input(id).blur());
}

describe("parameter editor lifecycle", () => {
  it("releases each of two overlapping fields independently after success and rejection, then permits retry", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(1));
    const a = deferred<Panel>(), b = deferred<Panel>();
    apiMock.setParameterValue.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise).mockResolvedValue(panel(3, "6", "12"));
    await mount();
    await edit("A", "6"); await edit("B", "12");
    expect(input("A").disabled).toBe(true); expect(input("B").disabled).toBe(true);
    apiMock.deviceParameters.mockResolvedValue(panel(2, "6"));
    await act(async () => a.resolve(panel(2, "6")));
    expect(input("A").disabled).toBe(false); expect(input("B").disabled).toBe(true);
    await act(async () => b.reject(new Error("write refused")));
    expect(input("B").disabled).toBe(false); expect(host.textContent).toContain("write refused");
    apiMock.deviceParameters.mockResolvedValue(panel(3, "6", "12"));
    await edit("B", "12");
    expect(input("A").disabled).toBe(false); expect(input("B").disabled).toBe(false);
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(3);
    expect(input("B").value).toBe("12");
  });

  it("does not let an older successful POST overwrite a newer accepted panel", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(1));
    const a = deferred<Panel>(), b = deferred<Panel>();
    const conflict = Object.assign(new Error("snapshot changed"), { status: 409, body: { kind: "parameterWriteConflict" } });
    apiMock.setParameterValue.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise).mockResolvedValue(panel(3, "6", "12"));
    await mount(); await edit("A", "6"); await edit("B", "12");
    // A committed revision 2 but its HTTP answer is delayed. B's original
    // revision-1 write is refused; the explicit retry reads revision 2 first.
    apiMock.currentProject.mockResolvedValue(tree(2));
    apiMock.deviceParameters.mockResolvedValue(panel(2, "6"));
    await act(async () => b.reject(conflict));
    expect(input("A").disabled).toBe(true); expect(input("B").disabled).toBe(false);
    const retry = host.querySelector<HTMLButtonElement>('[data-conflict-for="B"] button')!;
    expect(retry.disabled).toBe(false);
    apiMock.deviceParameters.mockResolvedValue(panel(3, "6", "12"));
    await act(async () => retry.click());
    expect(apiMock.setParameterValue).toHaveBeenNthCalledWith(3, 1, "B", "12", null,
      { serverIncarnation: "parameter-races", projectIncarnation: 1, snapshotRevision: 2, expectedValue: "10" });
    await act(async () => a.resolve(panel(2, "6", "10")));
    expect(input("A").disabled).toBe(false); expect(input("B").disabled).toBe(false);
    expect(input("B").value).toBe("12");
  });

  it("forwards a committed tree to its owner without updating a same-device replacement row", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(1));
    const pending = deferred<Panel>(); apiMock.setParameterValue.mockReturnValue(pending.promise);
    const onApplied = await mount(); await edit("A", "6");
    await act(async () => root!.render(<ParameterPanel key="replacement" deviceId={1} refreshKey={tree(3)} onValueApplied={onApplied} />));
    await act(async () => pending.resolve(panel(2, "6")));
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree(2));
    expect(input("A").value).toBe("5");
    expect(input("A").disabled).toBe(false);
  });

  it("independently fences a pre-write GET without a parent tree refresh", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(1));
    apiMock.setParameterValue.mockResolvedValue(panel(2, "6"));
    host = document.createElement("div"); document.body.append(host); root = createRoot(host);
    const publish = vi.fn();
    await act(async () => root!.render(<ParameterPanel deviceId={1} refreshKey={tree(1)} onValueApplied={publish} />));
    const oldRead = deferred<Panel>(); apiMock.deviceParameters.mockReturnValueOnce(oldRead.promise);
    // Re-read the same revision before A commits; the owner intentionally
    // does not publish the returned tree, isolating the read fence itself.
    await act(async () => root!.render(<ParameterPanel deviceId={1} refreshKey={tree(1)} onValueApplied={publish} />));
    await edit("A", "6");
    expect(publish).toHaveBeenCalledExactlyOnceWith(tree(2));
    await act(async () => oldRead.resolve(panel(1)));
    expect(input("A").value).toBe("6"); expect(input("B").value).toBe("10");
    expect(input("A").disabled).toBe(false); expect(input("B").disabled).toBe(false);
  });

  it("does not publish a conflict reload after its editor was retired", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(1));
    const reload = deferred<ProjectTree>(); apiMock.currentProject.mockReturnValue(reload.promise);
    apiMock.setParameterValue.mockRejectedValue(Object.assign(new Error("stale"),
      { status: 409, body: { kind: "parameterWriteConflict" } }));
    const onApplied = await mount(); await edit("A", "6");
    expect(apiMock.currentProject).toHaveBeenCalledTimes(1);
    await act(async () => root!.render(<ParameterPanel key="replacement" deviceId={1} refreshKey={tree(3)} onValueApplied={onApplied} />));
    await act(async () => reload.resolve(tree(2)));
    expect(onApplied).not.toHaveBeenCalled();
    expect(input("A").disabled).toBe(false);
  });

  it("does not retire a committed read because a different-device render suspended", async () => {
    const read = deferred<Panel>(), suspended = deferred<void>();
    apiMock.deviceParameters.mockReturnValueOnce(read.promise).mockResolvedValue(panel(1));
    const snapshot = tree(1);
    function Editor({ paused }: { paused: boolean }) {
      const state = useDeviceParameters(paused ? 2 : 1, snapshot);
      if (paused) throw suspended.promise;
      return <ParameterPanelContent deviceId={1} state={state} refreshKey={snapshot} onValueApplied={() => {}} />;
    }
    host = document.createElement("div"); document.body.append(host); root = createRoot(host);
    await act(async () => root!.render(<Suspense fallback={<p>Fallback</p>}><Editor paused={false} /></Suspense>));
    await act(async () => startTransition(() => {
      root!.render(<Suspense fallback={<p>Fallback</p>}><Editor paused /></Suspense>);
    }));
    expect(host.textContent).not.toContain("Fallback");
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(1);
    await act(async () => read.resolve(panel(1)));
    expect(input("A")).not.toBeNull();
    expect(input("A").value).toBe("5");
  });

  it("loads normally in StrictMode and ignores a read retired by effect replay", async () => {
    const old = deferred<Panel>();
    apiMock.deviceParameters.mockReturnValueOnce(old.promise).mockResolvedValue(panel(1));
    await mount(vi.fn(), true);
    expect(input("A").value).toBe("5");
    await act(async () => old.resolve(panel(1, "99")));
    expect(input("A").value).toBe("5");
  });
});
