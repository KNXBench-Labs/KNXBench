/** ProductInstallControl: legacy password prompt, remember, and report wording. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LegacyInstallReport } from "./api";

const apiMock = vi.hoisted(() => ({
  installProductPackage: vi.fn(),
  installLegacyProductDatabase: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ProductInstallControl from "./ProductInstallControl";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

function refusal(kind: string) {
  return Object.assign(new Error(`server says ${kind}`), { status: 422, body: { error: kind, kind } });
}

function report(overrides: Partial<LegacyInstallReport> = {}): LegacyInstallReport {
  return {
    payloadSha256: "a".repeat(64),
    originalSha256: "b".repeat(64),
    namespace: "LX1A2B3C4D",
    skipped: false,
    programs: ["M-1092_A-LX1A2B3C4D-300"],
    catalogItems: 1,
    parameters: 10,
    parameterRefs: 12,
    comObjectRefs: 3,
    translations: 8,
    diagnostics: [{ kind: "secret-withheld", detail: "1 value of device.DEVICE_BCU_PASSWORD withheld" }],
    password: "given",
    remembered: false,
    rememberProblem: null,
    ...overrides,
  };
}

async function render(onInstalled = vi.fn()) {
  host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ProductInstallControl className="catalog-install" label="Install" onInstalled={onInstalled} />);
  });
  return { root, onInstalled };
}

async function pick(name: string) {
  const input = host!.querySelector<HTMLInputElement>('input[type="file"]')!;
  const file = new File([new Uint8Array([1])], name);
  await act(async () => {
    Object.defineProperty(input, "files", { value: [file], configurable: true });
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  return file;
}

function dialog() {
  return document.body.querySelector<HTMLElement>('[aria-labelledby="legacy-password-title"]');
}

async function submitPassword(password: string, remember: boolean) {
  const panel = dialog()!;
  const field = panel.querySelector<HTMLInputElement>('input[type="password"]')!;
  const box = panel.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  await act(async () => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setter.call(field, password);
    field.dispatchEvent(new Event("input", { bubbles: true }));
    if (remember) box.click();
  });
  await act(async () => {
    panel.querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
  });
}

describe("ProductInstallControl", () => {
  it("asks for the password of an encrypted legacy database and imports with it", async () => {
    apiMock.installLegacyProductDatabase
      .mockRejectedValueOnce(refusal("legacyPasswordRequired"))
      .mockResolvedValueOnce(report({ remembered: true }));
    const { root, onInstalled } = await render();
    const file = await pick("marvin.vd4");
    expect(apiMock.installProductPackage).not.toHaveBeenCalled();
    expect(apiMock.installLegacyProductDatabase).toHaveBeenLastCalledWith(file, undefined, false);
    expect(dialog()!.textContent).toContain("Password for marvin.vd4");
    expect(dialog()!.textContent).toContain("is encrypted");
    expect(onInstalled).not.toHaveBeenCalled();

    await submitPassword("canary-typed", true);
    expect(apiMock.installLegacyProductDatabase).toHaveBeenLastCalledWith(file, "canary-typed", true);
    expect(dialog()).toBeNull();
    expect(onInstalled).toHaveBeenCalledTimes(1);
    const text = host!.textContent ?? "";
    expect(text).toContain("Legacy product database imported: 1 application program.");
    expect(text).toContain("It is now remembered.");
    expect(text).toContain("secret-withheld");
    expect(text).toContain("No such download has run on a real device yet, so readiness shows them as untested.");
    expect(document.body.innerHTML).not.toContain("canary-typed");
    root.unmount();
  });

  it("says when the password was wrong or the remembered one does not fit", async () => {
    apiMock.installLegacyProductDatabase
      .mockRejectedValueOnce(refusal("legacyRememberedPasswordDoesNotFit"))
      .mockRejectedValueOnce(refusal("legacyWrongPassword"));
    const { root } = await render();
    await pick("old.vd3");
    expect(dialog()!.textContent).toContain("The remembered password does not open this file");
    await submitPassword("canary-wrong", false);
    expect(dialog()!.textContent).toContain("The password does not open this file");
    expect(dialog()!.querySelector('input[type="password"]')!.getAttribute("aria-invalid")).toBe("true");
    await act(async () => {
      Array.from(dialog()!.querySelectorAll("button")).find((b) => b.textContent === "Cancel")!.click();
    });
    expect(dialog()).toBeNull();
    expect(document.body.innerHTML).not.toContain("canary-wrong");
    root.unmount();
  });

  it("sends a renamed legacy database on to the legacy installer", async () => {
    apiMock.installProductPackage.mockRejectedValueOnce(refusal("legacyProductDatabase"));
    apiMock.installLegacyProductDatabase.mockResolvedValueOnce(report({ password: "remembered", skipped: true }));
    const { root } = await render();
    await pick("renamed.knxprod");
    expect(apiMock.installLegacyProductDatabase).toHaveBeenCalledTimes(1);
    const text = host!.textContent ?? "";
    expect(text).toContain("was already imported (1 application program)");
    expect(text).toContain("Opened with the remembered password.");
    root.unmount();
  });

  it("shows any other failure and a remember problem plainly", async () => {
    apiMock.installLegacyProductDatabase
      .mockRejectedValueOnce(new Error("not an EX-IM file"))
      .mockResolvedValueOnce(report({ rememberProblem: "no configuration directory" }));
    const { root } = await render();
    await pick("broken.vd5");
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe("not an EX-IM file");
    expect(dialog()).toBeNull();
    await pick("fine.vd5");
    expect(host!.querySelector('[role="alert"]')).toBeNull();
    expect(host!.textContent).toContain("The password was not remembered: no configuration directory");
    root.unmount();
  });
});
