/** Section, group and scope marks for values that differ from the last download (#48). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LastDownload, ParameterField, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
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

import ParameterPanel, { type ParameterView } from "./ParameterPanel";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { differingAreas, indexParameterWorkspace } from "./parameterWorkspace";

const tree = (revision: number): ProjectTree => ({ schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false,
  is_modified: true, group_address_style: "ThreeLevel", installations: [], server_incarnation: "srv-48", project_incarnation: 1,
  snapshot_revision: revision });
const AT = "2026-01-05T08:09:10.000Z";
const group = (key: string, label: string) => ({ key, label });
const field = (etsId: string, groupKey: string, label: string, extra: Partial<ParameterField> = {}): ParameterField => ({
  etsId, name: `Field ${etsId}`, nameLanguage: null, text: `Text ${etsId}`, textLanguage: null, kind: "Number", value: "5",
  valueSource: "Stored", editable: true, min: "0", max: "10", enumOptions: [], displayOrder: null, access: null,
  writeEtsId: etsId, group: group(groupKey, label), ...extra,
});
const MODULE = { moduleNode: 7, moduleId: "Module A", moduleDefId: "MD-1" };
/** Fictional: two Status LED values differ (the report), one repeated, a display-only and a restricted field. */
function panel(lastDownload?: LastDownload): ParameterPanelDto {
  return {
    programId: "PROG-48", sourceLanguage: null, stale: [], diagnostics: [], tree: null, lastDownload,
    sections: [
      { scope: null, fields: [
        field("P1", "general", "General"),
        field("P10", "led", "Status LED"),
        field("P11", "led", "Status LED"),
        // The same ref presented twice must not mark twice.
        field("P10", "led", "Status LED"),
        field("P20", "display", "Display", { kind: "None", editable: false, writeEtsId: null }),
        field("P30", "factory", "Factory", { access: "Read", editable: false, writeEtsId: null }),
      ] },
      { scope: MODULE, fields: [field("P10", "module", "Module LED", { editable: false, writeEtsId: null })] },
    ],
  };
}
const usable = (...refIds: string[]): LastDownload => ({ state: "usable", at: AT,
  differing: refIds.map(refId => ({ refId, downloaded: "4", current: "5" })) });

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
async function type(etsId: string, next: string) {
  const input = host!.querySelector<HTMLInputElement>(`.parameter-field[data-ets-id="${etsId}"] input`)!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, next);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await Promise.resolve();
  });
}
const navButton = (label: string) => [...host!.querySelectorAll<HTMLButtonElement>(".parameter-workspace-nav button")]
  .find(button => button.firstElementChild!.textContent === label)!;
const scopeLabel = (text: string) => [...host!.querySelectorAll<HTMLElement>(".parameter-workspace-scope")]
  .find(scope => scope.textContent!.startsWith(text))!;
const described = (element: Element) => {
  const id = element.getAttribute("aria-describedby");
  return id === null ? null : document.getElementById(id)?.textContent ?? "";
};
const markedButtons = () => [...host!.querySelectorAll<HTMLButtonElement>('.parameter-workspace-nav button[data-differs-from-download="true"]')]
  .map(button => button.firstElementChild!.textContent);

