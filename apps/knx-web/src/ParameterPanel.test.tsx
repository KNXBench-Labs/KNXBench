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
  schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, group_address_style: "ThreeLevel", installations: [],
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
  resetSettingsForTests();
  resetUiLanguageForTests();
});

// Two sections (top-level + one module instantiation, D23), one stale
// entry (D21), one diagnostic (D26) — the exact fixture shape the brief's
// step 7 names.
const fixture: ParameterPanelDto = {
  programId: "PROG-1",
  sections: [
    {
      scope: null,
      fields: [
        {
          etsId: "P1",
          name: "Field A",
          text: null,
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
          text: null,
          kind: "Restriction",
          value: "1",
          valueSource: "Default",
          editable: false,
          min: null,
          max: null,
          enumOptions: [
            { value: "1", text: "On" },
            { value: "0", text: "Off" },
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
  it("renders both sections' field rows, the stale entry, and the collapsed diagnostic count", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    expect(apiMock.deviceParameters).toHaveBeenCalledWith(1, null);
    expect(host!.querySelectorAll(".parameter-field").length).toBe(2);
    expect(host!.textContent).toContain("Module #7");
    expect(host!.textContent).toContain("P3");
    expect(host!.textContent).toContain("99");
    expect(host!.textContent).toContain(
      "1 issue found while evaluating this device's parameters",
    );

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
          message: "A choice's controlling parameter could not be found.",
          detail: "UnresolvedParamRef { field: \"P2\" }",
        },
      ],
    };
    apiMock.deviceParameters.mockResolvedValue(twoDiagnostics);
    const root = await renderPanel();

    expect(host!.textContent).toContain(
      "2 issues found while evaluating this device's parameters",
    );
    expect(host!.textContent).not.toContain(
      "1 issue found while evaluating this device's parameters",
    );

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

  it("writes a module-scoped field's writeEtsId, not its declared etsId", async () => {
    const scopedPanel: ParameterPanelDto = {
      programId: "PROG-1",
      sections: [
        {
          scope: { moduleNode: 7, moduleId: "M-7", moduleDefId: "MD-1" },
          fields: [
            {
              etsId: "P2",
              name: "Field B",
              text: null,
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
      sections: [
        {
          scope: { moduleNode: 7, moduleId: "M-7", moduleDefId: "MD-1" },
          fields: [
            {
              etsId: "P2",
              name: "Field B",
              text: null,
              kind: "Restriction",
              value: "1",
              valueSource: "Default",
              editable: true,
              min: null,
              max: null,
              enumOptions: [
                { value: "1", text: "On" },
                { value: "0", text: "Off" },
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
});
