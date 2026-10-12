/** Previewed catalogue update and backup restore in the catalogue view (ADR-0118). */
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
  catalogBackups: vi.fn(),
  restoreCatalogBackup: vi.fn(),
  // Re-implemented, not stubbed: the panel branches on it.
  isCatalogueUpdateStale: (e: unknown) => {
    const error = e as { status?: unknown; body?: { kind?: unknown } } | null;
    return error?.status === 409 && error.body?.kind === "catalogueUpdateStale";
  },
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => apiMock);

import CatalogUpdatePanel from "./CatalogUpdatePanel";

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
  for (const fn of [apiMock.previewCatalogUpdate, apiMock.applyCatalogUpdate, apiMock.catalogBackups, apiMock.restoreCatalogBackup]) fn.mockReset();
  apiMock.catalogBackups.mockResolvedValue({ backups: [] });
});
afterEach(async () => {
  await act(async () => roots.forEach((root) => root.unmount()));
  roots = []; document.body.replaceChildren(); resetSettingsForTests(); resetUiLanguageForTests();
});

async function mount() {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); roots.push(root);
  const onInstalled = vi.fn();
  await act(async () => root.render(<CatalogUpdatePanel onInstalled={onInstalled} />));
  return { host, onInstalled };
}
async function choose(host: Element, file: File) {
  const input = host.querySelector<HTMLInputElement>('input[type="file"]');
  expect(input).not.toBeNull();
  Object.defineProperty(input!, "files", { value: [file], configurable: true });
  await act(async () => input!.dispatchEvent(new Event("change", { bubbles: true })));
}
async function openBackups(host: Element) {
  const details = host.querySelector<HTMLDetailsElement>("details.catalog-backups");
  expect(details).not.toBeNull();
  await act(async () => { details!.open = true; details!.dispatchEvent(new Event("toggle")); });
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
    const { host, onInstalled } = await mount();
    const chosen = file();
    await choose(host, chosen);
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
    apiMock.catalogBackups.mockResolvedValueOnce({ backups: [] }).mockResolvedValueOnce({ backups: [{ name: BACKUP, bytes: 2048, modifiedUnixSeconds: 1760000000 }] });
    const { host, onInstalled } = await mount();
    await openBackups(host);
    const chosen = file();
    await choose(host, chosen);
    await act(async () => button(host, "Back up and install").click());
    expect(apiMock.applyCatalogUpdate).toHaveBeenCalledWith(chosen, "token-1");
    expect(host.textContent).toContain(`Installed. The previous catalogue was saved as ${BACKUP}.`);
    expect(host.textContent).toContain("Installed: scheme 20");
    expect(onInstalled).toHaveBeenCalledTimes(1);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(2);
    expect(host.textContent).toContain("Catalogue backups (1)");
  });

  it("sends one install while the first is still running", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockReturnValue(new Promise(() => {}));
    const { host } = await mount();
    await choose(host, file());
    const apply = button(host, "Back up and install");
    await act(async () => { apply.click(); apply.click(); });
    expect(apiMock.applyCatalogUpdate).toHaveBeenCalledTimes(1);
    expect(host.textContent).toContain("Backing up and installing…");
  });

  it("cancels a preview without installing", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    const { host } = await mount();
    await choose(host, file());
    await act(async () => button(host, "Cancel").click());
    expect(host.textContent).not.toContain("Update preview");
    expect(apiMock.applyCatalogUpdate).not.toHaveBeenCalled();
  });

  it("says a stale preview installed nothing and previews the same file again on request", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockRejectedValue(stale);
    const { host, onInstalled } = await mount();
    const chosen = file();
    await choose(host, chosen);
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
    await choose(host, file());
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("not a KNX product package");
    expect(host.textContent).not.toContain("Back up and install");
  });

  it("notes conflicts and an already installed package", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview: { ...preview, idConflicts: 4, packageAlreadyInstalled: true } });
    const { host } = await mount();
    await choose(host, file());
    expect(host.textContent).toContain("4 ID conflicts with installed data.");
    expect(host.textContent).toContain("This package is already installed; installing it again changes nothing.");
  });
});

