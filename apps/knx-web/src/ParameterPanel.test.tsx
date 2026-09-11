// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterPanel as ParameterPanelDto } from "./api";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
  setParameterValue: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ParameterPanel from "./ParameterPanel";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
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
        },
      ],
    },
  ],
  stale: [{ etsId: "P3", raw: "99" }],
  diagnostics: [
    {
      scope: null,
      message: "A choice did not match any of its options.",
      detail: "NoBranchMatched { choose_node: 4821 }",
    },
  ],
};

async function renderPanel(deviceId = 1) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ParameterPanel deviceId={deviceId} />);
  });
  return root;
}

function setInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("ParameterPanel", () => {
  it("renders both sections' field rows, the stale entry, and the collapsed diagnostic count", async () => {
    apiMock.deviceParameters.mockResolvedValue(fixture);
    const root = await renderPanel();

    expect(apiMock.deviceParameters).toHaveBeenCalledWith(1);
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

    expect(apiMock.setParameterValue).toHaveBeenCalledWith(1, "P1", "6");
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
});
