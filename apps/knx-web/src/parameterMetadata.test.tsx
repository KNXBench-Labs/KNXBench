/** Verifies localized parameter metadata while preserving source text, tokens and values. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterField, ParameterPanel as PanelDto } from "./api";
import { translateFor } from "./i18n";
import { parameterMetadataLabel } from "./parameterMetadata";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { resetProductLanguageForTests } from "./productLanguage";

const apiMock = vi.hoisted(() => ({ deviceParameters: vi.fn(), setParameterValue: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (error: unknown) => String(error) }));
import ParameterPanel from "./ParameterPanel";

let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove(); host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests(); resetUiLanguageForTests(); resetProductLanguageForTests();
});

const CASES = [
  ["kind", "Restriction", "Auswahl"], ["kind", "Number", "Zahl"],
  ["kind", "Text", "Text"], ["kind", "None", "Ohne Wert"],
  ["kind", "Float", "Gleitkommazahl"], ["kind", "IPAddress", "IP-Adresse"],
  ["kind", "Picture", "Bild"], ["kind", "Raw", "Rohdaten"],
  ["kind", "Color", "Farbe"], ["kind", "Date", "Datum"],
  ["kind", "Time", "Zeit"], ["kind", "Other", "Sonstiger Typ"],
  ["source", "Stored", "Im Projekt gespeichert"], ["source", "ProgramDefault", "Programmstandard"],
  ["access", "None", "Kein Zugriff"], ["access", "Read", "Nur lesen"],
  ["access", "ReadWrite", "Lesen und Schreiben"],
] as const;

describe("parameter metadata labels", () => {
  it.each(CASES)("localizes %s %s without changing its English token", (category, token, german) => {
    expect(parameterMetadataLabel((key, params) => translateFor("de", key, params), category, token)).toBe(german);
    expect(parameterMetadataLabel((key, params) => translateFor("en", key, params), category, token)).toBe(token);
  });

  it.each(["bar", "tlh"])("keeps intentional English metadata fallback for %s", language => {
    expect(translateFor(language, "parameters.access.ReadWrite")).toBe("ReadWrite");
    expect(translateFor(language, "parameters.valueSource.ProgramDefault")).toBe("ProgramDefault");
  });

  it.each(["kind", "source", "access"] as const)("preserves unknown %s tokens, including prototype names", category => {
    for (const token of ["FutureToken", "ReadOnly", "constructor", "toString", "__proto__", ""]) {
      expect(parameterMetadataLabel((key, params) => translateFor("de", key, params), category, token)).toBe(token);
    }
  });
});

const field = (overrides: Partial<ParameterField> = {}): ParameterField => ({
  etsId: "P-1", group: { key: "area", label: "Manufacturer area" }, name: "InternalName",
  text: "Manufacturer caption", nameLanguage: null, textLanguage: null,
  kind: "Restriction", value: "on", valueSource: "ProgramDefault", editable: true,
  min: null, max: null, enumOptions: [{ value: "on", text: "Manufacturer option", language: null }],
  displayOrder: null, access: "ReadWrite", blockAccess: null, writeEtsId: "P-1", ...overrides,
});

async function mount(fields: ParameterField[], view: "parameters" | "restricted" = "parameters") {
  setSetting("uiLanguage", "de"); resetUiLanguageForTests();
  const panel: PanelDto = { programId: "PROG-1", sourceLanguage: "en-US",
    sections: [{ scope: null, fields }], stale: [], diagnostics: [], tree: null };
  const before = structuredClone(panel);
  apiMock.deviceParameters.mockResolvedValue(panel);
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<ParameterPanel deviceId={1} view={view} onValueApplied={() => {}} />));
  return { panel, before };
}

function infoValues() {
  return [...host!.querySelectorAll<HTMLElement>(".parameter-workspace-context dd")].map(node => node.textContent);
}

it("translates the mounted parameter badge and context, preserves originals, and switches language live", async () => {
  const { panel, before } = await mount([field()]);
  const nav = host!.querySelector<HTMLButtonElement>(".parameter-workspace-nav button")!;
  await act(async () => nav.click());
  const badge = host!.querySelector<HTMLElement>(".parameter-field .provenance-badge")!;
  expect(badge.textContent).toBe("Lesen und Schreiben"); expect(badge.title).toBe("ReadWrite");
  await act(async () => host!.querySelector<HTMLButtonElement>(".parameter-workspace-info")!.click());
  expect(infoValues()).toEqual(["P-1", "Auswahl", "Programmstandard", "on", "Lesen und Schreiben"]);
  expect(host!.querySelector('.parameter-workspace-context dd[title="Restriction"]')!.textContent).toBe("Auswahl");
  expect(host!.querySelector('.parameter-workspace-context dd[title="ProgramDefault"]')!.textContent).toBe("Programmstandard");
  const select = host!.querySelector<HTMLSelectElement>(".parameter-field select")!;
  expect(select.value).toBe("on");
  expect(select.selectedOptions[0].textContent).toBe("Manufacturer option");
  expect(host!.querySelector(".parameter-field-name")!.textContent).toBe("Manufacturer caption");
  expect(nav.textContent).toContain("Manufacturer area");
  await act(async () => setSetting("uiLanguage", "en"));
  expect(badge.textContent).toBe("ReadWrite");
  expect(infoValues()).toEqual(["P-1", "Restriction", "ProgramDefault", "on", "ReadWrite"]);
  await act(async () => setSetting("uiLanguage", "de"));
  expect(badge.textContent).toBe("Lesen und Schreiben");
  expect(panel).toEqual(before);
  expect(apiMock.deviceParameters).toHaveBeenCalledTimes(1);
  expect(apiMock.deviceParameters).toHaveBeenCalledWith(1, null);
  expect(apiMock.setParameterValue).not.toHaveBeenCalled();
});

it("localizes stored provenance and block access separately while retaining unknown metadata", async () => {
  const { panel, before } = await mount([field({ kind: "FutureKind", valueSource: "Stored", blockAccess: "ReadWrite" })]);
  await act(async () => host!.querySelector<HTMLButtonElement>(".parameter-workspace-nav button")!.click());
  await act(async () => host!.querySelector<HTMLButtonElement>(".parameter-workspace-info")!.click());
  expect(infoValues()).toEqual(["P-1", "FutureKind", "Im Projekt gespeichert", "on", "Lesen und Schreiben", "Lesen und Schreiben"]);
  expect(host!.querySelector('.parameter-workspace-context dd[title="ReadWrite"]')!.textContent).toBe("Lesen und Schreiben");
  expect(panel).toEqual(before); expect(apiMock.setParameterValue).not.toHaveBeenCalled();
});

it.each([["Read", "Nur lesen"], ["None", "Kein Zugriff"]] as const)(
  "translates %s badges in the restricted view without enabling editing", async (access, label) => {
    const { panel, before } = await mount([field({ access, editable: false, writeEtsId: null })], "restricted");
    await act(async () => host!.querySelector<HTMLButtonElement>(".parameter-group-toggle")!.click());
    const badge = host!.querySelector<HTMLElement>(".parameter-field .provenance-badge")!;
    expect(badge.textContent).toBe(label); expect(badge.title).toBe(access);
    expect(host!.querySelector<HTMLSelectElement>(".parameter-field select")!.disabled).toBe(true);
    expect(panel).toEqual(before); expect(apiMock.setParameterValue).not.toHaveBeenCalled();
  },
);