describe("catalogue backups", () => {
  const listed = { backups: [
    { name: BACKUP, bytes: 2048, modifiedUnixSeconds: 1760000000 },
    { name: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite", bytes: 1048576, modifiedUnixSeconds: 1750000000 },
  ] };

  it("reads backups only when the list is opened", async () => {
    const { host } = await mount();
    expect(apiMock.catalogBackups).not.toHaveBeenCalled();
    expect(host.textContent).toContain("Catalogue backups");
    await openBackups(host);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(1);
    await openBackups(host);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(1);
  });

  it("does not read backups after an update while the list was never opened", async () => {
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    apiMock.applyCatalogUpdate.mockResolvedValue({ applied: true, preview, backup: BACKUP, report });
    const { host } = await mount();
    await choose(host, file());
    await act(async () => button(host, "Back up and install").click());
    expect(apiMock.catalogBackups).not.toHaveBeenCalled();
  });

  it("lists backups in the server's order, or says there are none", async () => {
    const empty = await mount();
    await openBackups(empty.host);
    expect(empty.host.textContent).toContain("No backups yet. Each update with preview creates one.");
    apiMock.catalogBackups.mockResolvedValue(listed);
    const { host } = await mount();
    await openBackups(host);
    const names = Array.from(host.querySelectorAll(".catalog-backup-name")).map((n) => n.textContent);
    expect(names).toEqual([BACKUP, "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite"]);
    expect(host.textContent).toContain("Catalogue backups (2)");
  });

  it("asks before restoring, with keeping the catalogue as the safe default", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    const { host } = await mount();
    await openBackups(host);
    await act(async () => button(host, "Restore…").click());
    const dialog = document.querySelector('[role="dialog"]')!;
    expect(dialog.textContent).toContain(BACKUP);
    expect(dialog.textContent).toContain("Devices in your projects keep their references");
    expect(document.activeElement?.textContent).toBe("Keep the current catalogue");
    await act(async () => button(dialog, "Keep the current catalogue").click());
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    await act(async () => button(host, "Restore…").click());
    await act(async () => document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(apiMock.restoreCatalogBackup).not.toHaveBeenCalled();
  });

  it("restores exactly the confirmed backup, then reloads", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    apiMock.restoreCatalogBackup.mockResolvedValue({ restored: BACKUP });
    const { host, onInstalled } = await mount();
    await openBackups(host);
    await act(async () => button(host, "Restore…").click());
    await act(async () => button(document.querySelector('[role="dialog"]')!, "Restore backup").click());
    expect(apiMock.restoreCatalogBackup).toHaveBeenCalledWith(BACKUP);
    expect(host.textContent).toContain(`Restored ${BACKUP}.`);
    expect(onInstalled).toHaveBeenCalledTimes(1);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(2);
  });

  it("shows a refused restore and keeps the list", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    apiMock.restoreCatalogBackup.mockRejectedValue(new Error("not a usable product database backup: integrity check: bad"));
    const { host, onInstalled } = await mount();
    await openBackups(host);
    await act(async () => button(host, "Restore…").click());
    await act(async () => button(document.querySelector('[role="dialog"]')!, "Restore backup").click());
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("Restore refused: not a usable product database backup");
    expect(onInstalled).not.toHaveBeenCalled();
    expect(host.querySelectorAll(".catalog-backup-name")).toHaveLength(2);
  });

  it("speaks German when the interface does", async () => {
    saveUiLanguage(settingsStorage, "de"); resetUiLanguageForTests();
    apiMock.previewCatalogUpdate.mockResolvedValue({ applied: false, preview });
    const { host } = await mount();
    expect(host.textContent).toContain("Mit Vorschau aktualisieren…");
    await choose(host, file());
    expect(host.textContent).toContain("Noch ist nichts installiert.");
    expect(host.textContent).toContain("Sichern und installieren");
  });
});
