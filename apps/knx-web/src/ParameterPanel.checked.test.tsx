/** Row wiring for the checked parameter editor and refresh without unmounting rows. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterField, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { ParameterSourceEditorProps } from "./ParameterSourceEditor";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests } from "./settingsStore";

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

  it("offers nothing in the restricted view", async () => {
    apiMock.deviceParameters.mockResolvedValue({ ...panel, sections: [{ scope: null, fields: [field("T9", "Time", { access: "Read", editable: false, writeEtsId: null })] }] });
    await mount({ view: "restricted" });
    expect(row("T9")).not.toBeNull();
    expect(openButtons()).toHaveLength(0);
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
