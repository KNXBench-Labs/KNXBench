/** Mounted navigation keeps drafts, fallback paths and descendant marks without owning values. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import type { ParameterContainer, ParameterField, ParameterSection } from "./api";
import ParameterWorkspace from "./ParameterWorkspace";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

let root: Root | undefined;
let host: HTMLDivElement;
afterEach(async () => { if (root) await act(async () => root!.unmount()); host?.remove(); root = undefined; resetSettingsForTests(); resetUiLanguageForTests(); });
const owner = (nodeId: number): ParameterContainer => ({ scope: null, nodeId, elementId: null, kind: "Channel", label: "#" });
const field = (etsId: string, container: ParameterContainer | null): ParameterField => ({
  etsId, group: { key: etsId, label: `${etsId} page`, container }, name: etsId, text: etsId,
  nameLanguage: null, textLanguage: null, kind: "Number", value: "4", valueSource: "Stored", editable: true,
  min: "0", max: "100", enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId,
});
function Draft({ field: item }: { field: ParameterField }) {
  const [value, setValue] = useState(item.value!);
  return <input aria-label={item.etsId} value={value} onChange={event => setValue(event.target.value)} />;
}
function inputValue(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

it.each(["en", "de"] as const)("%s folds only navigation and retains source-ordered editors and searchable fallback paths", async language => {
  saveUiLanguage(settingsStorage, language); resetUiLanguageForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  const a = owner(13); const b = owner(19);
  const data: ParameterSection[] = [{ scope: null, fields: [field("Root", null), field("Amber", a), field("Blue", b)] }];
  const render = async (sections = data) => act(async () => root!.render(<ParameterWorkspace sections={sections}
    renderField={item => <Draft field={item} />} differs={item => item.etsId === "Amber"} />));
  await render();
  const buttons = [...host.querySelectorAll<HTMLButtonElement>(".parameter-workspace-container-toggle")];
  expect(buttons.map(button => button.textContent)).toEqual(["▸ Container 11", "▸ Container 21"]);
  expect(buttons[0].dataset.differsFromDownload).toBe("true");
  expect(buttons[1].dataset.differsFromDownload).toBeUndefined();
  expect(buttons.every(button => button.getAttribute("aria-expanded") === "false")).toBe(true);
  await act(async () => buttons[0].click());
  const amberArea = host.querySelector<HTMLButtonElement>(".parameter-workspace-container-children button")!;
  await act(async () => amberArea.click());
  const row = host.querySelector<HTMLDivElement>('[data-parameter-row="Amber"]')!;
  const input = row.querySelector<HTMLInputElement>("input")!;
  await act(async () => inputValue(input,"27"));
  await act(async () => buttons[0].click());
  expect(buttons[0].getAttribute("aria-expanded")).toBe("false");
  expect(row.hidden).toBe(false); // folding the register does not close the selected editor
  expect(input.value).toBe("27");
  const search = host.querySelector<HTMLInputElement>('input[type="search"]')!;
  await act(async () => inputValue(search,"Container 1 Amber"));
  expect(row.hidden).toBe(false);
  expect(row.querySelector(".parameter-workspace-result-path")!.textContent).toContain("Container 1 / Amber page");
  expect(buttons[0].getAttribute("aria-expanded")).toBe("true");
  await act(async () => inputValue(search,""));
  expect(buttons[0].getAttribute("aria-expanded")).toBe("false");
  const changed: ParameterSection[] = [{ ...data[0], fields: [data[0].fields[0], field("New",a), ...data[0].fields.slice(1)] }];
  await render(changed);
  expect(host.querySelector('[data-parameter-row="Amber"]')).toBe(row);
  expect(row.querySelector("input")).toBe(input);
  expect(input.value).toBe("27");
  expect(host.querySelectorAll(".parameter-workspace-row")).toHaveLength(4);
  expect(data[0].fields.map(item => item.value)).toEqual(["4","4","4"]);
});
