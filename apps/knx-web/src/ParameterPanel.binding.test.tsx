/** Goal B1: parameter writes bind to the shown snapshot and keep drafts on conflicts. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterField, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetProductLanguageForTests } from "./productLanguage";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
  setParameterValue: vi.fn(),
  currentProject: vi.fn(),
}));
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ParameterPanel from "./ParameterPanel";

const tree = (revision: number, project = 1): ProjectTree => ({
  schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", installations: [],
  server_incarnation: "srv-b", project_incarnation: project, snapshot_revision: revision,
});
const field = (value: string | null): ParameterField => ({
  etsId: "P1", name: "Field A", nameLanguage: null, text: null, textLanguage: null, kind: "Number",
  value, valueSource: "Stored", editable: true, min: "0", max: "10", enumOptions: [], displayOrder: null,
  access: null, writeEtsId: "P1",
});
const panel = (value: string | null, written: ProjectTree | null = null): ParameterPanelDto => ({
  programId: "PROG-1", sourceLanguage: null, sections: [{ scope: null, fields: [field(value)] }],
  stale: [], diagnostics: [], tree: written,
});
const binding = (revision: number, expectedValue: string | null) =>
  ({ serverIncarnation: "srv-b", projectIncarnation: 1, snapshotRevision: revision, expectedValue });
const refusal = (status: number, kind: string) =>
  Object.assign(new Error(`server says ${kind}`), { status, body: { error: `server says ${kind}`, kind } });

let host: HTMLDivElement | undefined;
let root: Root | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
  resetProductLanguageForTests();
});

async function mount(props: { deviceId?: number; refreshKey?: ProjectTree; onValueApplied?: (t: ProjectTree) => void }) {
  if (!host) {
    host = document.createElement("div");
    document.body.appendChild(host);
    root = createRoot(host);
  }
  await act(async () => root!.render(<ParameterPanel deviceId={props.deviceId ?? 1} refreshKey={props.refreshKey}
    onValueApplied={props.onValueApplied ?? (() => {})} />));
}
const input = () => host!.querySelector<HTMLInputElement>('input[type="number"]')!;
function type(value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input(), value);
  input().dispatchEvent(new Event("input", { bubbles: true }));
}
async function commit(value: string) {
  await act(async () => {
    type(value);
    input().dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
  });
}
const button = (label: string) => [...host!.querySelectorAll<HTMLButtonElement>(".parameter-conflict button")]
  .find((candidate) => candidate.textContent === label)!;

describe("revision-bound parameter writes (Goal B1)", () => {
  it("offers no write without a complete snapshot instead of sending an unbound one", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel("5"));
    await mount({});
    expect(input().disabled).toBe(true);
    expect(host!.textContent).toContain("no current project snapshot");
    await commit("6");
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
  });

  it("binds to the snapshot the shown panel was read under, not a newer tree still loading", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel("5")).mockReturnValueOnce(new Promise(() => {}));
    apiMock.setParameterValue.mockResolvedValue(panel("6", tree(4)));
    await mount({ refreshKey: tree(2) });
    await mount({ refreshKey: tree(3) });
    await commit("6");
    expect(apiMock.setParameterValue).toHaveBeenCalledExactlyOnceWith(1, "P1", "6", null, binding(2, "5"));
  });

  it("sends a received null value as an explicit null expectation", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(null));
    apiMock.setParameterValue.mockResolvedValue(panel("6", tree(3)));
    await mount({ refreshKey: tree(2) });
    await commit("6");
    expect(apiMock.setParameterValue).toHaveBeenCalledExactlyOnceWith(1, "P1", "6", null, binding(2, null));
  });

  it.each(["en", "de"] as const)("keeps the draft on a conflict, reloads, and resends only explicitly (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const reloaded = tree(5);
    const applied = vi.fn();
    apiMock.deviceParameters.mockResolvedValueOnce(panel("5")).mockResolvedValueOnce(panel("7"));
    apiMock.setParameterValue.mockRejectedValueOnce(refusal(409, "parameterWriteConflict"))
      .mockResolvedValueOnce(panel("6", tree(6)));
    apiMock.currentProject.mockResolvedValue(reloaded);
    await mount({ refreshKey: tree(2), onValueApplied: applied });
    await commit("6");
    expect(input().value).toBe("6");
    const alert = host!.querySelector('[role="alert"]')!.textContent;
    expect(alert).toContain(language === "de" ? "anderswo geändert" : "changed elsewhere");
    expect(alert).not.toContain("server says");
    expect(apiMock.currentProject).toHaveBeenCalledOnce();
    expect(applied).toHaveBeenCalledExactlyOnceWith(reloaded);
    // Until the newer panel has arrived, neither blur nor the button resends.
    expect(button(language === "de" ? "Meinen Wert übernehmen" : "Apply my value").disabled).toBe(true);
    await act(async () => input().dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(apiMock.setParameterValue).toHaveBeenCalledOnce();
    // The parent publishes the reloaded tree; the panel is read again under it.
    await mount({ refreshKey: reloaded, onValueApplied: applied });
    expect(input().value).toBe("6");
    expect(host!.querySelector(".parameter-conflict-current")!.textContent).toContain("7");
    // Each conflict answer loses one side (the reloaded value or the draft): both look destructive.
    const choices = Array.from(host!.querySelectorAll<HTMLButtonElement>(".parameter-conflict button"));
    expect(choices).toHaveLength(2);
    expect(choices.map((choice) => choice.classList.contains("danger-action"))).toEqual([true, true]);
    await act(async () => input().dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(apiMock.setParameterValue).toHaveBeenCalledOnce();
    await act(async () => button(language === "de" ? "Meinen Wert übernehmen" : "Apply my value").click());
    expect(apiMock.setParameterValue).toHaveBeenLastCalledWith(1, "P1", "6", null, binding(5, "7"));
    expect(host!.querySelector(".parameter-conflict")).toBeNull();
    expect(applied).toHaveBeenLastCalledWith(tree(6));
  });

  it("discards the retained draft back to the reloaded value without writing", async () => {
    const reloaded = tree(5);
    apiMock.deviceParameters.mockResolvedValueOnce(panel("5")).mockResolvedValueOnce(panel("7"));
    apiMock.setParameterValue.mockRejectedValueOnce(refusal(409, "parameterWriteConflict"));
    apiMock.currentProject.mockResolvedValue(reloaded);
    await mount({ refreshKey: tree(2) });
    await commit("6");
    await mount({ refreshKey: reloaded });
    expect(button("Discard my value").classList.contains("danger-action")).toBe(true);
    expect(button("Apply my value").classList.contains("danger-action")).toBe(true);
    await act(async () => button("Discard my value").click());
    expect(input().value).toBe("7");
    expect(host!.querySelector(".parameter-conflict")).toBeNull();
    expect(host!.querySelector(".field-error")).toBeNull();
    expect(apiMock.setParameterValue).toHaveBeenCalledOnce();
    // The row follows the project again after the conflict is closed.
    apiMock.deviceParameters.mockResolvedValueOnce(panel("8"));
    await mount({ refreshKey: tree(6) });
    expect(input().value).toBe("8");
  });

  it("names a failed reload while still keeping the draft", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel("5"));
    apiMock.setParameterValue.mockRejectedValueOnce(refusal(409, "parameterWriteConflict"));
    apiMock.currentProject.mockRejectedValue(new Error("offline"));
    await mount({ refreshKey: tree(2) });
    await commit("6");
    expect(input().value).toBe("6");
    expect(host!.querySelector(".field-error")!.textContent).toContain("reloading the current state failed: offline");
  });

  it.each([[409, "parameterWriteBusy"], [400, "parameterWriteConflict"], [422, "revisionBindingIncomplete"]])(
    "treats %i %s as an ordinary refusal: reverts, shows the server text, no reload", async (status, kind) => {
      apiMock.deviceParameters.mockResolvedValue(panel("5"));
      apiMock.setParameterValue.mockRejectedValueOnce(refusal(status, kind));
      await mount({ refreshKey: tree(2) });
      await commit("6");
      expect(input().value).toBe("5");
      expect(host!.querySelector(".field-error")!.textContent).toBe(`server says ${kind}`);
      expect(host!.querySelector(".parameter-conflict")).toBeNull();
      expect(apiMock.currentProject).not.toHaveBeenCalled();
    });

  it("does not show a late write reply for the previous device over the current one", async () => {
    let finish!: (value: ParameterPanelDto) => void;
    apiMock.deviceParameters.mockResolvedValueOnce(panel("5")).mockResolvedValueOnce(panel("2"));
    apiMock.setParameterValue.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    const applied = vi.fn();
    await mount({ deviceId: 1, refreshKey: tree(2), onValueApplied: applied });
    await commit("6");
    await mount({ deviceId: 2, refreshKey: tree(2), onValueApplied: applied });
    expect(input().value).toBe("2");
    await act(async () => finish(panel("6", tree(3))));
    expect(input().value).toBe("2");
    // The authoritative tree is still handed to the owner, whose guard orders trees.
    expect(applied).toHaveBeenCalledExactlyOnceWith(tree(3));
  });
});
