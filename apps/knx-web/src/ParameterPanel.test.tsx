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

async function renderPanel(deviceId = 1) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ParameterPanel deviceId={deviceId} onValueApplied={() => {}} />);
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

  it("renders both sections' field rows, the stale entry, and the collapsed diagnostic count", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    expect(apiMock.deviceParameters).toHaveBeenCalledWith(1, null);
    expect(host!.querySelectorAll(".parameter-field").length).toBe(2);
    expect(host!.textContent).toContain("Module #7");
    expect(host!.textContent).toContain("P3");
    expect(host!.textContent).toContain("99");
    expect(host!.textContent).toContain("1 note");

    root.unmount();
  });

  it("labels a no-branch diagnostic as information rather than a warning", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();
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
    const root = await renderPanel();
    const banner = host!.querySelector(".parameter-diagnostics-banner")!;
    expect(banner.querySelector("summary")?.textContent).toContain("1 warning");
    expect(banner.querySelector('li[data-severity="warning"]')?.textContent).toContain("Warning");
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
    const root = await renderPanel();

    expect(host!.textContent).toContain(
      "This device has no resolvable application program",
    );
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
    const root = await renderPanel();

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

  it("shows a read-only section's own diagnostic reason inside that section", async () => {
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
    // The module-scoped section (matching scope) must.
    expect(sections[1].textContent).toContain(reason);

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
    const root = await renderPanel();

    expect(host!.textContent).toContain(
      "Eine Auswahl passte auf keine ihrer Optionen.",
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
    expect(host!.textContent).not.toContain("NoBranchMatched");

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
    const root = await renderPanel();
    expect(host!.querySelector(".parameter-section")!.textContent).toContain(german);
    expect(host!.textContent).not.toContain(`server English for ${kind}`);
    root.unmount();
  });

  it("folds Access=None fields away by default, counts them, and shows them read-only on request", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel(
      [field("A", "ReadWrite"), field("B", "None"), field("C", "Read"), field("D", "None")], ["parameterAccessReadOnly"]));
    const root = await renderPanel();
    const section = host!.querySelector(".parameter-section")!;
    const shown = () => [...section.querySelectorAll(".parameter-field")].map((row) => row.getAttribute("data-ets-id"));
    expect(shown()).toEqual(["A", "C"]);
    const toggle = section.querySelector<HTMLButtonElement>(".parameter-hidden-toggle")!;
    expect(toggle.textContent).toBe("Show 2 fields without user access (Access None)");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    await act(async () => toggle.click());
    expect(shown()).toEqual(["A", "B", "C", "D"]);
    expect(toggle.textContent).toBe("Hide 2 fields without user access (Access None)");
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    for (const id of ["B", "C", "D"]) {
      const row = section.querySelector(`.parameter-field[data-ets-id="${id}"]`)!;
      expect(row.querySelector<HTMLInputElement | HTMLSelectElement>("input, select")!.disabled).toBe(true);
    }
    root.unmount();
  });

  it("offers no fold for a section without Access=None fields", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([field("A", "ReadWrite"), field("C", "Read")], []));
    const root = await renderPanel();
    expect(host!.querySelector(".parameter-hidden-toggle")).toBeNull();
    expect(host!.querySelectorAll(".parameter-field").length).toBe(2);
    root.unmount();
  });

  it("counts a single hidden field in the singular", async () => {
    apiMock.deviceParameters.mockResolvedValue(authorityPanel([field("B", "None")], ["parameterAccessReadOnly"]));
    const root = await renderPanel();
    expect(host!.querySelector(".parameter-hidden-toggle")!.textContent).toBe("Show 1 field without user access (Access None)");
    expect(host!.querySelectorAll(".parameter-field").length).toBe(0);
    root.unmount();
  });

  it("does not call a device-level read-only field shared across module instantiations", async () => {
    apiMock.deviceParameters.mockResolvedValue({
      ...authorityPanel([field("C", "Read")], ["parameterAccessReadOnly"]),
      sections: [
        { scope: null, fields: [field("C", "Read")] },
        fixture.sections[1],
      ],
    });
    const root = await renderPanel();
    const [device, module] = [...host!.querySelectorAll(".parameter-section")];
    expect(device.querySelector(".parameter-field-caption")!.textContent).toBe(
      "Not editable here — see the warnings for why.");
    expect(module.querySelector(".parameter-field-caption")!.textContent).toContain(
      "Shared across every instantiation of this module");
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
    expect(host!.querySelector(".parameter-language-summary")!.textContent).toBe(
      "2 Felder sind nicht vollständig in de übersetzt; sie zeigen den eigenen Text des Programms (en-US).");
    root.unmount();
  });
});
