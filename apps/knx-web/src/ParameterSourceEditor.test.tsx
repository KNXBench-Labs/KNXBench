/** Parameter row host for the checked editor: source read, verification, writes and refetch. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { flushSync } from "react-dom";
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
import { parameterEditorCopy } from "./features/parameter-editors/copy";

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
  const props: Props = { deviceId: 41, field, scope: null, programId: "A", snapshot: snapshot(), writeBinding: snapshot(11),
    onConflict: vi.fn().mockResolvedValue(undefined), language: "de-DE", label: "Delay",
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
  const date: ParameterField = { ...field, kind: "Date", value: "1899-12-31", min: null, max: null, writeEtsId: "DATE-WRITE" };
  const dateAnswer = (revision = 12) => ({ ...answer(revision), evidence: { status: "declared", source: {
    sourceSha256: sha, programId: "A", parameterTypeId: "PT", schema: 21, child: "TypeDate",
    attributes: { Encoding: "DPT 11" } as Record<string, string>, parentAttributes: { Id: "PT", Name: "Reference date" },
  } } });

  it.each(["en", "de"] as const)("%s Date joins real source projection, leaf and revision-bound mutation adapter", async locale => {
    saveUiLanguage(settingsStorage, locale); resetUiLanguageForTests();
    const copy = parameterEditorCopy[locale];
    apiMock.parameterEditorSource.mockResolvedValue(dateAnswer());
    apiMock.setParameterValue.mockResolvedValue(panel);
    const { host, props } = await mount({ field: date, label: "Reference date" });
    expect(apiMock.parameterEditorSource).toHaveBeenCalledExactlyOnceWith(41, "PR");
    expect(host.textContent).toContain(copy.dateFormat);
    expect(host.querySelector<HTMLInputElement>("input")?.inputMode).toBe("text");
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
    await type(host, "1900-02-29"); await act(async () => button(host, copy.apply).click());
    expect(host.querySelector('[role="alert"]')?.textContent).toBe(copy.date);
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
    await type(host, "0000-02-29");
    const input = host.querySelector<HTMLInputElement>("input")!;
    await act(async () => { input.focus(); input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); input.blur(); });
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
    await act(async () => { button(host, copy.apply).click(); button(host, copy.apply).click(); });
    expect(apiMock.setParameterValue).toHaveBeenCalledExactlyOnceWith(41, "DATE-WRITE", "0000-02-29", "de-DE", { ...snapshot(11), expectedValue: "1899-12-31" });
    expect(props.onUpdated).toHaveBeenCalledExactlyOnceWith(panel);
    expect(props.onValueApplied).toHaveBeenCalledExactlyOnceWith(tree);
  });

  it("keeps a Date service refusal recoverable without publishing or repairing its draft", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(dateAnswer());
    apiMock.setParameterValue.mockRejectedValueOnce(new Error("Authoritative date refusal")).mockResolvedValueOnce(panel);
    const { host, props } = await mount({ field: date }); await type(host, "9999-12-31");
    await act(async () => button(host, "Apply value").click());
    expect(host.textContent).toContain("Authoritative date refusal");
    expect(host.querySelector<HTMLInputElement>("input")?.value).toBe("9999-12-31");
    expect(props.onUpdated).not.toHaveBeenCalled(); expect(props.onValueApplied).not.toHaveBeenCalled();
    await act(async () => button(host, "Apply value").click());
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(2);
    expect(props.onUpdated).toHaveBeenCalledExactlyOnceWith(panel); expect(props.onValueApplied).toHaveBeenCalledExactlyOnceWith(tree);
  });

  it("reloads a Date conflict and requires reset before a freshly bound explicit Apply", async () => {
    apiMock.parameterEditorSource.mockResolvedValueOnce(dateAnswer()).mockResolvedValueOnce(dateAnswer(13));
    apiMock.setParameterValue.mockRejectedValueOnce(Object.assign(new Error("conflict"),
      { status: 409, body: { error: "conflict", kind: "parameterWriteConflict" } })).mockResolvedValueOnce(panel);
    const { host, props, root } = await mount({ field: date }); await type(host, "2000-02-29");
    await act(async () => button(host, "Apply value").click());
    expect(props.onConflict).toHaveBeenCalledOnce(); expect(props.onUpdated).not.toHaveBeenCalled();
    const changed = { ...props, field: { ...date, value: "0001-01-01" }, snapshot: snapshot(13), writeBinding: snapshot(13) };
    await act(async () => root.render(<ParameterSourceEditor {...changed} />));
    expect(host.querySelector<HTMLInputElement>("input")?.value).toBe("2000-02-29");
    expect(host.textContent).toContain(parameterEditorCopy.en.stale);
    await act(async () => button(host, "Apply value").click()); expect(apiMock.setParameterValue).toHaveBeenCalledTimes(1);
    await act(async () => button(host, "Discard draft and reload").click());
    expect(host.querySelector<HTMLInputElement>("input")?.value).toBe("0001-01-01");
    await type(host, "2000-02-29"); await act(async () => button(host, "Apply value").click());
    expect(apiMock.setParameterValue).toHaveBeenLastCalledWith(41, "DATE-WRITE", "2000-02-29", "de-DE", { ...snapshot(13), expectedValue: "0001-01-01" });
    expect(props.onUpdated).toHaveBeenCalledExactlyOnceWith(panel);
  });

  it.each(["mismatch", "qualified-encoding", "module"] as const)("Date %s keeps specialized fallback and makes no mutation", async mode => {
    const body = dateAnswer();
    if (mode === "mismatch") body.device = 42;
    if (mode === "qualified-encoding") body.evidence.source.attributes = { "v:Encoding": "DPT 11" };
    apiMock.parameterEditorSource.mockResolvedValue(body);
    const { host } = await mount({ field: date, scope: mode === "module" ? { moduleNode: 7, moduleId: "MD-1_MI-1", moduleDefId: "MD-1" } : null });
    expect(host.querySelector("input")).toBeNull();
    expect(host.textContent).toContain(parameterEditorCopy.en.fallback[mode === "module" ? "module-scope" : mode === "qualified-encoding" ? "unsupported-hints" : "missing-evidence"]);
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
  });

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
    // Bound to the panel's own snapshot and the field value it showed (Goal B1).
    expect(apiMock.setParameterValue).toHaveBeenCalledWith(41, "PR", "12", "de-DE", { ...snapshot(11), expectedValue: "0010" });
    expect(props.onUpdated).toHaveBeenCalledWith(panel);
    expect(props.onValueApplied).toHaveBeenCalledWith(tree);
  });

  it.each(["en", "de"] as const)("%s acknowledges its exact response even when the host publishes before Apply resolves", async locale => {
    saveUiLanguage(settingsStorage, locale); resetUiLanguageForTests();
    const copy = parameterEditorCopy[locale];
    const nextField = { ...field, value: "12" };
    const nextTree = { ...tree, server_incarnation: "srv-p", project_incarnation: 4, snapshot_revision: 13 };
    const returned: ParameterPanel = { ...panel, sections: [{ scope: null, fields: [nextField] }], tree: nextTree };
    apiMock.parameterEditorSource.mockResolvedValueOnce(answer()).mockResolvedValue(answer(13));
    apiMock.setParameterValue.mockResolvedValue(returned);
    let view!: Awaited<ReturnType<typeof mount>>;
    const publish = vi.fn((actual: ProjectTree) => {
      expect(actual).toEqual(nextTree);
      // Force the real host/leaf join to render the committed revision while its Apply promise is still pending.
      flushSync(() => view.root.render(<ParameterSourceEditor {...view.props} field={nextField} snapshot={snapshot(13)} writeBinding={snapshot(13)} />));
      expect(view.host.querySelector<HTMLInputElement>('input[type="text"]')?.disabled).toBe(true);
      expect(view.host.textContent).toContain(copy.stale);
    });
    view = await mount({ writeBinding: snapshot(), onValueApplied: publish });
    await type(view.host, "12"); await act(async () => button(view.host, copy.apply).click());
    expect(view.props.onUpdated).toHaveBeenCalledExactlyOnceWith(returned);
    expect(publish).toHaveBeenCalledExactlyOnceWith(nextTree);
    expect(apiMock.setParameterValue).toHaveBeenCalledExactlyOnceWith(41, "PR", "12", "de-DE", { ...snapshot(), expectedValue: "0010" });
    expect(view.host.textContent).toContain(copy.settled);
    expect(view.host.textContent).not.toContain(copy.stale);
    expect(view.host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12");
    expect(view.host.querySelector<HTMLInputElement>('input[type="text"]')?.disabled).toBe(false);
  });
  it("keeps the draft on a stale-write conflict, reloads once and publishes nothing", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    apiMock.setParameterValue.mockRejectedValueOnce(Object.assign(new Error("raw server text"),
      { status: 409, body: { error: "raw server text", kind: "parameterWriteConflict" } }));
    const { host, props } = await mount();
    await type(host, "12");
    await act(async () => button(host, "Apply value").click());
    expect(props.onConflict).toHaveBeenCalledOnce();
    expect(props.onUpdated).not.toHaveBeenCalled();
    expect(props.onValueApplied).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12");
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("changed elsewhere");
  });

  it("names a failed reload after a conflict", async () => {
    apiMock.parameterEditorSource.mockResolvedValue(answer());
    apiMock.setParameterValue.mockRejectedValueOnce(Object.assign(new Error("raw"), { status: 409, body: { error: "raw", kind: "parameterWriteConflict" } }));
    const { host } = await mount({ onConflict: vi.fn().mockRejectedValue(new Error("offline")) });
    await type(host, "12");
    await act(async () => button(host, "Apply value").click());
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("reloading the current state failed: offline");
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
