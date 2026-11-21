/** Row wiring for the checked parameter editor and refresh without unmounting rows. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterField, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { ParameterSourceEditorProps } from "./ParameterSourceEditor";
import { resetProductLanguageForTests } from "./productLanguage";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const apiMock = vi.hoisted(() => ({ deviceParameters: vi.fn(), setParameterValue: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)) }));
const editorProps = vi.hoisted(() => ({ last: null as ParameterSourceEditorProps | null }));
vi.mock("./ParameterSourceEditor", () => ({
  default: (props: ParameterSourceEditorProps) => { editorProps.last = props; return <div data-testid="checked-editor">checked:{props.field.etsId}</div>; },
}));

import ParameterPanel from "./ParameterPanel";

const tree = (revision: number): ProjectTree => ({
  schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  server_incarnation: "srv-r", project_incarnation: 5, snapshot_revision: revision,
  group_address_style: "ThreeLevel", installations: [],
});
function field(etsId: string, kind: string, extra: Partial<ParameterField> = {}): ParameterField {
  return { etsId, name: `Field ${etsId}`, nameLanguage: null, text: null, textLanguage: null, kind, value: "5", valueSource: "Stored",
    editable: true, min: "0", max: "10", enumOptions: [], displayOrder: null, access: null, writeEtsId: etsId, ...extra };
}
const panel: ParameterPanelDto = {
  programId: "PROG-1", sourceLanguage: null, stale: [], diagnostics: [], tree: null,
  sections: [
    { scope: null, fields: [field("T1", "Time"), field("N1", "Number"), field("R1", "Restriction", { min: null, max: null }), field("C1", "Color", { min: null, max: null })] },
    { scope: { moduleNode: 7, moduleId: "MD-1_MI-1", moduleDefId: "MD-1" }, fields: [field("T2", "Time")] },
  ],
};

let roots: Root[] = []; let host: HTMLDivElement;
afterEach(async () => {
  await act(async () => roots.forEach((r) => r.unmount())); roots = []; document.body.replaceChildren();
  vi.clearAllMocks(); editorProps.last = null; resetSettingsForTests(); resetProductLanguageForTests(); resetUiLanguageForTests();
});
type PanelProps = React.ComponentProps<typeof ParameterPanel>;
async function mount(props: Partial<PanelProps> = {}) {
  host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onValueApplied = vi.fn();
  const all: PanelProps = { deviceId: 3, refreshKey: tree(20), snapshot: { serverIncarnation: "srv-r", projectIncarnation: 5, snapshotRevision: 20 }, onValueApplied, ...props };
  await act(async () => root.render(<ParameterPanel {...all} />));
  return { root, all, onValueApplied };
}
const openButtons = () => Array.from(host.querySelectorAll("button")).filter((b) => b.textContent === "Checked editor");
function row(etsId: string) { return host.querySelector(`[data-ets-id="${etsId}"]`); }

describe("checked parameter editor in the parameter rows", () => {
  it("offers Date from the real candidate predicate and preserves the exact host context", async () => {
    const date = field("D1", "Date", { value: "1899-12-31", min: null, max: null, writeEtsId: "D-WRITE" });
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [date] }] });
    await mount();
    expect(openButtons().map(b => b.getAttribute("data-checked-for"))).toEqual(["D1"]);
    expect(row("D1")?.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("1899-12-31");
    await act(async () => openButtons()[0].click());
    expect(row("D1")?.querySelector("input")).toBeNull();
    expect(editorProps.last).toMatchObject({ field: date, deviceId: 3, scope: null, programId: "PROG-1",
      snapshot: { serverIncarnation: "srv-r", projectIncarnation: 5, snapshotRevision: 20 },
      writeBinding: { serverIncarnation: "srv-r", projectIncarnation: 5, snapshotRevision: 20 } });
    await act(async () => editorProps.last!.onClose());
    expect(row("D1")?.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("1899-12-31");
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
  });

  it("does not offer a Date checked editor without authoritative project identity", async () => {
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [field("D1", "Date", { min: null, max: null })] }] });
    await mount({ snapshot: null });
    expect(openButtons()).toHaveLength(0);
    // Absence of checked-editor evidence does not prohibit the existing plain field.
    expect(row("D1")?.querySelector<HTMLInputElement>("input")?.disabled).toBe(false);
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
  });

  it("offers it for Time, Number and Colour rows, not for choices", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel);
    await mount();
    expect(openButtons().map((b) => b.getAttribute("data-checked-for"))).toEqual(["T1", "N1", "C1", "T2"]);
    expect(openButtons()[0].title).toBe("Edit this value with the range and unit declared in the product data");
  });

  it("offers nothing without a complete project identity", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel);
    await mount({ snapshot: null });
    expect(openButtons()).toHaveLength(0);
  });

  it("offers no checked editor in the Manufacturer fields view without the project identity", async () => {
    // The Manufacturer fields tab shows only manufacturer-restricted rows;
    // without the tree's complete identity no checked editor is offered there
    // either (the same rule as the Parameters view). With identity, the row's
    // caption link is covered by the lastDownload tests below.
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [field("T9", "Time", { access: "Read", editable: false, writeEtsId: null })] }] });
    await mount({ view: "restricted", snapshot: null });
    expect(row("T9")).not.toBeNull();
    expect(openButtons()).toHaveLength(0);
  });

  it("offers the checked editor in the Manufacturer fields view with the identity, and links its caption (WP-37 S2)", async () => {
    const restricted = field("T9", "Time", { access: "Read", editable: false, writeEtsId: null });
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [restricted] }],
      lastDownload: { state: "usable", at: "2026-01-05T08:09:10.000Z", differing: [{ refId: "T9", downloaded: "4", current: "5" }] } });
    await mount({ view: "restricted" });
    expect(row("T9")).not.toBeNull();
    expect(openButtons().map(b => b.getAttribute("data-checked-for"))).toEqual(["T9"]);
    await act(async () => openButtons()[0].click());
    const note = host.querySelector<HTMLElement>(".parameter-download-note")!;
    expect(note.textContent).toBe("Value at last download: 4");
    expect(note.hidden).toBe(false);
    expect(editorProps.last!.describedBy).toBe(note.id);
    // German wording flows into the same caption.
    await act(async () => setSetting(UI_LANGUAGE_STORAGE_KEY, "de"));
    expect(host.querySelector(".parameter-download-note")!.textContent).toBe("Wert beim letzten Programmieren: 4");
    expect(editorProps.last!.describedBy).toBe(host.querySelector<HTMLElement>(".parameter-download-note")!.id);
  });

  it("replaces the plain input with the checked editor and hands it the exact context", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel);
    const { onValueApplied } = await mount();
    await act(async () => openButtons()[0].click());
    expect(row("T1")?.querySelector("input")).toBeNull();
    expect(host.textContent).toContain("checked:T1");
    const props = editorProps.last!;
    expect(props).toMatchObject({ deviceId: 3, scope: null, programId: "PROG-1", language: null, label: "Field T1",
      snapshot: { serverIncarnation: "srv-r", projectIncarnation: 5, snapshotRevision: 20 } });
    expect(props.field).toEqual(panel.sections[0].fields[0]);
    const written = { ...panel, tree: tree(21) };
    await act(async () => { props.onUpdated(written); props.onValueApplied(tree(21)); });
    expect(onValueApplied).toHaveBeenCalledWith(tree(21));
  });

  it("keeps the downloaded value visible beside the open editor until the write drops it (ADR-0173)", async () => {
    const date = field("D1", "Date", { value: "1899-12-31", min: null, max: null, writeEtsId: "D-WRITE" });
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [date] }],
      lastDownload: { state: "usable", at: "2026-01-05T08:09:10.000Z", differing: [{ refId: "D1", downloaded: "1899-12-30", current: "1899-12-31" }] } });
    await mount();
    await act(async () => openButtons()[0].click());
    expect(host.textContent).toContain("checked:D1");
    const note = () => host.querySelector<HTMLElement>(".parameter-download-note");
    expect(row("D1")!.getAttribute("data-differs-from-download")).toBe("true");
    expect(row("D1")!.nextElementSibling).toBe(note());
    expect(note()!.textContent).toBe("Value at last download: 1899-12-30");
    expect(note()!.querySelector(".parameter-download-mark")!.getAttribute("aria-hidden")).toBe("true");
    // The editor's write response carries no comparison; its ref becomes unknown.
    await act(async () => editorProps.last!.onUpdated({ ...panel, sections: [{ scope: null, fields: [date] }], tree: tree(21) }));
    expect(note()).toBeNull();
    // It was the only differing value: only the next read may say none differs.
    expect(host.querySelector(".parameter-last-download")).toBeNull();
  });

  it("links the caption beside the open editor as the editor controls' description (WP-37 S2)", async () => {
    const date = field("D1", "Date", { value: "1899-12-31", min: null, max: null, writeEtsId: "D-WRITE" });
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [date] }],
      lastDownload: { state: "usable", at: "2026-01-05T08:09:10.000Z", differing: [{ refId: "D1", downloaded: "1899-12-30", current: "1899-12-31" }] } });
    await mount();
    // Closed row: the plain control carries the description, the editor does not exist yet.
    expect(editorProps.last).toBeNull();
    await act(async () => openButtons()[0].click());
    const note = host.querySelector<HTMLElement>(".parameter-download-note")!;
    expect(note).not.toBeNull();
    expect(note.id).not.toBe("");
    // The caption stays always visible beside the open editor — never focus-only.
    expect(note.hidden).toBe(false);
    // The editor host received the caption's id, so its controls describe themselves with it.
    expect(editorProps.last!.describedBy).toBe(note.id);
    expect(document.getElementById(note.id)).toBe(note);
    // The write drops the ref: the caption and with it the link go at once.
    await act(async () => editorProps.last!.onUpdated({ ...panel, sections: [{ scope: null, fields: [date] }], tree: tree(21) }));
    expect(host.querySelector(".parameter-download-note")).toBeNull();
    expect(editorProps.last!.describedBy).toBeUndefined();
  });

  it("offers no description link when the row does not differ (WP-37 S2)", async () => {
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [field("T1", "Time")] }],
      lastDownload: { state: "usable", at: "2026-01-05T08:09:10.000Z", differing: [{ refId: "OTHER", downloaded: "1", current: "5" }] } });
    await mount();
    await act(async () => openButtons()[0].click());
    expect(host.querySelector(".parameter-download-note")).toBeNull();
    expect(editorProps.last!.describedBy).toBeUndefined();
  });

  it("shows nothing to link for an unusable comparison (WP-37 S2)", async () => {
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [field("T1", "Time")] }],
      lastDownload: { state: "otherProgram" } });
    await mount();
    await act(async () => openButtons()[0].click());
    expect(host.querySelector(".parameter-download-note")).toBeNull();
    expect(editorProps.last!.describedBy).toBeUndefined();
  });

  it("passes the module scope through for scoped rows", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel);
    await mount();
    await act(async () => openButtons()[3].click());
    expect(editorProps.last!.scope).toEqual({ moduleNode: 7, moduleId: "MD-1_MI-1", moduleDefId: "MD-1" });
  });

  it("returns to the plain input on close", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel);
    await mount();
    await act(async () => openButtons()[0].click());
    await act(async () => editorProps.last!.onClose());
    expect(row("T1")?.querySelector("input")).not.toBeNull();
    expect(host.textContent).not.toContain("checked:T1");
  });

  it("keeps rows and an open checked editor mounted while a new revision is read", async () => {
    let next!: (value: ParameterPanelDto) => void;
    apiMock.deviceParameters.mockResolvedValueOnce(panel).mockReturnValueOnce(new Promise((done) => { next = done; }));
    const { root, all } = await mount();
    await act(async () => openButtons()[0].click());
    await act(async () => root.render(<ParameterPanel {...all} refreshKey={tree(21)} snapshot={{ ...all.snapshot!, snapshotRevision: 21 }} />));
    expect(host.textContent).not.toContain("Loading parameters");
    expect(host.textContent).toContain("checked:T1");
    expect(editorProps.last!.snapshot.snapshotRevision).toBe(21);
    await act(async () => next(panel));
    expect(host.textContent).toContain("checked:T1");
  });

  it("clears a failed refresh once a later one succeeds", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel).mockRejectedValueOnce(new Error("Refresh refused")).mockResolvedValueOnce(panel);
    const { root, all } = await mount();
    await act(async () => root.render(<ParameterPanel {...all} refreshKey={tree(21)} />));
    expect(host.textContent).toContain("Refresh refused");
    await act(async () => root.render(<ParameterPanel {...all} refreshKey={tree(22)} />));
    expect(host.textContent).not.toContain("Refresh refused");
    expect(row("T1")).not.toBeNull();
  });
  it("still blanks the rows for another device", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel).mockReturnValueOnce(new Promise(() => {}));
    const { root, all } = await mount();
    await act(async () => root.render(<ParameterPanel {...all} deviceId={4} />));
    expect(host.textContent).toContain("Loading parameters");
    expect(row("T1")).toBeNull();
  });
});
