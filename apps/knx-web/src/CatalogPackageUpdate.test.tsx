/** Tests that a chosen package is previewed and installed only on confirmation (ADR-0118, B5). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CatalogInstallReport, CatalogueUpdatePreview } from "./api";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const apiMock = vi.hoisted(() => ({
  previewCatalogUpdate: vi.fn(),
  applyCatalogUpdate: vi.fn(),
  // Re-implemented, not stubbed: the panel branches on it.
  isCatalogueUpdateStale: (e: unknown) => {
    const error = e as { status?: unknown; body?: { kind?: unknown } } | null;
    return error?.status === 409 && error.body?.kind === "catalogueUpdateStale";
  },
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => apiMock);

import CatalogPackageUpdate from "./CatalogPackageUpdate";

const preview: CatalogueUpdatePreview = {
  sourceName: "dimmer-v2.knxprod",
  packageSha256: "ab12".repeat(16),
  packageAlreadyInstalled: false,
  programs: [
    { id: "M-0001_A-0001-20-0001", manufacturerId: "M-0001", name: "Dimmer", applicationNumber: "1", applicationVersion: "2.0", status: "newVersion",
      installedVersions: [{ id: "M-0001_A-0001-10-0001", applicationVersion: "1.0" }] },
    { id: "M-0001_A-0002-10-0001", manufacturerId: "M-0001", name: null, applicationNumber: null, applicationVersion: null, status: "new", installedVersions: [] },
    { id: "M-0001_A-0003-10-0001", manufacturerId: "M-0001", name: "Switch", applicationNumber: "3", applicationVersion: "1.0", status: "alreadyInstalled", installedVersions: [] },
  ],
  productsAdded: 2, hardware2programsAdded: 3, idConflicts: 0, catalogueFingerprint: "f".repeat(64), token: "token-1",
};
const report: CatalogInstallReport = {
  sha256: preview.packageSha256, scheme: 20, skipped: false, members: [], unknown: 0, conflicts: 0,
  translations: { program: 0, catalog: 0, hardware: 0, master: 0 }, droppedDatapointTypes: 0, facts: null,
};
const BACKUP = "products.sqlite.backup-1760000000-ab12ab12ab12.sqlite";
const stale = Object.assign(new Error("the product database or the package changed since the preview; preview again"), { status: 409, body: { kind: "catalogueUpdateStale" } });

let roots: Root[] = [];
beforeEach(() => {
  for (const fn of [apiMock.previewCatalogUpdate, apiMock.applyCatalogUpdate]) fn.mockReset();
});
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren(); resetSettingsForTests(); resetUiLanguageForTests();
});

async function mount(chosen: File = file(), onLegacy = vi.fn()) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onInstalled = vi.fn(); const onDone = vi.fn();
  await act(async () => root.render(<CatalogPackageUpdate file={chosen} onInstalled={onInstalled} onDone={onDone} onLegacy={onLegacy} />));
  return { host, onInstalled, onDone, onLegacy, chosen };
}
function button(root: ParentNode, label: string) {
  const found = Array.from(root.querySelectorAll("button")).find((b) => b.textContent === label);
  expect(found, `missing button ${label}`).toBeDefined();
  return found!;
}
const file = () => new File(["package"], "dimmer-v2.knxprod");

describe("catalogue update with preview", () => {
  it("previews a chosen package and installs nothing yet", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    const { host, onInstalled, chosen } = await mount();
    expect(apiMock.previewCatalogUpdate).toHaveBeenCalledWith(chosen);
    const text = host.textContent!;
    expect(text).toContain("Update preview: dimmer-v2.knxprod");
    expect(text).toContain("Nothing is installed yet.");
    expect(text).toContain("Dimmer");
    expect(text).toContain("New version");
    expect(text).toContain("Installed: 1.0");
    expect(text).toContain("M-0001_A-0002-10-0001");
    expect(text).toContain("Already installed");
    expect(text).toContain("Adds 2 products and 3 hardware–program links.");
    expect(text).toContain("Existing projects keep their exact application programs");
    expect(text).toContain(preview.packageSha256);
    expect(apiMock.applyCatalogUpdate).not.toHaveBeenCalled();
    expect(onInstalled).not.toHaveBeenCalled();
  });

  it("backs up and installs with the same file and token, then reloads", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockResolvedValue({ applied: true, preview, backup: BACKUP, report });
    const { host, onInstalled, chosen } = await mount();
    await act(async () => button(host, "Back up and install").click());
    expect(apiMock.applyCatalogUpdate).toHaveBeenCalledWith(chosen, "token-1");
    expect(host.textContent).toContain(`Installed. The previous catalogue was saved as ${BACKUP}.`);
    expect(host.textContent).toContain("Installed: scheme 20");
    expect(onInstalled).toHaveBeenCalledTimes(1);
  });

  it("sends one install while the first is still running", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockReturnValue(new Promise(() => {}));
    const { host } = await mount();
    const apply = button(host, "Back up and install");
    await act(async () => { apply.click(); apply.click(); });
    expect(apiMock.applyCatalogUpdate).toHaveBeenCalledTimes(1);
    expect(host.textContent).toContain("Backing up and installing…");
  });

  it("cancels a preview without installing", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    const { host, onDone } = await mount();
    await act(async () => button(host, "Cancel").click());
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(apiMock.applyCatalogUpdate).not.toHaveBeenCalled();
  });

  it("says a stale preview installed nothing and previews the same file again on request", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockRejectedValue(stale);
    const { host, onInstalled, chosen } = await mount();
    await act(async () => button(host, "Back up and install").click());
    expect(host.textContent).toContain("The catalogue or the file changed since the preview. Nothing was installed.");
    expect(onInstalled).not.toHaveBeenCalled();
    await act(async () => button(host, "Preview again").click());
    expect(apiMock.previewCatalogUpdate).toHaveBeenCalledTimes(2);
    expect(apiMock.previewCatalogUpdate).toHaveBeenLastCalledWith(chosen);
    expect(apiMock.applyCatalogUpdate).toHaveBeenCalledTimes(1);
  });

  it("shows a refused preview with the server's reason", async () => {
    apiMock.previewCatalogUpdate.mockRejectedValue(new Error("not a KNX product package"));
    const { host } = await mount();
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("not a KNX product package");
    expect(host.textContent).not.toContain("Back up and install");
  });

  it("notes conflicts and an already installed package", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview: { ...preview, idConflicts: 4, packageAlreadyInstalled: true } });
    const { host } = await mount();
    expect(host.textContent).toContain("4 ID conflicts with installed data.");
    expect(host.textContent).toContain("This package is already installed; installing it again changes nothing.");
  });
});

describe("a product file that is no package", () => {
  it("hands a legacy product database on and installs nothing as a package", async () => {
    const legacy = Object.assign(new Error("a legacy product database"), { status: 422, body: { kind: "legacyProductDatabase" } });
    apiMock.previewCatalogUpdate.mockRejectedValue(legacy);
    const { host, onLegacy, chosen } = await mount(new File(["x"], "renamed.knxprod"));
    expect(onLegacy).toHaveBeenCalledWith(chosen);
    expect(host.querySelector('[role="alert"]')).toBeNull();
    expect(apiMock.applyCatalogUpdate).not.toHaveBeenCalled();
  });
});

it("speaks German when the interface does", async () => {
  saveUiLanguage(settingsStorage, "de"); resetUiLanguageForTests();
  apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
  const { host } = await mount();
  expect(host.textContent).toContain("Noch ist nichts installiert.");
  expect(host.textContent).toContain("Sichern und installieren");
});
