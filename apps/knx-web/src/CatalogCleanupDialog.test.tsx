/** Cleanup selection, preview authority, recovery and late-response regression tests. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CatalogItem } from "./api";
import { resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests } from "./settingsStore";

const api = vi.hoisted(() => ({ catalogItems: vi.fn(), previewCatalogCleanup: vi.fn(), applyCatalogCleanup: vi.fn(),
  errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error),
  // The real predicate, not a stub: only a 409 of this kind is stale.
  isCatalogueCleanupStale: (e: unknown) => {
    const error = e as { status?: unknown; body?: { kind?: unknown } } | null;
    return error?.status === 409 && error.body?.kind === "catalogueCleanupStale";
  } }));
vi.mock("./api", () => api);
import CatalogCleanupDialog from "./CatalogCleanupDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const items: CatalogItem[] = ["cat-a", "cat-b", "cat-v2"].map((id, index) => ({ id, manufacturerId: "M", name: "Actuator",
  number: "1", visibleDescription: null, productRefId: "P", hardware2programRefId: index === 2 ? "HP2" : "HP1",
  nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null,
  details: { applicationProgramId: index === 2 ? "APP2" : "APP1", applicationName: "Switch", applicationVersion: index === 2 ? "32" : "16",
    catalogSectionId: `section-${index}`, sourceNames: [`source-${index}.knxprod`] } }));
let root: Root;
let host: HTMLDivElement;
let onRemoved: ReturnType<typeof vi.fn<(result: { removedItemIds: string[]; backup: string }) => void | Promise<void>>>;
let onClose: ReturnType<typeof vi.fn<() => void>>;
beforeEach(() => {
  vi.resetAllMocks(); resetUiLanguageForTests(); resetSettingsForTests();
  api.catalogItems.mockResolvedValue(items);
  api.previewCatalogCleanup.mockResolvedValue({ token: "bound-token",
    items: [{ ...items[1], impact: { lastPlacementForBinding: false, openProjectDevices: 0 } }] });
  api.applyCatalogCleanup.mockResolvedValue({ removedItemIds: ["cat-b"], backup: "products.sqlite.backup-1-abcdefabcdef.sqlite" });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  onRemoved = vi.fn<(result: { removedItemIds: string[]; backup: string }) => void | Promise<void>>(); onClose = vi.fn<() => void>();
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });
async function render() { await act(async () => root.render(<CatalogCleanupDialog onRemoved={onRemoved} onClose={onClose} />)); }
function button(text: string) { const b = Array.from(host.querySelectorAll("button")).find((b) => b.textContent === text); expect(b, text).toBeDefined(); return b!; }
async function click(text: string) { await act(async () => button(text).click()); }
async function select(id: string) { await act(async () => host.querySelector<HTMLInputElement>(`input[value="${id}"]`)!.click()); }

describe("CatalogCleanupDialog", () => {
  it("starts with nothing selected and makes no preview or apply request on cancel", async () => {
    await render();
    expect(host.querySelectorAll('input[type="checkbox"]:checked')).toHaveLength(0);
    expect(button("Preview removal").disabled).toBe(true);
    expect(host.textContent).toContain("APP1"); expect(host.textContent).toContain("source-1.knxprod");
    await click("Cancel");
    expect(onClose).toHaveBeenCalledOnce(); expect(api.previewCatalogCleanup).not.toHaveBeenCalled(); expect(api.applyCatalogCleanup).not.toHaveBeenCalled();
  });

  it("suggests only extra exact-reference placements, previews selected ids, then applies exactly the shown token", async () => {
    await render(); await click("Select extra placements");
    expect(Array.from(host.querySelectorAll<HTMLInputElement>('input:checked')).map((i) => i.value)).toEqual(["cat-b"]);
    await click("Preview removal");
    expect(api.previewCatalogCleanup).toHaveBeenCalledExactlyOnceWith(["cat-b"]);
    expect(api.applyCatalogCleanup).not.toHaveBeenCalled();
    expect(host.textContent).toContain("Preview only — nothing has been removed.");
    expect(host.textContent).toContain("cat-b");
    await click("Back up and remove 1 entry");
    expect(api.applyCatalogCleanup).toHaveBeenCalledExactlyOnceWith(["cat-b"], "bound-token");
    expect(onRemoved).toHaveBeenCalledExactlyOnceWith({ removedItemIds: ["cat-b"], backup: "products.sqlite.backup-1-abcdefabcdef.sqlite" });
    expect(host.textContent).toContain("Removed 1 catalogue entry. Backup: products.sqlite.backup-1-abcdefabcdef.sqlite");
    expect(host.textContent).toContain("does not necessarily restore");
  });

  it("can select an older version manually without considering another version redundant", async () => {
    await render(); await select("cat-a"); await select("cat-v2"); await click("Preview removal");
    expect(api.previewCatalogCleanup).toHaveBeenCalledExactlyOnceWith(["cat-a", "cat-v2"]);
  });

  it("warns before removing the last placement of a binding that open-project devices use", async () => {
    api.previewCatalogCleanup.mockResolvedValue({ token: "t", items: [
      { ...items[2], impact: { lastPlacementForBinding: true, openProjectDevices: 2 } },
      { ...items[1], impact: { lastPlacementForBinding: false, openProjectDevices: 1 } }] });
    await render(); await select("cat-v2"); await select("cat-b"); await click("Preview removal");
    const rows = host.querySelectorAll(".catalog-cleanup-list li");
    expect(rows[0].textContent).toContain("Last catalogue entry");
    expect(rows[0].textContent).toContain("2 devices in the open project");
    expect(rows[1].textContent).not.toContain("Last catalogue entry");
    expect(rows[1].textContent).toContain("1 device in the open project");
    expect(host.textContent).toContain("Saved projects that are not open are not checked");
  });

  it("filters on what a row shows, not on its wire format", async () => {
    await render();
    const filter = host.querySelector<HTMLInputElement>('input[aria-label="Filter entries to clean up"]')!;
    const type = async (value: string) => act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(filter, value);
      filter.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await type("null");
    expect(host.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
    await type("source-2");
    expect(Array.from(host.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')).map((i) => i.value)).toEqual(["cat-v2"]);
    await type("16");
    expect(host.querySelectorAll('input[type="checkbox"]')).toHaveLength(2);
  });

  it("keeps a confirmed removal confirmed when the catalogue refresh fails", async () => {
    onRemoved.mockRejectedValue(new Error("refresh failed"));
    await render(); await select("cat-b"); await click("Preview removal"); await click("Back up and remove 1 entry");
    expect(host.textContent).toContain("Removed 1 catalogue entry.");
    expect(host.querySelector('[role="alert"]')).toBeNull();
  });

  it("invalidates a stale preview instead of offering a blind apply retry", async () => {
    api.applyCatalogCleanup.mockRejectedValue({ status: 409, body: { kind: "catalogueCleanupStale" } });
    await render(); await select("cat-b"); await click("Preview removal"); await click("Back up and remove 1 entry");
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("catalogue changed");
    expect(Array.from(host.querySelectorAll("button")).some((b) => b.textContent?.startsWith("Back up and remove"))).toBe(false);
    expect(onRemoved).not.toHaveBeenCalled(); await click("Read catalogue again");
    expect(api.catalogItems).toHaveBeenCalledTimes(2);
    expect(host.querySelectorAll('input:checked')).toHaveLength(0);
  });

  it("keeps an unconfirmed removal distinct from refusal and does not resend", async () => {
    api.applyCatalogCleanup.mockRejectedValue(new Error("connection lost"));
    await render(); await select("cat-b"); await click("Preview removal"); await click("Back up and remove 1 entry");
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("could not be confirmed");
    expect(api.applyCatalogCleanup).toHaveBeenCalledOnce(); expect(onRemoved).not.toHaveBeenCalled();
    expect(Array.from(host.querySelectorAll("button")).some((b) => b.textContent?.startsWith("Back up and remove"))).toBe(false);
  });

  it("prevents dismissal and duplicate requests during apply", async () => {
    let resolve!: (v: { removedItemIds: string[]; backup: string }) => void;
    api.applyCatalogCleanup.mockImplementation(() => new Promise((r) => { resolve = r; }));
    await render(); await select("cat-b"); await click("Preview removal");
    await click("Back up and remove 1 entry");
    await act(async () => host.querySelector('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).not.toHaveBeenCalled(); expect(api.applyCatalogCleanup).toHaveBeenCalledOnce();
    await act(async () => resolve({ removedItemIds: ["cat-b"], backup: "backup.sqlite" }));
    expect(onRemoved).toHaveBeenCalledOnce();
  });
});
