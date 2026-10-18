/** Parameter navigation retains editors and never changes engineering data. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { ParameterField, ParameterGroupLayout, ParameterSection, ParameterSeparator } from "./api";
import ParameterWorkspace from "./ParameterWorkspace";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

let host: HTMLDivElement;
let root: Root;
afterEach(async () => { if (root) await act(async () => root.unmount()); host?.remove(); resetSettingsForTests(); resetUiLanguageForTests(); });
const field = (etsId: string, key?: string): ParameterField => ({
  etsId, group: key ? { key, label: key } : null, name: etsId, text: etsId, nameLanguage: null,
  textLanguage: null, kind: "Number", value: "5", valueSource: "Stored", editable: true,
  min: "0", max: "10", enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId,
});
function DraftField({ field: item }: { field: ParameterField }) {
  const [value, setValue] = useState(item.value ?? "");
  return <label className="parameter-field">{item.text}<input type="number" value={value}
    onChange={event => setValue(event.target.value)} /></label>;
}
const renderField = (item: ParameterField) => <DraftField field={item} />;
const sections: ParameterSection[] = [{ scope: null, fields: [field("Mode", "Buttons"), field("Delay", "LEDs"), field("Other")] }];
async function mount(data = sections) {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root.render(<ParameterWorkspace sections={data} renderField={renderField} />));
}
function setInput(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}
function nav(name: string) {
  return [...host.querySelectorAll<HTMLButtonElement>(".parameter-workspace-nav button")].find(button => button.textContent?.includes(name))!;
}

describe("ParameterWorkspace", () => {
  it.each([["en", "Parameter group 3"], ["de", "Parametergruppe 3"]])(
    "%s searches the visible numbered caption and retains source context and editor identity", async (language, fallback) => {
      saveUiLanguage(settingsStorage, language); resetUiLanguageForTests();
      const data: ParameterSection[] = [{ scope: null, fields: [field("First", "One"), field("Second", "Two"),
        { ...field("Target"), group: { key: "PB-3", label: "#", labelFill: { template: "Source {{0}}", source: "EmptyValue" } } }] }];
      const before = structuredClone(data);
      await mount(data);
      const button = nav(fallback);
      expect(button).toBeDefined();
      expect(button.title).toContain("Source {{0}}");
      await act(async () => button.click());
      const row = host.querySelector<HTMLDivElement>('[data-parameter-row="Target"]')!;
      const input = row.querySelector<HTMLInputElement>("input")!;
      await act(async () => setInput(input, "7"));
      const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
      for (const query of [fallback, "#", "Source {{0}}", "Target"]) {
        await act(async () => setInput(search, query));
        expect(row.hidden, `search ${query}`).toBe(false);
        expect(host.querySelectorAll(".parameter-workspace-row:not([hidden])")).toHaveLength(1);
        expect(row.querySelector(".parameter-workspace-result-path")!.textContent).toContain(fallback);
        expect(host.querySelector('[data-parameter-row="Target"]')).toBe(row);
        expect(row.querySelector("input")).toBe(input);
        expect(input.value).toBe("7");
      }
      await act(async () => setInput(search, ""));
      expect(button.getAttribute("aria-current")).toBe("page");
      expect(data).toEqual(before);
    });

  it("uses the same safe captions in separator rows and declared grid captions", async () => {
    const data: ParameterSection[] = [{ scope: null, fields: [field("P", "Area")],
      separators: [{ position: 0, group: { key: "Area", label: "Area" }, elementId: "S", text: "{{0}}",
        uiHint: "Headline", access: null, blockAccess: null }] }];
    await mount(data);
    await act(async () => nav("Area").click());
    expect(host.querySelector(".parameter-workspace-separator h5")!.textContent).toBe("Section text");
    expect(host.querySelector<HTMLElement>(".parameter-workspace-separator")!.title).toContain("{{0}}");
  });

  it("uses readable section names and source tooltips instead of raw templates or hash labels", async () => {
    const labels = ["Functions: {{0}}", "#", "", "   ", "{{9:unknown}}"];
    const data: ParameterSection[] = [{ scope: null, fields: labels.map((label, index) => ({
      ...field(`P-${index}`), group: { key: `PB-${index}`, label },
    })) }];
    const before = structuredClone(data);
    await mount(data);
    const buttons = [...host.querySelectorAll<HTMLButtonElement>(".parameter-workspace-nav button")];
    expect(buttons).toHaveLength(5);
    for (const [index, button] of buttons.entries()) {
      expect(button.querySelector("span")!.textContent).toBe(`Parameter group ${index + 1}`);
      expect(button.title).toContain("caption");
      if (labels[index].trim()) expect(button.title).toContain(labels[index]);
    }
    await act(async () => buttons[0].click());
    expect(host.querySelector(".parameter-workspace-heading h4")!.textContent).toBe("Parameter group 1");
    expect(data).toEqual(before);
  });

  it("starts closed: every area is listed with its count, but none opens until chosen", async () => {
    await mount();
    expect(host.querySelectorAll('.parameter-workspace-nav [aria-current="page"]')).toHaveLength(0);
    expect(nav("Buttons").textContent).toContain("1");
    expect(host.querySelectorAll(".parameter-workspace-row:not([hidden])")).toHaveLength(0);
    expect(host.querySelector(".parameter-workspace-columns")).toBeNull();
    expect(host.querySelector(".parameter-workspace-closed")!.textContent).toContain("Choose an area");
    expect(host.querySelector("details.parameter-section")).toBeNull();
  });

  it("shows a chosen area as compact aligned rows instead of stacked accordions", async () => {
    await mount();
    await act(async () => nav("Buttons").click());
    expect(nav("Buttons").getAttribute("aria-current")).toBe("page");
    expect(host.querySelector('[data-parameter-row="Mode"]')!.closest("[hidden]")).toBeNull();
    expect(host.querySelector('[data-parameter-row="Delay"]')!.closest("[hidden]")).not.toBeNull();
    expect(host.querySelector(".parameter-workspace-columns")!.textContent).toContain("Value");
    expect(host.querySelector(".parameter-workspace-closed")).toBeNull();
  });

  it("keeps an explicitly opened area across refreshes while newly appearing areas stay closed", async () => {
    await mount();
    await act(async () => nav("LEDs").click());
    await act(async () => root.render(<ParameterWorkspace sections={[{ scope: null,
      fields: [...sections[0].fields, field("Fresh", "New group")] }]} renderField={renderField} />));
    expect(nav("LEDs").getAttribute("aria-current")).toBe("page");
    expect(nav("New group").getAttribute("aria-current")).toBeNull();
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Fresh"]')!.hidden).toBe(true);
  });

  it("retains the same draft editor across navigation and search, including no-result searches", async () => {
    await mount();
    await act(async () => nav("Buttons").click());
    const input = host.querySelector<HTMLInputElement>('[data-parameter-row="Mode"] input')!;
    await act(async () => setInput(input, "7"));
    await act(async () => nav("LEDs").click());
    await act(async () => nav("Buttons").click());
    const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => setInput(search, "absent"));
    expect(host.textContent).toContain("No parameters match");
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Mode"]')!.hidden).toBe(true);
    await act(async () => setInput(search, ""));
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Mode"]')!.hidden).toBe(false);
    expect(host.querySelector('[data-parameter-row="Mode"] input')).toBe(input);
    expect(input.value).toBe("7");
  });

  it("searches every source group and shows field metadata without inventing a description", async () => {
    await mount();
    const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => setInput(search, "Delay"));
    const row = host.querySelector<HTMLDivElement>('[data-parameter-row="Delay"]')!;
    expect(row.hidden).toBe(false);
    await act(async () => row.querySelector("input")!.focus());
    const info = host.querySelector(".parameter-workspace-context")!;
    expect(info.textContent).toContain("Delay");
    expect(info.textContent).toContain("ReadWrite");
    expect(info.textContent).toContain("0");
    expect(info.textContent).toContain("10");
    expect(info.textContent).not.toContain("Description");
  });

  it("keeps display-only headings and blank spacers free of information controls", async () => {
    await mount([{ scope: null, fields: [{ ...field("heading"), kind: "None", text: "Timing" },
      { ...field("spacer"), kind: "None", text: "", name: "d_space" }] }]);
    expect(host.querySelector('[data-parameter-row="heading"] .parameter-workspace-info')).toBeNull();
    expect(host.querySelector('[data-parameter-row="spacer"] .parameter-workspace-info')).toBeNull();
    expect(host.querySelector('[data-parameter-row="spacer"]')!.getAttribute("data-display-only")).toBe("true");
  });

  it("returns to the closed overview when the chosen area disappears, opening nothing else", async () => {
    await mount();
    await act(async () => nav("LEDs").click());
    await act(async () => root.render(<ParameterWorkspace sections={[{ scope: null, fields: [field("Mode", "Buttons")] }]} renderField={renderField} />));
    expect(host.querySelectorAll('.parameter-workspace-nav [aria-current="page"]')).toHaveLength(0);
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Mode"]')!.hidden).toBe(true);
    expect(host.querySelector(".parameter-workspace-closed")).not.toBeNull();
  });
});

describe("ParameterWorkspace declared grids", () => {
  const grid = (patch: Partial<ParameterGroupLayout> = {}): ParameterGroupLayout => ({
    groupKey: "Pairs", version: 1, declaredLayout: "Table", status: "grid",
    rows: [{ id: "R-1", text: "First pair", textParameterRefId: null, collapseIfEmpty: null },
      { id: "R-2", text: "Second pair", textParameterRefId: null, collapseIfEmpty: null }],
    columns: [{ id: "C-1", text: "Left", textParameterRefId: null, width: "50%" },
      { id: "C-2", text: "Right", textParameterRefId: null, width: "50%" }],
    labels: [{ row: 2, column: 1, text: "Fictional caption" }], issues: [], ...patch,
  });
  const placed = (etsId: string, row: number, column: number): ParameterField => ({ ...field(etsId, "Pairs"), cell: { row, column } });
  const gridSections = (layout: ParameterGroupLayout | null = grid()): ParameterSection[] => [{ scope: null,
    fields: [placed("L1", 1, 1), placed("R1", 1, 2), placed("R2", 2, 2), { ...field("Loose", "Pairs"), cell: null }, field("Delay", "LEDs")],
    groupLayouts: layout ? [layout] : [] }];
  const rowsParent = () => host.querySelector<HTMLDivElement>(".parameter-workspace-rows")!;
  const style = (etsId: string) => host.querySelector<HTMLDivElement>(`[data-parameter-row="${etsId}"]`)!.style;

  it("safely labels unresolved separator captions in a declared grid", async () => {
    await mount(gridSections(grid({ labels: [{ row: 1, column: 1, text: "{{0}}",
      textFill: { template: "{{0}}", source: "NoReference" } }] })));
    await act(async () => nav("Pairs").click());
    const caption = host.querySelector<HTMLElement>('.parameter-matrix-caption[data-caption="label"]')!;
    expect(caption.textContent).toBe("Section text");
    expect(caption.title).toContain("{{0}}");
  });

  it("starts closed and renders a matrix only after an area with a declared grid is chosen", async () => {
    await mount(gridSections());
    expect(rowsParent().getAttribute("data-layout")).toBeNull();
    await act(async () => nav("Pairs").click());
    expect(rowsParent().getAttribute("data-layout")).toBe("matrix");
    expect(rowsParent().style.gridTemplateColumns).toContain("50fr");
    expect(style("L1").gridRow).toBe("2 / span 1");
    expect(style("L1").gridColumn).toBe("2 / span 1");
    expect(style("R2").gridRow).toBe("3 / span 1");
    expect(style("R2").gridColumn).toBe("3 / span 1");
    expect(style("Loose").gridColumn).toBe("1 / -1");
    const captions = [...host.querySelectorAll(".parameter-matrix-caption")].map(node => node.textContent);
    expect(captions).toEqual(["Left", "Right", "First pair", "Second pair", "Fictional caption"]);
    expect(host.querySelector(".parameter-matrix-unplaced")!.textContent).toContain("Not placed");
    expect(host.querySelector(".parameter-workspace-layout-note")!.textContent).toContain("Table");
    expect(host.querySelector(".parameter-workspace-columns")).toBeNull();
  });

  it("keeps the row view with an explicit hint when no grid is declared", async () => {
    await mount(gridSections());
    await act(async () => nav("LEDs").click());
    expect(rowsParent().getAttribute("data-layout")).toBeNull();
    expect(style("Delay").gridRow).toBe("");
    expect(host.querySelector(".parameter-workspace-layout-note")!.textContent).toContain("No grid is declared");
    expect(host.querySelector(".parameter-matrix-caption")).toBeNull();
  });

  it("reports an unknown layout variant verbatim and never renders it as a matrix", async () => {
    await mount(gridSections(grid({ status: "unknownLayout", declaredLayout: "Carousel" })));
    await act(async () => nav("Pairs").click());
    expect(rowsParent().getAttribute("data-layout")).toBeNull();
    expect(host.querySelector(".parameter-workspace-layout-note")!.textContent).toContain("Carousel");
    expect(style("L1").gridRow).toBe("");
  });

  it("names declarations that could not be placed in a closed disclosure", async () => {
    await mount(gridSections(grid({ issues: [{ kind: "ambiguousCell", element: "ParameterRefRef", reference: "Loose", cell: null },
      { kind: "futureKind", element: "Widget", reference: null, cell: "1,1" }] })));
    await act(async () => nav("Pairs").click());
    const details = host.querySelector<HTMLDetailsElement>(".parameter-matrix-issues")!;
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("2 declarations could not be placed");
    expect(details.textContent).toContain("Loose");
    expect(details.textContent).toContain("futureKind");
  });

  it("keeps the same draft editors across matrix, row view and search", async () => {
    await mount(gridSections());
    await act(async () => nav("Pairs").click());
    const input = host.querySelector<HTMLInputElement>('[data-parameter-row="R2"] input')!;
    await act(async () => setInput(input, "9"));
    await act(async () => nav("LEDs").click());
    await act(async () => nav("Pairs").click());
    const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => setInput(search, "R2"));
    expect(rowsParent().getAttribute("data-layout")).toBeNull();
    expect(style("R2").gridRow).toBe("");
    await act(async () => setInput(search, ""));
    expect(rowsParent().getAttribute("data-layout")).toBe("matrix");
    expect(host.querySelector('[data-parameter-row="R2"] input')).toBe(input);
    expect(input.value).toBe("9");
  });

  it("treats a server without layout reports as unreported, not as a missing grid", async () => {
    const [section] = gridSections();
    await mount([{ scope: null, fields: section.fields }]);
    await act(async () => nav("Pairs").click());
    expect(rowsParent().getAttribute("data-layout")).toBeNull();
    expect(host.querySelector(".parameter-workspace-layout-note")!.textContent).toContain("does not report");
  });

  // ADR-0139: manufacturer separators between the rows of a chosen area.
  const separator = (position: number, key: string, text: string | null, uiHint: string | null): ParameterSeparator => ({
    position, group: { key, label: key }, elementId: `S-${position}-${uiHint}`, text, uiHint, access: null, blockAccess: null });
  const withSeparators = (): ParameterSection[] => [{ scope: null, fields: [field("Mode", "Buttons"), field("Delay", "Buttons")],
    separators: [separator(0, "Buttons", "Behaviour", "Headline"), separator(1, "Buttons", null, "HorizontalRuler"),
      separator(2, "Buttons", "Check the wiring", "Error"), separator(2, "About", "Firmware notes", "Information"),
      separator(2, "About", "Plain text", "SomethingNew")] }];
  const shown = () => [...host.querySelectorAll<HTMLElement>(".parameter-workspace-rows > :not([hidden])")]
    .map(node => node.dataset.parameterRow ?? `${node.dataset.uiHint}:${node.textContent}`);

  it("shows separators by UI hint between the rows of their area only", async () => {
    await mount(withSeparators());
    expect(shown()).toEqual([]);
    await act(async () => nav("Buttons").click());
    expect(shown()).toEqual(["Headline:Behaviour", "Mode", "HorizontalRuler:", "Delay", "Error:Manufacturer warning: Check the wiring"]);
    expect(host.querySelector('[data-ui-hint="Headline"] h5')!.textContent).toBe("Behaviour");
    expect(host.querySelector('[data-ui-hint="HorizontalRuler"] hr')).not.toBeNull();
    // A block of text alone is an area; an unknown hint stays plain text.
    expect(nav("About").textContent).toContain("0");
    await act(async () => nav("About").click());
    expect(shown()).toEqual(["Information:Information: Firmware notes", "SomethingNew:Plain text"]);
  });

  it("keeps separators out of search results and leaves editor identity alone", async () => {
    await mount(withSeparators());
    await act(async () => nav("Buttons").click());
    const input = host.querySelector<HTMLInputElement>('[data-parameter-row="Delay"] input')!;
    await act(async () => setInput(input, "7"));
    await act(async () => setInput(host.querySelector<HTMLInputElement>('input[type="search"]')!, "Delay"));
    expect(shown()).toEqual(["Delay"]);
    expect(host.querySelector<HTMLInputElement>('[data-parameter-row="Delay"] input')).toBe(input);
    expect(input.value).toBe("7");
  });
  it("indents a row by its declared level, but not in search results", async () => {
    const indented = { ...field("Delay", "Buttons"), indentLevel: 2 };
    await mount([{ scope: null, fields: [field("Mode", "Buttons"), indented] }]);
    await act(async () => nav("Buttons").click());
    expect(host.querySelector('[data-parameter-row="Delay"]')!.getAttribute("data-indent")).toBe("2");
    expect(host.querySelector('[data-parameter-row="Mode"]')!.hasAttribute("data-indent")).toBe(false);
    const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => setInput(search, "Delay"));
    expect(host.querySelector('[data-parameter-row="Delay"]')!.hasAttribute("data-indent")).toBe(false);
  });
});
