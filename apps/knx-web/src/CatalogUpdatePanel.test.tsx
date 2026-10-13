/** Catalogue backups: listed, restored and deleted only after asking (ADR-0118, B5). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const apiMock = vi.hoisted(() => ({
  catalogBackups: vi.fn(),
  restoreCatalogBackup: vi.fn(),
  deleteCatalogBackup: vi.fn(),
  isCatalogueBackupChanged: (e: unknown) => {
    const error = e as { status?: unknown; body?: { kind?: unknown } } | null;
    return error?.status === 409 && error.body?.kind === "catalogueBackupChanged";
  },

  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
}));
vi.mock("./api", () => apiMock);

import CatalogUpdatePanel from "./CatalogUpdatePanel";

const BACKUP = "products.sqlite.backup-1760000000-ab12ab12ab12.sqlite";

let roots: Root[] = [];
beforeEach(() => {
  for (const fn of [apiMock.catalogBackups, apiMock.restoreCatalogBackup,
    apiMock.deleteCatalogBackup]) fn.mockReset();
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
  const show = (installRevision: number) => act(async () => root.render(<CatalogUpdatePanel onInstalled={onInstalled} installRevision={installRevision} />));
  await show(0);
  return { host, onInstalled, rerender: show };
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

describe("catalogue backups", () => {
  const listed = { backups: [
    { name: BACKUP, bytes: 2048, modifiedUnixSeconds: 1760000000, identity: "id-newest" },
    { name: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite", bytes: 1048576, modifiedUnixSeconds: 1750000000, identity: "id-older" },
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

  it("reads backups again after an install only once the list was opened", async () => {
    const { host, rerender } = await mount();
    await rerender(1);
    expect(apiMock.catalogBackups).not.toHaveBeenCalled();
    await openBackups(host);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(1);
    await rerender(2);
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(2);
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
    apiMock.catalogBackups.mockResolvedValue(listed);
    const { host } = await mount();
    await openBackups(host);
    expect(host.textContent).toContain("Katalogsicherungen (2)");
    expect(host.textContent).toContain("Löschen…");
  });

  // Goal B5: a backup goes only after asking, and only the very file that was listed.
  it("asks before deleting, with keeping the backup as the safe default", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    const { host } = await mount();
    await openBackups(host);
    await act(async () => button(host.querySelectorAll(".catalog-backup-list li")[1], "Delete…").click());
    const dialog = document.querySelector('[role="dialog"]')!;
    expect(dialog.textContent).toContain("products.sqlite.backup-1750000000-cd34cd34cd34.sqlite");
    expect(dialog.textContent).toContain("cannot be undone");
    expect(document.activeElement?.textContent).toBe("Keep the backup");
    await act(async () => document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(apiMock.deleteCatalogBackup).not.toHaveBeenCalled();
  });

  it("deletes exactly the listed file, then reads the list again", async () => {
    apiMock.catalogBackups.mockResolvedValueOnce(listed).mockResolvedValueOnce({ backups: [listed.backups[0]] });
    apiMock.deleteCatalogBackup.mockResolvedValue({ deleted: listed.backups[1].name });
    const { host, onInstalled } = await mount();
    await openBackups(host);
    await act(async () => button(host.querySelectorAll(".catalog-backup-list li")[1], "Delete…").click());
    await act(async () => button(document.querySelector('[role="dialog"]')!, "Delete backup").click());
    expect(apiMock.deleteCatalogBackup).toHaveBeenCalledWith(listed.backups[1].name, "id-older");
    expect(host.textContent).toContain(`Deleted ${listed.backups[1].name}.`);
    expect(Array.from(host.querySelectorAll(".catalog-backup-name")).map((n) => n.textContent)).toEqual([BACKUP]);
    // The catalogue itself did not change.
    expect(onInstalled).not.toHaveBeenCalled();
  });

  it("keeps a backup that changed since the list was read, and shows the fresh list", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    apiMock.deleteCatalogBackup.mockRejectedValue(Object.assign(new Error("this backup changed since it was listed"),
      { status: 409, body: { kind: "catalogueBackupChanged" } }));
    const { host } = await mount();
    await openBackups(host);
    await act(async () => button(host.querySelectorAll(".catalog-backup-list li")[0], "Delete…").click());
    await act(async () => button(document.querySelector('[role="dialog"]')!, "Delete backup").click());
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("changed since the list was read");
    expect(apiMock.catalogBackups).toHaveBeenCalledTimes(2);
    expect(host.querySelectorAll(".catalog-backup-name")).toHaveLength(2);
  });

  it("says why a deletion failed and keeps the list", async () => {
    apiMock.catalogBackups.mockResolvedValue(listed);
    apiMock.deleteCatalogBackup.mockRejectedValue(new Error("the backup could not be deleted: permission denied"));
    const { host } = await mount();
    await openBackups(host);
    await act(async () => button(host.querySelectorAll(".catalog-backup-list li")[0], "Delete…").click());
    await act(async () => button(document.querySelector('[role="dialog"]')!, "Delete backup").click());
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("permission denied");
    expect(host.querySelectorAll(".catalog-backup-name")).toHaveLength(2);
  });

  it("offers no update picker of its own any more", async () => {
    const { host } = await mount();
    expect(host.querySelector('input[type="file"]')).toBeNull();
  });
});