describe("#48 section marks in the Parameters view", () => {
  it("marks the Status LED area and its scope, and nothing else, before any area is opened", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable("P10", "P11", "P20", "P30", "P99")));
    await render();
    expect(host!.querySelector(".parameter-workspace-closed")).not.toBeNull();
    // P20 is display-only, P30 lives in the Manufacturer fields tab, P99 no field presents, P10's module row is scoped.
    expect(markedButtons()).toEqual(["Status LED"]);
    const led = navButton("Status LED");
    expect(described(led)).toBe(en["parameters.lastDownload.section"]);
    expect(led.querySelectorAll(".parameter-section-download-mark")).toHaveLength(1);
    expect(led.querySelector(".parameter-section-download-mark")!.getAttribute("aria-hidden")).toBe("true");
    // The accessible name is unchanged: label and count, no mark text.
    expect(led.textContent).toBe("Status LED3");
    for (const clean of ["General", "Display", "Module LED"]) {
      expect(navButton(clean).hasAttribute("aria-describedby"), clean).toBe(false);
      expect(navButton(clean).querySelector(".parameter-section-download-mark"), clean).toBeNull();
    }
    const device = scopeLabel(en["parameters.deviceScope"]);
    expect(device.dataset.differsFromDownload).toBe("true");
    expect(device.querySelector(".sr-only")!.textContent).toBe(` (${en["parameters.lastDownload.section"]})`);
    const module = scopeLabel("Module A");
    expect(module.dataset.differsFromDownload).toBeUndefined();
    expect(module.querySelector(".parameter-section-download-mark")).toBeNull();
  });

  it("keeps the marks whichever area is open and while searching", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable("P10")));
    await render();
    await act(async () => navButton("General").click());
    expect(navButton("General").getAttribute("aria-current")).toBe("page");
    expect(markedButtons()).toEqual(["Status LED"]);
    const search = host!.querySelector<HTMLInputElement>(".parameter-workspace-search input")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(search, "Text P1");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(markedButtons()).toEqual(["Status LED"]);
  });

  it("marks nothing without a usable comparison", async () => {
    for (const [index, lastDownload] of [undefined, { state: "otherProgram" } as LastDownload, { state: "absent" } as LastDownload,
      { state: "usable", at: AT, differing: [] } as LastDownload].entries()) {
      apiMock.deviceParameters.mockResolvedValue(panel(lastDownload));
      await render("parameters", tree(index + 2));
      expect(host!.querySelector("[data-differs-from-download]"), JSON.stringify(lastDownload)).toBeNull();
      expect(host!.querySelector(".parameter-section-download-mark")).toBeNull();
      if (root) await act(async () => root!.unmount());
      host!.remove(); host = undefined; root = undefined;
    }
  });

  it("drops the written ref's area at once, keeps the others, and takes the next read", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable("P1", "P10", "P11")));
    const written = panel(undefined);
    written.sections[0].fields[0] = field("P1", "general", "General", { value: "4" });
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    expect(markedButtons()).toEqual(["General", "Status LED"]);
    await type("P1", "4");
    expect(apiMock.setParameterValue).toHaveBeenCalledTimes(1);
    // P1's state is unknown until the read; Status LED still holds two differing values.
    expect(markedButtons()).toEqual(["Status LED"]);
    expect(scopeLabel(en["parameters.deviceScope"]).dataset.differsFromDownload).toBe("true");
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable("P1")));
    await render("parameters", tree(3));
    expect(markedButtons()).toEqual(["General"]);
  });

  it("clears the scope mark when the last differing area is written, until the read decides", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable("P1")));
    const written = panel(undefined);
    written.tree = tree(3);
    apiMock.setParameterValue.mockResolvedValue(written);
    await render();
    await type("P1", "4");
    expect(markedButtons()).toEqual([]);
    expect(scopeLabel(en["parameters.deviceScope"]).dataset.differsFromDownload).toBeUndefined();
  });

  it("carries no marks to another device", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(panel(usable("P10")));
    await render();
    expect(markedButtons()).toEqual(["Status LED"]);
    apiMock.deviceParameters.mockResolvedValueOnce(panel(undefined));
    await render("parameters", tree(2), 2);
    expect(apiMock.deviceParameters).toHaveBeenLastCalledWith(2, null);
    expect(markedButtons()).toEqual([]);
  });

  it("speaks German", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(panel(usable("P11")));
    await render();
    expect(described(navButton("Status LED"))).toBe(de["parameters.lastDownload.section"]);
    expect(scopeLabel(de["parameters.deviceScope"]).querySelector(".sr-only")!.textContent).toBe(` (${de["parameters.lastDownload.section"]})`);
  });
});

describe("#48 section marks in the Manufacturer fields view", () => {
  it("marks the closed section and the collapsed group holding a restricted differing value", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable("P10", "P30")));
    await render("restricted");
    const sections = [...host!.querySelectorAll<HTMLDetailsElement>("details.parameter-section")];
    // Only the unscoped section holds a restricted field here.
    expect(sections).toHaveLength(1);
    expect(sections[0].open).toBe(false);
    const summary = sections[0].querySelector("summary")!;
    expect(summary.dataset.differsFromDownload).toBe("true");
    expect(described(summary)).toBe(en["parameters.lastDownload.section"]);
    expect(summary.querySelector(".parameter-section-download-mark")!.getAttribute("aria-hidden")).toBe("true");
    const toggle = host!.querySelector<HTMLButtonElement>(".parameter-group-toggle")!;
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.dataset.differsFromDownload).toBe("true");
    expect(described(toggle)).toBe(en["parameters.lastDownload.section"]);
    // P10 differs too, but the Manufacturer fields view does not present it.
    expect(host!.querySelectorAll('[data-differs-from-download="true"]')).toHaveLength(3);
  });

  it("leaves the section unmarked when only unrestricted values differ", async () => {
    apiMock.deviceParameters.mockResolvedValue(panel(usable("P10", "P11")));
    await render("restricted");
    expect(host!.querySelector("[data-differs-from-download]")).toBeNull();
    expect(host!.querySelector(".parameter-section-download-mark")).toBeNull();
  });
});

describe("differingAreas", () => {
  it("maps refs to their actual areas once, never through display-only fields", () => {
    const { rows, groups } = indexParameterWorkspace(panel().sections);
    const refs = new Set(["P10", "P20"]);
    const marks = differingAreas(rows, groups, (candidate, scope) => scope === null && refs.has(candidate.etsId));
    expect([...marks.groups].map(key => groups.find(g => g.key === key)!.label)).toEqual(["Status LED"]);
    expect(marks.sections.size).toBe(1);
    expect(differingAreas(rows, groups, () => false)).toEqual({ groups: new Set(), sections: new Set(), containers: new Set() });
  });
});
