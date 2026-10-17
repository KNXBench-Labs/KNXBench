/** Parameter navigation retains editors and never changes engineering data. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { ParameterField, ParameterSection } from "./api";
import ParameterWorkspace from "./ParameterWorkspace";

let host: HTMLDivElement;
let root: Root;
afterEach(async () => { if (root) await act(async () => root.unmount()); host?.remove(); });
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
  it("shows source-group navigation and a compact editing area instead of stacked accordions", async () => {
    await mount();
    expect(nav("Buttons").getAttribute("aria-current")).toBe("page");
    expect(host.querySelector('[data-parameter-row="Mode"]')!.closest("[hidden]")).toBeNull();
    expect(host.querySelector('[data-parameter-row="Delay"]')!.closest("[hidden]")).not.toBeNull();
    expect(host.querySelector(".parameter-workspace-columns")!.textContent).toContain("Value");
    expect(host.querySelector("details.parameter-section")).toBeNull();
  });

  it("retains the same draft editor across navigation and search, including no-result searches", async () => {
    await mount();
    const input = host.querySelector<HTMLInputElement>('[data-parameter-row="Mode"] input')!;
    await act(async () => setInput(input, "7"));
    await act(async () => nav("LEDs").click());
    await act(async () => nav("Buttons").click());
    const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => setInput(search, "absent"));
    expect(host.textContent).toContain("No parameters match");
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Mode"]')!.hidden).toBe(true);
    await act(async () => setInput(search, ""));
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

  it("falls back visibly when the selected group disappears", async () => {
    await mount();
    await act(async () => nav("LEDs").click());
    await act(async () => root.render(<ParameterWorkspace sections={[{ scope: null, fields: [field("Mode", "Buttons")] }]} renderField={renderField} />));
    expect(nav("Buttons").getAttribute("aria-current")).toBe("page");
    expect(host.querySelector<HTMLDivElement>('[data-parameter-row="Mode"]')!.hidden).toBe(false);
  });
});
