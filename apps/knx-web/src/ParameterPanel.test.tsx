/** Tests for ParameterPanel's fetch/render/edit behaviour across product languages. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { PRODUCT_LANGUAGE_STORAGE_KEY, resetProductLanguageForTests } from "./productLanguage";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

// T3 fix round 1, item 6: every `ParameterPanelDto` fixture now needs a
// `tree` field. `null` is correct for a GET-only fixture (the server only
// attaches a tree to a successful write); a fixture also mocked as a
// `setParameterValue` response needs a concrete one, since `apply()`'s
// runtime guard throws on a write response with no tree.
const panelTree: ProjectTree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, is_modified: true, group_address_style: "ThreeLevel", installations: [],
};

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
  setParameterValue: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ParameterPanel from "./ParameterPanel";
import { resetSettingsForTests, setSetting } from "./settingsStore";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetProductLanguageForTests();
  resetUiLanguageForTests();
});

// Two sections (top-level + one module instantiation, D23), one stale
// entry (D21), one diagnostic (D26) — the exact fixture shape the brief's
// step 7 names.
const fixture: ParameterPanelDto = {
  programId: "PROG-1",
  sourceLanguage: null,
  sections: [
    {
      scope: null,
      fields: [
        {
          etsId: "P1",
          name: "Field A",
          nameLanguage: null,
          text: null,
          textLanguage: null,
          kind: "Number",
          value: "5",
          valueSource: "Stored",
          editable: true,
          min: "0",
          max: "10",
          enumOptions: [],
          displayOrder: null,
          access: null,
          writeEtsId: "P1",
        },
      ],
    },
    {
      // `moduleId: null` on purpose — pins D23's fallback label,
      // "Module #{module_node}".
      scope: { moduleNode: 7, moduleId: null, moduleDefId: "MD-1" },
      fields: [
        {
          etsId: "P2_M7_MI-1",
          name: "Field B",
          nameLanguage: null,
          text: null,
          textLanguage: null,
          kind: "Restriction",
          value: "1",
          valueSource: "Default",
          editable: false,
          min: null,
          max: null,
          enumOptions: [
            { value: "1", text: "On", language: null },
            { value: "0", text: "Off", language: null },
          ],
          displayOrder: null,
          access: null,
          writeEtsId: null,
        },
      ],
    },
  ],
  stale: [{ etsId: "P3", raw: "99" }],
  diagnostics: [
    {
      scope: null,
      kind: "noBranchMatched",
      severity: "info",
      message: "A choice did not match any of its options.",
      detail: "NoBranchMatched { choose_node: 4821 }",
    },
  ],
  tree: panelTree,
};

async function renderPanel(deviceId = 1, view: "parameters" | "diagnostics" | "restricted" = "parameters") {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ParameterPanel deviceId={deviceId} view={view} onValueApplied={() => {}} />);
  });
  return root;
}

function setInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function setSelectValue(select: HTMLSelectElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value")!.set!;
  setter.call(select, value);
  select.dispatchEvent(new Event("change", { bubbles: true }));
}

describe("ParameterPanel", () => {
  it("reloads the same device after a project snapshot and ignores the older pending reply", async () => {
    let resolveOld!: (panel: ParameterPanelDto) => void;
    apiMock.deviceParameters
      .mockImplementationOnce(() => new Promise<ParameterPanelDto>((resolve) => { resolveOld = resolve; }))
      .mockResolvedValueOnce({
        ...fixture, stale: [], diagnostics: [],
        sections: [{ ...fixture.sections[0], fields: [{ ...fixture.sections[0].fields[0], text: "NEW-PARAMETER" }] }],
      });
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(
      <ParameterPanel deviceId={17} refreshKey={panelTree} onValueApplied={() => {}} />,
    ));
    await act(async () => root.render(
      <ParameterPanel deviceId={17} refreshKey={{ ...panelTree }} onValueApplied={() => {}} />,
    ));
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(2);
    expect(host.textContent).toContain("NEW-PARAMETER");
    await act(async () => resolveOld({ ...fixture, sections: [] }));
    expect(host.textContent).toContain("NEW-PARAMETER");
    await act(async () => root.unmount());
  });

  it("renders field rows without evaluation prose or stored unmatched values", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    expect(apiMock.deviceParameters).toHaveBeenCalledWith(1, null);
    expect(host!.querySelectorAll(".parameter-field").length).toBe(2);
    expect(host!.textContent).toContain("Module #7");
    expect(host!.textContent).not.toContain("P3");
    expect(host!.textContent).not.toContain("99");
    expect(host!.querySelector(".parameter-diagnostics-banner")).toBeNull();

    root.unmount();
  });

  it("labels a no-branch diagnostic as information rather than a warning", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel(1, "diagnostics");
    const banner = host!.querySelector<HTMLDetailsElement>(".parameter-diagnostics-banner")!;
    expect(banner.querySelector("summary")?.textContent).toContain("1 note");
    expect(banner.querySelector("summary")?.textContent).not.toContain("warning");
    await act(async () => { banner.querySelector("summary")!.click(); });
    const item = banner.querySelector<HTMLLIElement>('li[data-severity="info"]');
    expect(item?.textContent).toContain("Info");
    expect(item?.textContent).not.toContain("Warning");
    root.unmount();
  });

  it.each([
    ["unknown", "future"],
    ["missing", undefined],
  ])("fails closed to a warning for a %s diagnostic severity", async (_case, wireSeverity) => {
    const response: ParameterPanelDto = {
      ...fixture,
      diagnostics: [{ ...fixture.diagnostics[0], severity: wireSeverity as "warning" }],
    };
    apiMock.deviceParameters.mockResolvedValue(response);
    const root = await renderPanel(1, "diagnostics");
    const banner = host!.querySelector(".parameter-diagnostics-banner")!;
    expect(banner.querySelector("summary")?.textContent).toContain("1 warning");
    expect(banner.querySelector('li[data-severity="warning"]')?.textContent).toContain("Warning");
    root.unmount();
  });

  it("copies every raw record in a repeated diagnostic group without duplicating the headline", async () => {
    const details = Array.from({ length: 6 }, (_, index) => `NoBranchMatched { choose_node: ${index} }`);
    apiMock.deviceParameters.mockResolvedValue({
      ...fixture, diagnostics: details.map((detail) => ({ ...fixture.diagnostics[0], detail })),
    });
    const root = await renderPanel(1, "diagnostics");
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    expect(host!.querySelectorAll("li[data-severity]")).toHaveLength(1);
    expect(host!.textContent).toContain("6 occurrences");
    expect([...host!.querySelectorAll(".parameter-diagnostic-details pre")].map((item) => item.textContent)).toEqual(details);
    await act(async () => host!.querySelector<HTMLButtonElement>(".parameter-diagnostics-banner button")!.click());
    expect(writeText).toHaveBeenCalledWith(details.join("\n"));
    root.unmount();
  });

  it("submits an edit on an editable field and re-renders from the mocked response", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const updated: ParameterPanelDto = {
      ...fixture,
      sections: [
        {
          scope: null,
          fields: [{ ...fixture.sections[0].fields[0], value: "6", valueSource: "Stored" }],
        },
        fixture.sections[1],
      ],
    };
    apiMock.setParameterValue.mockResolvedValue(updated);
    const root = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('input[type="number"]')!;
    await act(async () => {
      setInputValue(input, "6");
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });

    expect(apiMock.setParameterValue).toHaveBeenCalledWith(1, "P1", "6", null);
    expect(host!.querySelector<HTMLInputElement>('input[type="number"]')!.value).toBe("6");
    root.unmount();
  });

  it("renders a module-scoped field's input present but disabled", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>("select")!;
    expect(select).toBeTruthy();
    expect(select.disabled).toBe(true);

    root.unmount();
  });

  it("reverts to the prior value and surfaces the message on a rejected write", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    apiMock.setParameterValue.mockRejectedValue(new Error("out of range"));
    const root = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('input[type="number"]')!;
    await act(async () => {
      setInputValue(input, "99");
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });

    expect(apiMock.setParameterValue).toHaveBeenCalledWith(1, "P1", "99", null);
    expect(host!.querySelector<HTMLInputElement>('input[type="number"]')!.value).toBe("5");
    expect(host!.querySelector(".field-error")!.textContent).toBe("out of range");

    root.unmount();
  });

  it("renders the empty-program state while still rendering a non-empty stale list", async () => {
    const noProgram: ParameterPanelDto = {
      programId: null,
      sourceLanguage: null,
      sections: [],
      stale: [{ etsId: "P9", raw: "legacy-raw" }],
      diagnostics: [],
      tree: null,
    };
    apiMock.deviceParameters.mockResolvedValue(noProgram);
    const root = await renderPanel(1, "diagnostics");

    expect(host!.textContent).toContain("No parameter evaluation warnings or notes.");
    expect(host!.querySelectorAll(".parameter-field").length).toBe(0);
    expect(host!.textContent).toContain("P9");
    expect(host!.textContent).toContain("legacy-raw");

    root.unmount();
  });

  it("sends the active product language when loading the panel", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    expect(apiMock.deviceParameters).toHaveBeenCalledWith(1, "de");

    root.unmount();
  });

  it("sends the active product language when writing a value", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(fixture);
    apiMock.setParameterValue.mockResolvedValue(fixture);
    const root = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('input[type="number"]')!;
    await act(async () => {
      setInputValue(input, "6");
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });

    expect(apiMock.setParameterValue).toHaveBeenCalledWith(1, "P1", "6", "de");

    root.unmount();
  });

  it("renders the diagnostics count's plural branch for more than one diagnostic", async () => {
    const twoDiagnostics: ParameterPanelDto = {
      ...fixture,
      diagnostics: [
        fixture.diagnostics[0],
        {
          scope: null,
          kind: "unresolvedParamRef",
          severity: "warning",
          message: "A choice's controlling parameter could not be found.",
          detail: "UnresolvedParamRef { field: \"P2\" }",
        },
      ],
    };
    apiMock.deviceParameters.mockResolvedValue(twoDiagnostics);
    const root = await renderPanel(1, "diagnostics");

    const banner = host!.querySelector(".parameter-diagnostics-banner")!;
    expect(banner.querySelector("summary")?.textContent).toContain("1 warning");
    expect(banner.querySelector("summary")?.textContent).toContain("1 note");
    expect(banner.querySelector("summary")?.textContent).not.toContain("2 issues");
    const items = banner.querySelectorAll("li[data-severity]");
    expect([...items].map((item) => item.getAttribute("data-severity"))).toEqual(["info", "warning"]);
    expect(items[0].textContent).toContain("Info");
    expect(items[1].textContent).toContain("Warning");

    root.unmount();
  });

  it("renders a field's text in preference to its name", async () => {
    const withText: ParameterPanelDto = {
      ...fixture,
      sections: [
        {
          scope: null,
          fields: [{ ...fixture.sections[0].fields[0], name: "General", text: "Allgemein" }],
        },
        fixture.sections[1],
      ],
    };
    apiMock.deviceParameters.mockResolvedValue(withText);
    const root = await renderPanel();

    expect(host!.textContent).toContain("Allgemein");
    expect(host!.textContent).not.toContain("General");

    root.unmount();
  });

  it("renders a Time field as a bounded integer input like Number (PDB-9)", async () => {
    const timed: ParameterPanelDto = {
      ...fixture,
      sections: [
        {
          scope: null,
          fields: [{ ...fixture.sections[0].fields[0], kind: "Time", min: "0", max: "3600" }],
        },
      ],
    };
    apiMock.deviceParameters.mockResolvedValue(timed);
    const root = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('input[type="number"]')!;
    expect(input).not.toBeNull();
    expect(input.min).toBe("0");
    expect(input.max).toBe("3600");

    root.unmount();
  });

  it("writes a module-scoped field's writeEtsId, not its declared etsId", async () => {
    const scopedPanel: ParameterPanelDto = {
      programId: "PROG-1",
      sourceLanguage: null,
      sections: [
        {
          scope: { moduleNode: 7, moduleId: "M-7", moduleDefId: "MD-1" },
          fields: [
            {
              etsId: "P2",
              name: "Field B",
              nameLanguage: null,
              text: null,
              textLanguage: null,
              kind: "Number",
              value: "1",
              valueSource: "Default",
              editable: true,
              min: "0",
              max: "10",
              enumOptions: [],
              displayOrder: null,
              access: null,
              writeEtsId: "M-7_MI-3_P2",
            },
          ],
        },
      ],
      stale: [],
      diagnostics: [],
      tree: panelTree,
    };
    apiMock.deviceParameters.mockResolvedValue(scopedPanel);
    apiMock.setParameterValue.mockResolvedValue(scopedPanel);
    const root = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('input[type="number"]')!;
    await act(async () => {
      setInputValue(input, "2");
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });

    expect(apiMock.setParameterValue).toHaveBeenCalledWith(1, "M-7_MI-3_P2", "2", null);
    expect(apiMock.setParameterValue).not.toHaveBeenCalledWith(1, "P2", "2", null);

    root.unmount();
  });

  it("disables a field and refuses to submit it when writeEtsId is null, even if editable says true", async () => {
    // The server's contract makes `editable === false` and
    // `writeEtsId === null` exact opposites (never a third state), but
    // this fixture deliberately breaks that contract to prove the
    // control's own belt-and-braces check (`ParameterPanel.tsx`'s
    // `disabled`/`apply()` guards) does not simply trust `editable`.
    const contractBrokenPanel: ParameterPanelDto = {
      programId: "PROG-1",
      sourceLanguage: null,
      sections: [
        {
          scope: { moduleNode: 7, moduleId: "M-7", moduleDefId: "MD-1" },
          fields: [
            {
              etsId: "P2",
              name: "Field B",
              nameLanguage: null,
              text: null,
              textLanguage: null,
              kind: "Restriction",
              value: "1",
              valueSource: "Default",
              editable: true,
              min: null,
              max: null,
              enumOptions: [
                { value: "1", text: "On", language: null },
                { value: "0", text: "Off", language: null },
              ],
              displayOrder: null,
              access: null,
              writeEtsId: null,
            },
          ],
        },
      ],
      stale: [],
      diagnostics: [],
      tree: null,
    };
    apiMock.deviceParameters.mockResolvedValue(contractBrokenPanel);
    const root = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>("select")!;
    expect(select.disabled).toBe(true);

    await act(async () => {
      setSelectValue(select, "0");
      select.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });

    expect(apiMock.setParameterValue).not.toHaveBeenCalled();

    root.unmount();
  });

  it("keeps module reasons once in Diagnostics with their section identity", async () => {
    const reason = "No imported module instance matches this module; its fields are read-only.";
    const panelWithSectionDiagnostic: ParameterPanelDto = {
      ...fixture,
      diagnostics: [
        ...fixture.diagnostics,
        {
          scope: { moduleNode: 7, moduleId: null, moduleDefId: "MD-1" },
          kind: "noModuleInstanceMatch",
          severity: "warning",
          message: reason,
          detail: "No imported ModuleInstance's RefId matches module 'M-7' (D39 rule 2, zero matches).",
        },
      ],
    };
    apiMock.deviceParameters.mockResolvedValue(panelWithSectionDiagnostic);
    const root = await renderPanel();

    const sections = host!.querySelectorAll(".parameter-section");
    expect(sections.length).toBe(2);
    // The device-scope section (no matching diagnostic) must not show it.
    expect(sections[0].textContent).not.toContain(reason);
    // The reason is no longer repeated within module sections.
    expect(sections[1].textContent).not.toContain(reason);
    await act(async () => root.render(<ParameterPanel deviceId={1} view="diagnostics" onValueApplied={() => {}} />));
    const item = [...host!.querySelectorAll("li[data-severity]")].find((li) => li.textContent?.includes(reason));
    expect(item?.textContent).toContain("Module #7");

    root.unmount();
  });

  // KNOWN_LIMITATIONS.md §66: `ParameterDiagnostic.message` used to be a
  // fixed English sentence composed server-side and rendered verbatim.
  // It's now a `kind` tag translated client-side through
  // `describeParameterDiagnosticMessage` (`ParameterPanel.tsx`), the same
  // discriminated-union pattern §67 used for a rejected language pack's
  // own reason. This test proves the banner headline follows the active
  // UI language while `.detail` — developer-facing, carries raw Rust
  // debug formatting — stays English on purpose.
  it("§66: a diagnostic's message is translated with the UI language; its detail stays English", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel(1, "diagnostics");

    expect(host!.textContent).toContain(
      "Der aktuelle Steuerwert passt zu keiner Option; für diese Auswahl wurde kein Zweig gewählt.",
    );
    expect(host!.textContent).not.toContain(
      "A choice did not match any of its options.",
    );
    // Fix round 1 (Q3): `.detail` is rendered nowhere in the banner, so
    // `host!.textContent` never contains it whether or not translation
    // exists — that assertion held before this task and would hold under
    // any mutation of the translation path, which makes it evidence of
    // nothing. `.detail` reaches the user through exactly one door, the
    // "copy details" button's `copyDetail` handler (`ParameterPanel.tsx`);
    // asserting on that handler's clipboard payload is the real claim
    // `KNOWN_LIMITATIONS.md` cites this test for.
    const technical = host!.querySelector<HTMLDetailsElement>(".parameter-diagnostic-details")!;
    expect(technical.open).toBe(false);
    expect(technical.querySelector("pre")!.textContent).toBe("NoBranchMatched { choose_node: 4821 }");

    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    const copyButton = host!.querySelector<HTMLButtonElement>(
      ".parameter-diagnostics-banner button",
    )!;
    await act(async () => {
      copyButton.click();
    });
    // The clipboard payload is D26's raw diagnostic debug string,
    // untranslated, byte for byte — see `api.ts`'s doc comment on
    // `ParameterDiagnostic.detail`.
    expect(writeText).toHaveBeenCalledWith("NoBranchMatched { choose_node: 4821 }");

    root.unmount();
  });

  // ADR-0080 adoption (UI owner): the write-authority reasons are translated
  // like every other kind, and `Access=None` fields — no user right to view
  // or modify (Project Schema23 §1.1.2.1) — are folded away by default, with
  // their count, and stay one click from view; nothing is dropped.
  const field = (etsId: string, access: string | null, editable = access === null || access === "ReadWrite") => ({
    ...fixture.sections[0].fields[0], etsId, name: `Field ${etsId}`, access, editable,
    writeEtsId: editable ? etsId : null,
  });
  const authorityPanel = (fields: ParameterPanelDto["sections"][number]["fields"], kinds: string[]): ParameterPanelDto => ({
    ...fixture,
    sections: [{ scope: null, fields }],
    stale: [],
    diagnostics: kinds.map((kind) => ({
      scope: null, kind: kind as ParameterPanelDto["diagnostics"][number]["kind"], severity: "warning" as const,
      message: `server English for ${kind}`, detail: "2 field(s), effective Access is not ReadWrite: X, Y",
    })),
  });

  it.each([
    ["parameterAccessReadOnly", "Einige Felder hat der Hersteller schreibgeschützt oder verborgen (Access); sie sind nicht beschreibbar."],
    ["manufacturerCalculation", "Einige Felder sind Ein- oder Ausgaben einer Herstellerberechnung, die KNXBench nicht ausführt; sie sind schreibgeschützt."],
    ["writeAuthorityUnavailable", "Die Produktdatenbank hat für dieses Programm keine Schreibberechtigung erfasst; seine Felder sind schreibgeschützt. Installiere das Produkt neu, um sie zu erfassen."],
    ["unsupportedControlKind", "Der steuernde Parameter einer Auswahl hat einen nicht unterstützten Typ; ihre Zweige wurden nicht ausgewertet."],
    ["evaluationWorkBudgetExhausted", "Dieses Programm hat die Auswertungsgrenze überschritten; seine unvollständige Parameteransicht ist schreibgeschützt."],
  ])("translates the %s reason instead of showing the server's English", async (kind, german) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([field("A", "ReadWrite")], [kind]));
    const root = await renderPanel(1, "diagnostics");
    expect(host!.querySelector(".parameter-diagnostics-banner")!.textContent).toContain(german);
    expect(host!.textContent).not.toContain(`server English for ${kind}`);
    root.unmount();
  });

  it("moves manufacturer-restricted fields out of the editor, preserving their order and values", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel(
      [field("A", "ReadWrite"), field("B", "None"), field("C", "Read"), field("D", "None")], ["parameterAccessReadOnly"]));
    const root = await renderPanel();
    const ids = () => [...host!.querySelectorAll(".parameter-field")].map((row) => row.getAttribute("data-ets-id"));
    expect(ids()).toEqual(["A"]);
    expect(host!.querySelector(".parameter-hidden-toggle")).toBeNull();
    await act(async () => root.render(<ParameterPanel deviceId={1} view="restricted" onValueApplied={() => {}} />));
    expect(ids()).toEqual(["B", "C", "D"]);
    const inputs = [...host!.querySelectorAll<HTMLInputElement>("input")];
    expect(inputs.map((input) => input.value)).toEqual(["5", "5", "5"]);
    expect(inputs.every((input) => input.disabled)).toBe(true);
    expect(host!.textContent).toContain("Hidden by the manufacturer (Access None)");
    expect(host!.textContent).toContain("Read-only by the manufacturer (Access Read)");
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it.each(["Read", "None"])("refuses Access %s edits even if the response falsely claims write authority", async (access) => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([field("B", access, true)], []));
    const root = await renderPanel(1, "restricted");
    const input = host!.querySelector<HTMLInputElement>("input")!;
    expect(input.disabled).toBe(true);
    await act(async () => {
      setInputValue(input, "9");
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    });
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
    root.unmount();
  });

  it("states when no manufacturer-restricted fields were evaluated", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([field("A", "ReadWrite")], []));
    const root = await renderPanel(1, "restricted");
    expect(host!.textContent).toContain("No manufacturer-restricted fields in the evaluated sections.");
    expect(host!.querySelectorAll(".parameter-field")).toHaveLength(0);
    root.unmount();
  });

  it("keeps non-manufacturer read-only module fields in the editor with a Diagnostics pointer", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();
    expect(host!.querySelectorAll(".parameter-section")[1].textContent).toContain(
      "Shared across every instantiation of this module");
    expect(host!.querySelectorAll(".parameter-section")[1].textContent).toContain("diagnostics");
    root.unmount();
  });

  // AR10 slice 2a (`b6a94c24`): the server names the stored language that
  // answered each label (`textLanguage`, `nameLanguage`,
  // `enumOptions[].language`) or `null` when the package's own text was kept,
  // whose language is the panel's `sourceLanguage`. With a product language
  // selected, a label that fell back is marked; without one, nothing fell back.
  const lang = (
    etsId: string,
    label: { text?: string | null; textLanguage?: string | null; name?: string | null; nameLanguage?: string | null },
    options: { value: string; text: string | null; language: string | null }[] = [],
  ) => ({
    ...field(etsId, null),
    kind: options.length > 0 ? "Restriction" : "Number",
    text: label.text ?? null, textLanguage: label.textLanguage ?? null,
    name: label.name ?? null, nameLanguage: label.nameLanguage ?? null,
    enumOptions: options, value: options.length > 0 ? options[0].value : "1",
  });
  const languagePanel = (sourceLanguage: string | null, fields: ParameterPanelDto["sections"][number]["fields"]): ParameterPanelDto => ({
    ...authorityPanel(fields, []), sourceLanguage,
  });
  const mixedFields = () => [
    lang("A", { text: "Allgemein", textLanguage: "de-DE" }),
    lang("B", { text: "Channel A" }),
    lang("C", { text: "Modus", textLanguage: "de-DE" }, [
      { value: "1", text: "Ein", language: "de-DE" }, { value: "0", text: "Off", language: null }]),
    lang("D", { name: "Kanal", nameLanguage: "de-DE" }),
    lang("E", {}),
    lang("F", { text: "Allgemein", textLanguage: "de-DE", name: "General" }),
    // An option without a label shows its value; no translation can change that.
    lang("G", { text: "Stufe", textLanguage: "de-DE" }, [{ value: "2", text: null, language: null }]),
  ];
  const badgeOf = (id: string) =>
    host!.querySelector(`.parameter-field[data-ets-id="${id}"] .language-fallback-badge`);

  it("marks every label that fell back to the program's own text when a product language is selected", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(languagePanel("en-US", mixedFields()));
    const root = await renderPanel();
    expect(badgeOf("A")).toBeNull();
    expect(badgeOf("B")!.textContent).toBe("Untranslated (en-US)");
    expect(badgeOf("B")!.getAttribute("title")).toBe("No de translation is stored; this is the original text.");
    expect(badgeOf("C")!.textContent).toBe("Options untranslated (en-US)");
    expect(badgeOf("D")).toBeNull();
    expect(badgeOf("E")).toBeNull();
    expect(badgeOf("F")).toBeNull();
    expect(badgeOf("G")).toBeNull();
    expect(host!.querySelector(".parameter-language-summary")).toBeNull();
    await act(async () => root.render(<ParameterPanel deviceId={1} view="diagnostics" onValueApplied={() => {}} />));
    expect(host!.querySelector(".parameter-language-summary")!.textContent).toBe(
      "2 fields are not fully translated into de; they show the program's own text (en-US).");
    root.unmount();
  });

  it("marks nothing when no product language is selected", async () => {
    apiMock.deviceParameters.mockResolvedValue(languagePanel("en-US", mixedFields()));
    const root = await renderPanel();
    expect(host!.querySelectorAll(".language-fallback-badge").length).toBe(0);
    expect(host!.querySelector(".parameter-language-summary")).toBeNull();
    root.unmount();
  });

  it("does not guess the source language when the program declares none", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(languagePanel(null, [lang("B", { text: "Channel A" })]));
    const root = await renderPanel();
    expect(badgeOf("B")!.textContent).toBe("Untranslated");
    expect(host!.querySelector(".parameter-language-summary")).toBeNull();
    await act(async () => root.render(<ParameterPanel deviceId={1} view="diagnostics" onValueApplied={() => {}} />));
    expect(host!.querySelector(".parameter-language-summary")!.textContent).toBe(
      "1 field is not fully translated into de; it shows the program's own text.");
    root.unmount();
  });

  it("shows no summary when every label answered in the selected language", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(languagePanel("en-US", [lang("A", { text: "Allgemein", textLanguage: "de-DE" })]));
    const root = await renderPanel();
    expect(host!.querySelector(".parameter-language-summary")).toBeNull();
    expect(host!.querySelectorAll(".language-fallback-badge").length).toBe(0);
    root.unmount();
  });

  it("speaks German about untranslated labels", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(languagePanel("en-US", mixedFields()));
    const root = await renderPanel();
    expect(badgeOf("B")!.textContent).toBe("Unübersetzt (en-US)");
    expect(badgeOf("C")!.textContent).toBe("Optionen unübersetzt (en-US)");
    expect(host!.querySelector(".parameter-language-summary")).toBeNull();
    await act(async () => root.render(<ParameterPanel deviceId={1} view="diagnostics" onValueApplied={() => {}} />));
    expect(host!.querySelector(".parameter-language-summary")!.textContent).toBe(
      "2 Felder sind nicht vollständig in de übersetzt; sie zeigen den eigenen Text des Programms (en-US).");
    root.unmount();
  });

  // KL §128: kind None (ETS TypeNone, legacy atomic type 0) has no value.
  const displayOnly = (etsId: string, text: string | null, editable = false) => ({
    ...field(etsId, null, editable), kind: "None", value: null, name: "d_space", text,
  });

  it("renders a TypeNone heading as a label and a TypeNone spacer as empty space, with no inputs", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel(
      [displayOnly("H", "Timing"), displayOnly("S", ""), displayOnly("N", null), field("A", "ReadWrite")], []));
    const root = await renderPanel();
    const row = (id: string) => host!.querySelector(`.parameter-field[data-ets-id="${id}"]`)!;
    expect(row("H").classList.contains("parameter-label-row")).toBe(true);
    expect(row("H").textContent).toBe("Timing");
    for (const id of ["S", "N"]) {
      expect(row(id).classList.contains("parameter-spacer")).toBe(true);
      expect(row(id).textContent).toBe("");
      expect(row(id).getAttribute("aria-hidden")).toBe("true");
    }
    for (const id of ["H", "S", "N"]) {
      expect(row(id).querySelector("input, select")).toBeNull();
      expect(row(id).textContent).not.toContain("d_space");
    }
    expect(host!.querySelectorAll("input")).toHaveLength(1);
    expect(host!.textContent).not.toContain("Not editable here");
    root.unmount();
  });

  it("never offers a TypeNone row as an input, even if the response falsely claims it is editable", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([displayOnly("H", "Timing", true), displayOnly("S", "", true)], []));
    const root = await renderPanel();
    expect(host!.querySelectorAll("input, select")).toHaveLength(0);
    expect(apiMock.setParameterValue).not.toHaveBeenCalled();
    root.unmount();
  });

  it("counts an untranslated TypeNone heading but never an empty spacer", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(languagePanel("en-US", [displayOnly("H", "Timing"), displayOnly("S", "")]));
    const root = await renderPanel();
    expect(badgeOf("H")!.textContent).toBe("Untranslated (en-US)");
    expect(badgeOf("S")).toBeNull();
    await act(async () => root.render(<ParameterPanel deviceId={1} view="diagnostics" onValueApplied={() => {}} />));
    expect(host!.querySelector(".parameter-language-summary")!.textContent).toBe(
      "1 field is not fully translated into de; it shows the program's own text (en-US).");
    root.unmount();
  });
});
