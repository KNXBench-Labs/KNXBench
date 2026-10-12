/** Parameter row host for the checked editor: source read, verification, writes and refetch. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ParameterField, ParameterPanel } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { SnapshotContext } from "./snapshotIdentity";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const apiMock = vi.hoisted(() => ({
  parameterEditorSource: vi.fn(),
  setParameterValue: vi.fn(),
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => apiMock);

import ParameterSourceEditor from "./ParameterSourceEditor";

const sha = "c".repeat(64);
const field: ParameterField = {
  etsId: "PR", name: "Delay", nameLanguage: null, text: "Delay", textLanguage: null, kind: "Time", value: "0010", valueSource: "stored",
  editable: true, min: "0", max: "3600", enumOptions: [], displayOrder: 1, access: "ReadWrite", writeEtsId: "PR",
};
const snapshot = (revision = 12): SnapshotContext => ({ serverIncarnation: "srv-p", projectIncarnation: 4, snapshotRevision: revision });
function answer(revision = 12, overrides: Record<string, unknown> = {}) {
  return {
    device: 41, parameterRefId: "PR", serverIncarnation: "srv-p", projectIncarnation: 4, snapshotRevision: revision,
    evidence: { status: "declared", source: { sourceSha256: sha, programId: "A", parameterTypeId: "PT", schema: 21, child: "TypeTime",
      attributes: { minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" }, parentAttributes: { Id: "PT" } } },
    ...overrides,
  };
}
const tree = { schema_version: 1, snapshot_revision: 13 } as unknown as ProjectTree;
const panel = { programId: "A", sourceLanguage: null, sections: [], stale: [], diagnostics: [], tree } as unknown as ParameterPanel;

let roots: Root[] = [];
beforeEach(() => { apiMock.parameterEditorSource.mockReset(); apiMock.setParameterValue.mockReset(); });
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren(); resetSettingsForTests(); resetUiLanguageForTests();
});

type Props = React.ComponentProps<typeof ParameterSourceEditor>;
async function mount(extra: Partial<Props> = {}) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const props: Props = { deviceId: 41, field, scope: null, programId: "A", snapshot: snapshot(), language: "de-DE", label: "Delay",
    onUpdated: vi.fn(), onValueApplied: vi.fn(), onClose: vi.fn(), ...extra };
  await act(async () => root.render(<ParameterSourceEditor {...props} />));
  return { host, root, props };
}
async function type(host: Element, value: string) {
  const input = host.querySelector<HTMLInputElement>('input[type="text"]'); expect(input).not.toBeNull();
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value); input!.dispatchEvent(new Event("input", { bubbles: true })); });
}
function button(host: Element, label: string) {
  const found = Array.from(host.querySelectorAll("button")).find((b) => b.textContent === label);
  expect(found, `missing button ${label}`).toBeDefined(); return found!;
}

describe("checked parameter editor host", () => {
  it("reads the source for the exact device and reference, then offers the time-count editor", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    const { host } = await mount();
    expect(apiMock.parameterEditorSource).toHaveBeenCalledWith(41, "PR");
    expect(host.textContent).toContain("Decimal count; no time conversion");
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("0010");
  });

  it("writes through setParameterValue and publishes the returned panel and tree", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    apiMock.setParameterValue.mockResolvedValue(panel);
    const { host, props } = await mount();
    await type(host, "12");
    await act(async () => button(host, "Apply value").click());
    expect(apiMock.setParameterValue).toHaveBeenCalledWith(41, "PR", "12", "de-DE");
    expect(props.onUpdated).toHaveBeenCalledWith(panel);
    expect(props.onValueApplied).toHaveBeenCalledWith(tree);
  });

  it("still publishes a write that finishes after the editor was closed", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    let settle!: (value: ParameterPanel) => void;
    apiMock.setParameterValue.mockReturnValue(new Promise((done) => { settle = done; }));
    const { host, root, props } = await mount();
    await type(host, "12");
    await act(async () => button(host, "Apply value").click());
    await act(async () => root.unmount()); roots = roots.filter((r) => r !== root);
    await act(async () => settle(panel));
    expect(props.onUpdated).toHaveBeenCalledWith(panel);
    expect(props.onValueApplied).toHaveBeenCalledWith(tree);
  });

  it("shows an unavailable reason next to the fallback, without an editor", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer(12, { evidence: { status: "unavailable", reason: "missingSource" } }));
    const { host } = await mount();
    expect(host.textContent).toContain("Exact source evidence is unavailable");
    expect(host.textContent).toContain("missingSource");
    expect(host.querySelector('input[type="text"]')).toBeNull();
  });

  it("refuses an answer for another device as evidence", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer(12, { device: 42 }));
    const { host } = await mount();
    expect(host.textContent).toContain("did not match this field");
    expect(host.querySelector('input[type="text"]')).toBeNull();
  });

  it("shows a failed read and returns to the plain field on request", async () => {
    apiMock.parameterEditorSource.mockRejectedValue(new Error("Project changed while reading parameter evidence"));
    const { host, props } = await mount();
    expect(host.textContent).toContain("Project changed while reading parameter evidence");
    await act(async () => button(host, "Back to the plain field").click());
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });

  it("re-reads on a new revision and keeps a draft across Save", async () => {
    apiMock.parameterEditorSource.mockImplementation(async () => answer(apiMock.parameterEditorSource.mock.calls.length === 1 ? 12 : 13));
    const { host, root, props } = await mount();
    await type(host, "12");
    await act(async () => root.render(<ParameterSourceEditor {...props} snapshot={snapshot(13)} />));
    expect(apiMock.parameterEditorSource).toHaveBeenCalledTimes(2);
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12");
    expect(host.textContent).not.toContain("draft is retained");
  });

  it("ignores a source answer that arrives after a newer request", async () => {
    let first!: (value: unknown) => void;
    apiMock.parameterEditorSource
      .mockReturnValueOnce(new Promise((done) => { first = done; }))
      .mockResolvedValueOnce(answer(13));
    const { host, root, props } = await mount();
    await act(async () => root.render(<ParameterSourceEditor {...props} snapshot={snapshot(13)} />));
    await act(async () => first({ nonsense: true }));
    expect(host.textContent).toContain("Decimal count; no time conversion");
    expect(host.textContent).not.toContain("did not match this field");
  });

  it("keeps colour values as an unconverted original, never a picker", async () => {
    const colour = { ...field, kind: "Color", value: "#00FF00", min: null, max: null };
    apiMock.parameterEditorSource.mockResolvedValue(answer(12, { evidence: { status: "declared", source: {
      sourceSha256: sha, programId: "A", parameterTypeId: "PT", schema: 21, child: "TypeColor", attributes: {}, parentAttributes: { Id: "PT" } } } }));
    const { host } = await mount({ field: colour });
    expect(host.textContent).toContain("This value format is not verified");
    expect(host.textContent).toContain("#00FF00");
    expect(host.querySelector('input[type="color"]')).toBeNull();
  });

  it("speaks German when the interface does", async () => {
    saveUiLanguage(settingsStorage, "de"); resetUiLanguageForTests();
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    const { host } = await mount();
    expect(host.textContent).toContain("Dezimaler Zähler");
    expect(host.textContent).toContain("Zurück zum einfachen Feld");
  });
});
