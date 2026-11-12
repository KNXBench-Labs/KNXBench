/** Tests for the parameter panel's last-download marks and summary line (ADR-0173 §5). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LastDownload, ParameterField, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { formatLogTime } from "./logRows";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetProductLanguageForTests } from "./productLanguage";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
  setParameterValue: vi.fn(),
  parameterEditorSource: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ParameterPanel, { ParameterPanelContent, useDeviceParameters, type ParameterView } from "./ParameterPanel";
import { resetSettingsForTests, setSetting } from "./settingsStore";

const tree = (revision: number): ProjectTree => ({ schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false,
  is_modified: true, group_address_style: "ThreeLevel", installations: [], server_incarnation: "srv-ld", project_incarnation: 1,
  snapshot_revision: revision });

const AT = "2026-01-05T08:09:10.000Z";
const field = (etsId: string, extra: Partial<ParameterField> = {}): ParameterField => ({
  etsId, name: `Field ${etsId}`, nameLanguage: null, text: null, textLanguage: null, kind: "Number", value: "5",
  valueSource: "Stored", editable: true, min: "0", max: "10", enumOptions: [], displayOrder: null, access: null,
  writeEtsId: etsId, ...extra,
});
const mode = field("P4", { kind: "Restriction", value: "1", enumOptions: [
  { value: "1", text: "On", language: null }, { value: "0", text: "Off", language: null }, { value: "2", text: null, language: null }] });
const hiddenByAccess = field("P5", { access: "None", editable: false, writeEtsId: null });

function panel(lastDownload?: LastDownload, programId = "PROG-1"): ParameterPanelDto {
  return {
    programId, sourceLanguage: null, stale: [], diagnostics: [], tree: null, lastDownload,
    sections: [
      { scope: null, fields: [field("P1"), mode, hiddenByAccess] },
      { scope: { moduleNode: 7, moduleId: null, moduleDefId: "MD-1" },
        fields: [field("P1", { editable: false, writeEtsId: null })] },
    ],
  };
}
const usable = (differing: NonNullable<LastDownload["differing"]>): LastDownload => ({ state: "usable", at: AT, differing });

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetProductLanguageForTests();
  resetUiLanguageForTests();
});

async function render(view: ParameterView = "parameters", refreshKey = tree(2), deviceId = 1) {
  if (!host) { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); }
  await act(async () => {
    root!.render(<ParameterPanel deviceId={deviceId} view={view} refreshKey={refreshKey} onValueApplied={() => {}} />);
  });
}

/** Types `next` into P1's input and leaves it, which applies the value. */
async function writeP1(next: string) {
  const input = unscopedRow("P1").querySelector<HTMLInputElement>("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, next);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await Promise.resolve();
  });
}

const summary = () => host!.querySelector<HTMLElement>(".parameter-last-download");
const unscopedRow = (etsId: string) => host!.querySelectorAll<HTMLLabelElement>(`.parameter-field[data-ets-id="${etsId}"]`)[0];
const marks = () => host!.querySelectorAll(".parameter-download-mark");
const time = (language: "en" | "de") => formatLogTime(Date.parse(AT), language, new Date())!;
/** The text a control's `aria-describedby` points at; `null` without one. */
function description(row: HTMLElement): string | null {
  const control = row.querySelector("input, select")!;
  const id = control.getAttribute("aria-describedby");
  return id === null ? null : document.getElementById(id)?.textContent ?? "";
}

describe("ParameterPanel last download (ADR-0173)", () => {
  it("marks differing unscoped rows, names enum labels and counts values the view does not show", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable([
      { refId: "P1", downloaded: "4", current: "5" },
      { refId: "P4", downloaded: "0", current: "1" },
      { refId: "P5", downloaded: "1", current: "2" },
      { refId: "P9", downloaded: "3", current: null },
    ])));
    await render();
    // P5 is manufacturer-restricted (its own tab marks it); P9 no field presents.
    expect(summary()!.textContent).toBe(`4 values differ from the last download (${time("en")}). `
      + "1 of them is not shown with the current settings. 1 of them is in the Manufacturer fields tab.");
    expect(summary()!.getAttribute("role")).toBeNull();
    expect(summary()!.getAttribute("aria-live")).toBeNull();
    expect(summary()!.querySelector(".parameter-last-download-dot")!.getAttribute("aria-hidden")).toBe("true");
    const p1 = unscopedRow("P1"); const p4 = unscopedRow("P4");
    expect(p1.dataset.differsFromDownload).toBe("true");
    expect(description(p1)).toBe("Value at last download: 4");
    expect(description(p4)).toBe("Value at last download: Off");
    expect(p4.querySelector(".parameter-download-mark")!.getAttribute("title")).toBe("Value at last download: Off");
    // The mark is a hidden dot plus a description: the label's name text is unchanged.
    const dot = p1.querySelector(".parameter-download-mark")!;
    expect(dot.getAttribute("aria-hidden")).toBe("true");
    expect(dot.textContent).toBe("");
    expect(p1.querySelector(".parameter-field-name")!.textContent).toBe("Field P1");
    // The note is the label's next sibling, never part of its name, always shown.
    const note = document.getElementById(p1.querySelector("input")!.getAttribute("aria-describedby")!)!;
    expect(p1.contains(note)).toBe(false);
    expect(p1.nextElementSibling).toBe(note);
    expect(note.classList.contains("parameter-download-note")).toBe(true);
    expect(note.hidden).toBe(false);
    // Module-scoped rows never match: baselines are unscoped.
    expect(marks()).toHaveLength(2);
    const moduleRow = host!.querySelectorAll('[data-ets-id="P1"]')[1];
    expect(moduleRow).toBeDefined();
    expect(moduleRow.getAttribute("data-differs-from-download")).toBeNull();
  });

  it("says one value in the singular and shows an undeclared or missing downloaded value plainly", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable([{ refId: "P4", downloaded: "7", current: "1" }])));
    await render();
    expect(summary()!.textContent).toBe(`1 value differs from the last download (${time("en")}).`);
    expect(description(unscopedRow("P4"))).toBe("Value at last download: 7");
    await act(async () => root!.unmount()); root = undefined; host!.remove(); host = undefined;

    apiMock.deviceParameters.mockResolvedValue(panel(usable([{ refId: "P1", downloaded: null, current: "5" },
      { refId: "P4", downloaded: "2", current: "1" }])));
    await render();
    expect(description(unscopedRow("P1"))).toBe("Value at last download: no value");
    // An option without a label shows its value, as the select does.
    expect(description(unscopedRow("P4"))).toBe("Value at last download: 2");
  });

  it("says when no value differs, without marks", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable([])));
    await render();
    expect(summary()!.textContent).toBe(`No value differs from the last download (${time("en")}).`);
    expect(summary()!.dataset.differs).toBeUndefined();
    expect(marks()).toHaveLength(0);
    expect(description(unscopedRow("P1"))).toBeNull();
  });

  it.each(["absent", "otherProgram", "productUnavailable", "currentUnreadable"] as const)(
    "gives one plain sentence and no marks for %s, in English and German", async (state) => {
      apiMock.deviceParameters.mockResolvedValue(panel({ state }));
      await render();
      expect(summary()!.textContent).toBe(en[`parameters.lastDownload.${state}`]);
      expect(summary()!.dataset.state).toBe(state);
      expect(marks()).toHaveLength(0);
      await act(async () => setSetting(UI_LANGUAGE_STORAGE_KEY, "de"));
      expect(summary()!.textContent).toBe(de[`parameters.lastDownload.${state}`]);
    });

  it("speaks German with plural forms and enum labels", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(panel(usable([
      { refId: "P4", downloaded: "0", current: "1" }, { refId: "P9", downloaded: null, current: "1" }])));
    await render();
    expect(summary()!.textContent).toBe(`2 Werte weichen vom letzten Programmieren ab (${time("de")}). Davon wird 1 mit den aktuellen Einstellungen nicht angezeigt.`);
    expect(description(unscopedRow("P4"))).toBe("Wert beim letzten Programmieren: Off");
    await act(async () => root!.unmount()); root = undefined; host!.remove(); host = undefined;
    apiMock.deviceParameters.mockResolvedValue(panel(usable([{ refId: "P1", downloaded: null, current: "5" }])));
    await render();
    expect(summary()!.textContent).toBe(`1 Wert weicht vom letzten Programmieren ab (${time("de")}).`);
    expect(description(unscopedRow("P1"))).toBe("Wert beim letzten Programmieren: kein Wert");
  });

  it("renders nothing when the server reports no comparison", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(undefined));
    await render();
    expect(summary()).toBeNull();
    expect(marks()).toHaveLength(0);
    expect(host!.querySelector("[data-differs-from-download]")).toBeNull();
  });

  it("shows the line only in the Parameters view and marks rows of the Restricted view", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable([{ refId: "P5", downloaded: "1", current: "2" }])));
    await render("restricted");
    expect(summary()).toBeNull();
    expect(description(unscopedRow("P5"))).toBe("Value at last download: 1");
    expect(unscopedRow("P5").nextElementSibling!.classList.contains("parameter-download-note")).toBe(true);
    await render("diagnostics");
    expect(summary()).toBeNull();
  });

  it("keeps the marks across a write's response and takes the next read's comparison", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" },
      { refId: "P4", downloaded: "0", current: "1" }])));
    const written = panel(undefined);
    written.sections[0].fields[0] = field("P1", { value: "4" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    expect(marks()).toHaveLength(2);
    expect(summary()!.dataset.refreshing).toBeUndefined();
    await writeP1("4");
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(1);
    expect(unscopedRow("P1").querySelector("input")!.value).toBe("4");
    // The write's response has no comparison: the previous one stays until the
    // GET, without the written ref, whose state is unknown now; muted meanwhile.
    expect(summary()!.textContent).toBe(`1 value differs from the last download (${time("en")}).`);
    expect(summary()!.dataset.refreshing).toBe("true");
    expect(unscopedRow("P1").dataset.differsFromDownload).toBeUndefined();
    expect(description(unscopedRow("P1"))).toBeNull();
    expect(unscopedRow("P4").dataset.differsFromDownload).toBe("true");
    expect(marks()).toHaveLength(1);
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "3", current: "4" },
      { refId: "P4", downloaded: "0", current: "1" }])));
    await render("parameters", tree(3));
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(2);
    // The read decides again: here the written value still differs.
    expect(summary()!.textContent).toBe(`2 values differ from the last download (${time("en")}).`);
    expect(summary()!.dataset.refreshing).toBeUndefined();
    expect(description(unscopedRow("P1"))).toBe("Value at last download: 3");
    expect(marks()).toHaveLength(2);
  });

  it("drops a ref by the written field's write id too", async () => {
    const aliased = panel(usable([{ refId: "P1", downloaded: "4", current: "5" }, { refId: "P1-W", downloaded: "4", current: "5" },
      { refId: "P4", downloaded: "0", current: "1" }]));
    aliased.sections[0].fields[0] = field("P1", { writeEtsId: "P1-W" });
    apiMock.deviceParameters.mockResolvedValueOnce(aliased);
    const written = panel(undefined);
    written.sections[0].fields[0] = field("P1", { writeEtsId: "P1-W", value: "4" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    expect(summary()!.textContent).toContain("3 values differ");
    await writeP1("4");
    expect(apiMock.setParameterValue.mock.calls[0][1]).toBe("P1-W");
    expect(summary()!.textContent).toBe(`1 value differs from the last download (${time("en")}).`);
  });

  it("says nothing while refreshing once the only differing value was written", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" }])));
    const written = panel(undefined);
    written.sections[0].fields[0] = field("P1", { value: "4" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    expect(summary()!.textContent).toContain("1 value differs");
    await writeP1("4");
    // Only the server's next read may say that no value differs.
    expect(summary()).toBeNull();
    expect(host!.textContent).not.toContain("No value differs");
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([])));
    await render("parameters", tree(3));
    expect(summary()!.textContent).toBe(`No value differs from the last download (${time("en")}).`);
    expect(summary()!.getAttribute("aria-busy")).toBeNull();
  });

  it("marks a kept comparison as busy while it waits for the read", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" },
      { refId: "P4", downloaded: "0", current: "1" }])));
    const written = panel(undefined);
    written.sections[0].fields[0] = field("P1", { value: "4" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    expect(summary()!.getAttribute("aria-busy")).toBeNull();
    await writeP1("4");
    expect(summary()!.getAttribute("aria-busy")).toBe("true");
  });

  it("keeps the unscoped mark when a module-scoped row of the same id is written", async () => {
    const scoped = (dto: ParameterPanelDto) => {
      dto.sections[1].fields[0] = field("P1", { writeEtsId: "P1@7" });
      return dto;
    };
    apiMock.deviceParameters.mockResolvedValueOnce(scoped(panel(usable([{ refId: "P1", downloaded: "4", current: "5" }]))));
    const written = scoped(panel(undefined));
    written.sections[1].fields[0] = field("P1", { writeEtsId: "P1@7", value: "9" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    const moduleInput = host!.querySelectorAll<HTMLLabelElement>('.parameter-field[data-ets-id="P1"]')[1]
      .querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(moduleInput, "9");
      moduleInput.dispatchEvent(new Event("input", { bubbles: true }));
      moduleInput.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setParameterValue.mock.calls[0][1]).toBe("P1@7");
    // The module row's value is its own; the unscoped comparison is unchanged.
    expect(unscopedRow("P1").dataset.differsFromDownload).toBe("true");
    expect(summary()!.textContent).toContain("1 value differs");
  });

  it("does not retain a comparison when a late write answers after a device switch", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" },
      { refId: "P4", downloaded: "0", current: "1" }])));
    let answer!: (written: ParameterPanelDto) => void;
    apiMock.setParameterValue.mockImplementation(() => new Promise<ParameterPanelDto>((resolve) => { answer = resolve; }));
    await render();
    await writeP1("6");
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(1);
    // Device 2 runs the same program and has no comparison of its own.
    const other = panel(undefined);
    other.sections[0].fields[0] = field("P1", { value: "9" });
    apiMock.deviceParameters.mockResolvedValueOnce(other);
    await render("parameters", tree(2), 2);
    expect(apiMock.deviceParameters).toHaveBeenLastCalledWith(2, null);
    expect(summary()).toBeNull();
    const late = panel(undefined);
    late.sections[0].fields[0] = field("P1", { value: "6" });
    late.tree = tree(3);
    await act(async () => answer(late));
    expect(summary()).toBeNull();
    expect(marks()).toHaveLength(0);
    expect(unscopedRow("P1").querySelector("input")!.value).toBe("9");
  });

  it("refuses a write answer for the previous device at the read model, keeping the new device's comparison", async () => {
    let state: ReturnType<typeof useDeviceParameters> | null = null;
    const key = tree(2);
    function Harness(props: { deviceId: number }) {
      const own = useDeviceParameters(props.deviceId, key);
      state = own;
      return <ParameterPanelContent deviceId={props.deviceId} refreshKey={key} onValueApplied={() => {}} state={own} />;
    }
    host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P4", downloaded: "0", current: "1" }])));
    await act(async () => root!.render(<Harness deviceId={1} />));
    expect(marks()).toHaveLength(1);
    // Device 2, same program, has its own comparison.
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" }])));
    await act(async () => root!.render(<Harness deviceId={2} />));
    expect(summary()!.textContent).toBe(`1 value differs from the last download (${time("en")}).`);
    const late = panel(undefined);
    late.tree = tree(3);
    await act(async () => state!.acceptWrite(1, null, late, ["P1"]));
    expect(summary()!.dataset.refreshing).toBeUndefined();
    expect(description(unscopedRow("P1"))).toBe("Value at last download: 4");
    expect(marks()).toHaveLength(1);
  });

  it("does not carry a comparison over to another program's write response", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable([{ refId: "P1", downloaded: "4", current: "5" }])));
    const written = panel(undefined, "PROG-2");
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    await writeP1("6");
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(1);
    expect(summary()).toBeNull();
    expect(marks()).toHaveLength(0);
  });
});
